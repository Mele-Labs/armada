import { readdirSync, readFileSync } from "node:fs";
import type { WaitingItem } from "@armada/protocol";
import { describe, expect, it, vi } from "vitest";

import { homeOf, type Mac } from "./fleet";
import { answer } from "./fixture/gateway";
import { everyState, fixtureMacs } from "./fixture/wire";
import { rowStateOf } from "./rowState";
import {
  asksOf,
  choicesOf,
  documentOf,
  draftsOf,
  onMacOf,
  OPEN_ON_MAC,
  pressesOf,
  rowsOf,
  runsOf,
  sessionOfThread,
  takesWords,
} from "./thread";
import { jobStateOf } from "@armada/screens/src/session-state";
import { sketchAnswerOf, documentAnswerOf } from "@armada/screens/src/thread-cards";
import { Refused } from "./client";
import { attemptPage } from "./jobState";
import { fit, followScreen } from "./viewport";
import type { PhoneSession } from "./gateway";
import { codeFromInput, linkOf } from "./pairing";
import { routeOf } from "./router";
import { bytesOf } from "./push";
import { SseParser } from "./sse";
import { bodyHash, hex, signHeaders, signedMessage } from "./signing";

// A fixed P-256 key (test only).
const PRIVATE = {
  kty: "EC",
  crv: "P-256",
  x: "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU",
  y: "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0",
  d: "jpsQnnGQmL-YBIffH1136cspYG6-0iY7X1fCE9-E9LI",
};

const EMPTY = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

describe("the signer", () => {
  it("joins method, target, time and the body hash with nothing between", async () => {
    expect(await bodyHash("")).toBe(EMPTY);
    expect(await signedMessage("GET", "/api/sessions?state=ended", 1760000000, "")).toBe(
      `GET/api/sessions?state=ended1760000000${EMPTY}`,
    );
  });

  it("signs that message with the key, as 64 raw bytes the public key verifies", async () => {
    const key = await crypto.subtle.importKey(
      "jwk",
      PRIVATE,
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      false,
      [
        "sign",
      ],
    );
    const headers = await signHeaders(key, "abc", "POST", "/api/x", '{"a":1}', 1760000000);
    expect(headers["X-Pocket-Device"]).toBe("abc");
    expect(headers["X-Pocket-Time"]).toBe("1760000000");
    expect(headers["X-Pocket-Signature"]).toMatch(/^[0-9a-f]{128}$/);
    const { d: _private, ...open } = PRIVATE;
    const verify = await crypto.subtle.importKey(
      "jwk",
      open,
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      false,
      [
        "verify",
      ],
    );
    const message = new TextEncoder().encode(
      await signedMessage("POST", "/api/x", 1760000000, '{"a":1}'),
    );
    const raw = Uint8Array.from(
      headers["X-Pocket-Signature"].match(/../g)!.map((byte) => parseInt(byte, 16)),
    );
    expect(
      await crypto.subtle.verify(
        {
          name: "ECDSA",
          hash: "SHA-256",
        },
        verify,
        raw,
        message,
      ),
    ).toBe(true);
    expect(hex(raw)).toBe(headers["X-Pocket-Signature"]);
  });
});

describe("the SSE parser", () => {
  it("returns an event once its blank line arrives, across chunk boundaries", () => {
    const parser = new SseParser();
    expect(parser.feed('data: {"change":"ses')).toEqual([]);
    expect(parser.feed('sion"}\n')).toEqual([]);
    expect(parser.feed('\ndata: {"change":"resync"}\n\n')).toEqual([
      '{"change":"session"}',
      '{"change":"resync"}',
    ]);
  });

  it("joins multi-line data, ignores other fields and comments, and reads CRLF", () => {
    const parser = new SseParser();
    expect(parser.feed(": hello\r\nevent: x\r\ndata: a\r\ndata: b\r\n\r\n")).toEqual([
      "a\nb",
    ]);
  });
});

describe("the push key", () => {
  it("decodes base64url, without padding, to the bytes of the point", () => {
    expect(Array.from(bytesOf("BAH_-w"))).toEqual([
      4,
      1,
      255,
      251,
    ]);
  });
});

describe("codeFromInput", () => {
  const code = "0123456789abcdef0123456789abcdef";
  it("reads the code from a whole link", () => {
    expect(codeFromInput(`https://mac.example:7777/pair?code=${code}`)).toBe(code);
    expect(codeFromInput(`  https://mac.example/pair?code=${code}\n`)).toBe(code);
  });
  it("accepts a bare code", () => expect(codeFromInput(code)).toBe(code));
  it("refuses junk", () => {
    expect(codeFromInput("")).toBe("");
    expect(codeFromInput("hello")).toBe("");
    expect(codeFromInput("https://mac.example/pair?code=abc")).toBe("");
  });
});

describe("linkOf", () => {
  const code = "0123456789abcdef0123456789abcdef";
  it("takes the Mac from the link and this page's Mac for a bare code", () => {
    expect(
      linkOf(`https://laptop.tail0.ts.net/pair?code=${code}`, "https://studio.tail0.ts.net"),
    ).toEqual({
      origin: "https://laptop.tail0.ts.net",
      code,
    });
    expect(linkOf(code, "https://studio.tail0.ts.net")).toEqual({
      origin: "https://studio.tail0.ts.net",
      code,
    });
    expect(
      linkOf("https://laptop.tail0.ts.net/pair", "https://studio.tail0.ts.net"),
    ).toBeUndefined();
  });
});

describe("routeOf", () => {
  it("reads a Job's Mac and id, and what it showed", () => {
    const mac = encodeURIComponent("https://studio.tail0.ts.net");
    expect(routeOf(`/jobs/${mac}/s1`)).toEqual({
      screen: "job",
      origin: "https://studio.tail0.ts.net",
      id: "s1",
    });
    expect(routeOf("/jobs/_/s1")).toEqual({
      screen: "job",
      origin: undefined,
      id: "s1",
    });
    expect(routeOf(`/jobs/${mac}/s1/file?path=%2Fa%2Fb.md`)).toEqual({
      screen: "file",
      origin: "https://studio.tail0.ts.net",
      id: "s1",
      path: "/a/b.md",
    });
    expect(routeOf(`/jobs/${mac}/s1/sketch?id=sketch%3AHome`)).toEqual({
      screen: "sketch",
      origin: "https://studio.tail0.ts.net",
      id: "s1",
      sketch: "sketch:Home",
    });
    expect(routeOf("/pair?code=abc")).toEqual({
      screen: "pair",
      code: "abc",
    });
    expect(routeOf(`/drafts/${mac}/d1`)).toEqual({
      screen: "draft",
      origin: "https://studio.tail0.ts.net",
      id: "d1",
    });
    expect(routeOf("/drafts/_/d1")).toEqual({
      screen: "draft",
      origin: "_",
      id: "d1",
    });
    expect(routeOf("/nowhere")).toEqual({
      screen: "home",
    });
  });
});

describe("homeOf", () => {
  it("sorts each state into the section the desktop's bucket says", () => {
    const { live } = everyState(Date.parse("2026-10-08T12:00:00Z"));
    const home = homeOf({
      macs: [
        mac("https://a", {
          live,
        }),
      ],
    });
    const ids = (
      items: {
        session: PhoneSession;
      }[],
    ) => items.map((one) => one.session.id).sort();
    expect(ids(home.drafts)).toEqual([
      "x-blank",
    ]);
    expect(ids(home.needs)).toEqual([
      "x-failing",
      "x-gave-up",
      "x-stuck",
      "x-unstarted",
    ]);
    expect(ids(home.waiting)).toEqual([
      "x-checks",
      "x-job",
      "x-limited",
      "x-queue",
      "x-quit",
      "x-restarting",
      "x-watch",
    ]);
    expect(ids(home.running)).toEqual([
      "x-paused",
      "x-reading",
    ]);
  });

  const session = (id: string, extra: Partial<PhoneSession>): PhoneSession => ({
    id,
    kind: "hosted",
    waiting: false,
    working: false,
    held: false,
    started_at: "2026-10-08T10:00:00Z",
    ...extra,
  });
  const mac = (origin: string, extra: Partial<Mac>): Mac => ({
    machine: {
      origin,
      name: origin.slice(8),
      deviceId: "d",
      key: {} as CryptoKey,
    },
    paired: true,
    needs: [],
    live: [],
    ...extra,
  });
  it("leaves out an unpaired Mac's Jobs and lists it as troubled", () => {
    const home = homeOf({
      macs: [
        mac("https://a", {
          live: [
            session("a1", {
              waiting: true,
              waiting_since: "2026-10-08T11:00:00Z",
            }),
          ],
        }),
        mac("https://b", {
          paired: false,
          live: [
            session("b1", {
              waiting: true,
            }),
          ],
        }),
      ],
    });
    expect(home.needs.map((one) => one.session.id)).toEqual([
      "a1",
    ]);
    expect(home.troubled.map((one) => one.machine.origin)).toEqual([
      "https://b",
    ]);
    expect(home.several).toBe(true);
  });
  it("keeps a waiting Job out of Running", () => {
    const home = homeOf({
      macs: [
        mac("https://a", {
          live: [
            session("a1", {
              working: true,
              waiting: true,
            }),
            session("a2", {
              working: true,
            }),
          ],
        }),
      ],
    });
    expect(home.running.map((one) => one.session.id)).toEqual([
      "a2",
    ]);
    expect(home.several).toBe(false);
  });
  it("lists a held Job under drafts alone, whether or not it waits on its first message", () => {
    const home = homeOf({
      macs: [
        mac("https://a", {
          needs: [
            session("d1", {
              held: true,
              waiting: true,
            }),
          ],
          live: [
            session("d1", {
              held: true,
              waiting: true,
            }),
            session("d2", {
              held: true,
            }),
            session("a1", {
              working: true,
            }),
          ],
        }),
      ],
    });
    expect(home.drafts.map((one) => one.session.id).sort()).toEqual([
      "d1",
      "d2",
    ]);
    expect(home.needs).toEqual([]);
    expect(home.running.map((one) => one.session.id)).toEqual([
      "a1",
    ]);
  });
  it("lists an open Job with no turn running, which is neither waiting nor ended", () => {
    const home = homeOf({
      macs: [
        mac("https://a", {
          live: [
            session("a1", {
              working: true,
            }),
            session("a2", {}),
            session("a3", {
              waiting: true,
            }),
          ],
        }),
      ],
    });
    expect(home.running.map((one) => one.session.id).sort()).toEqual([
      "a1",
      "a2",
    ]);
  });
});

describe("rowStateOf", () => {
  const session = (id: string, extra: Partial<PhoneSession>): PhoneSession => ({
    id,
    kind: "hosted",
    waiting: false,
    working: false,
    started_at: "2026-10-08T10:00:00Z",
    held: false,
    ...extra,
  });

  it("gives a row the state and the words its Job's header gives, in every state", () => {
    const { live, threads } = everyState(Date.parse("2026-10-08T12:00:00Z"));
    expect(live.length).toBeGreaterThan(0);
    for (const listed of live)
      expect(rowStateOf(listed), listed.id).toEqual(
        jobStateOf(sessionOfThread(threads[listed.id]!), () => true),
      );
  });

  it("reads a reply as idle, since the phone keeps no record of what he read", () => {
    expect(
      rowStateOf(
        session("r1", {
          standing: "replied",
        }),
      ).state,
    ).toBe("idle");
  });

  it("reads a Job Fleet has run out of ways to bring back as asking, and one it is still bringing back as waiting", () => {
    const stopped = (gave_up: boolean) =>
      rowStateOf(
        session("s1", {
          stopped: {
            reason: "process_gone",
            restart: "failed",
            gave_up,
          },
        }),
      ).state;
    expect(stopped(true)).toBe("stopped_asks");
    expect(stopped(false)).toBe("stopped");
  });
});

describe("the headers and the thread's width", () => {
  const css = readFileSync(new URL("./pocket.css", import.meta.url), "utf8");
  const depth = readFileSync(
    new URL("../../../packages/components/src/depth.css", import.meta.url),
    "utf8",
  );
  const rule = (selector: string) => {
    const at = css.indexOf(`${selector} {`);
    expect(at).toBeGreaterThan(-1);
    return css.slice(at, css.indexOf("}", at));
  };
  it("clears the status bar past the look's band rule, whose :is() weighs four classes", () => {
    expect(depth).toContain(".pr-desc > .pr-what__head");
    const band = rule(':root[data-look="new"] .pk-app .deck-band');
    expect(band).toMatch(/padding-top:\s*env\(safe-area-inset-top\)/);
    expect(band).toMatch(
      /min-height:\s*calc\(var\(--h-title-bar\) \+ env\(safe-area-inset-top\)\)/,
    );
  });
  it("stops every column of the thread's end at the screen, so a long line cannot pan it", () => {
    for (const one of [
      ".pk-trailing",
      ".pk-ask",
      ".pk-card",
    ])
      expect(rule(one)).toMatch(/grid-template-columns:\s*minmax\(0,\s*1fr\)/);
  });
});

describe("the visible screen", () => {
  const root = () => {
    const props = new Map<string, string>();
    let keyboard = false;
    return {
      props,
      keyboard: () => keyboard,
      element: {
        style: {
          setProperty: (name: string, value: string) => void props.set(name, value),
        },
        toggleAttribute: (_name: string, on: boolean) => {
          keyboard = on;
        },
      } as unknown as HTMLElement,
    };
  };
  it("marks a keyboard when iOS shortens the layout's height with it", () => {
    const one = root();
    const view = {
      width: 402,
      height: 874,
      offsetTop: 0,
      scale: 1,
    };
    const follow = followScreen(view, one.element, () => undefined);
    follow();
    expect(one.keyboard()).toBe(false);
    view.height = 500;
    follow();
    expect(one.keyboard()).toBe(true);
    view.height = 874;
    follow();
    expect(one.keyboard()).toBe(false);
  });
  it("measures the screen again when it turns", () => {
    const one = root();
    const view = {
      width: 402,
      height: 874,
      offsetTop: 0,
      scale: 1,
    };
    const follow = followScreen(view, one.element, () => undefined);
    follow();
    view.width = 874;
    view.height = 402;
    follow();
    expect(one.keyboard()).toBe(false);
  });
  it("undoes a pan, and not a pinch zoom's", () => {
    const one = root();
    const view = {
      width: 402,
      height: 500,
      offsetTop: 0,
      scale: 1,
    };
    let unpanned = 0;
    const follow = followScreen(view, one.element, () => {
      unpanned += 1;
    });
    follow();
    expect(unpanned).toBe(0);
    view.offsetTop = 300;
    follow();
    expect(unpanned).toBe(1);
    view.scale = 2;
    follow();
    expect(unpanned).toBe(1);
  });
  it("takes the visual viewport's height and pan, and marks a keyboard", () => {
    const one = root();
    const view = {
      height: 500,
      offsetTop: 40,
      scale: 1,
    };
    fit(view, 844, one.element);
    expect(one.props.get("--pk-visible-height")).toBe(`${view.height}px`);
    expect(one.props.get("--pk-visible-top")).toBe(`${view.offsetTop}px`);
    expect(one.keyboard()).toBe(true);
  });
  it("marks no keyboard for a toolbar's height", () => {
    const one = root();
    fit(
      {
        height: 790,
        offsetTop: 0,
        scale: 1,
      },
      844,
      one.element,
    );
    expect(one.keyboard()).toBe(false);
  });
  it("leaves the app as it was through a pinch zoom", () => {
    const one = root();
    fit(
      {
        height: 300,
        offsetTop: 90,
        scale: 2,
      },
      844,
      one.element,
    );
    expect(one.props.size).toBe(0);
  });
});

describe("what the owner is asked beside the thread", () => {
  const item = (source: WaitingItem["source"], more: Partial<WaitingItem> = {}): WaitingItem => ({
    id: source,
    text: source,
    since: "2026-11-03T00:00:00Z",
    source,
    ...more,
  });
  const session = (waitingFor: WaitingItem[]) =>
    ({
      waitingFor,
    }) as unknown as Parameters<typeof asksOf>[0];
  it("takes the agent's and Fleet's items with choices, and leaves another Job's to theirs", () => {
    const asked: WaitingItem["source"][] = [
      "agent",
      "delivery",
      "slot_choice",
      "task_done",
      "rule_act",
      "watch_not_ours",
      "words_address",
    ];
    const left: WaitingItem["source"][] = [
      "sketch",
      "document",
      "draft",
      "walk",
      "first_message",
      "fleet_call",
    ];
    expect(
      asksOf(
        session(
          [
            ...asked,
            ...left,
          ].map((one) => item(one)),
        ),
      ).map((one) => one.source),
    ).toEqual(asked);
  });
  it("keeps a command for its Run card", () => {
    expect(
      asksOf(
        session([
          item("agent", {
            act: {
              kind: "run",
              target: "scripts/preview --restart",
            },
          }),
        ]),
      ),
    ).toEqual([]);
  });
  it("offers words only where Fleet takes them", () => {
    expect(
      (
        [
          "delivery",
          "slot_choice",
          "task_done",
          "rule_act",
        ] as const
      ).map((one) => takesWords(item(one))),
    ).toEqual([
      false,
      false,
      false,
      false,
    ]);
    expect(
      (
        [
          "agent",
          "watch_not_ours",
          "words_address",
        ] as const
      ).map((one) => takesWords(item(one))),
    ).toEqual([
      true,
      true,
      true,
    ]);
  });
  it("keeps each option's index on the wire, without an address's Other", () => {
    expect(
      choicesOf(
        item("words_address", {
          options: [
            {
              label: "a",
            },
            {
              label: "b",
            },
            {
              label: "Other",
            },
          ],
        }),
      ).map((one) => [
        one.label,
        one.choice,
      ]),
    ).toEqual([
      [
        "a",
        0,
      ],
      [
        "b",
        1,
      ],
    ]);
  });
  it("asks nothing while the slot it asked for is being made", () => {
    expect(
      choicesOf(
        item("slot_choice", {
          creating_since: "2026-11-03T00:00:00Z",
          options: [
            {
              label: "Wait for a slot",
            },
            {
              label: "Add a slot",
            },
          ],
        }),
      ),
    ).toEqual([]);
  });
});

describe("runsOf", () => {
  const thread = () =>
    fixtureMacs(Date.now(), {
      studio: "https://studio",
      laptop: "https://laptop",
    })["https://studio"]!.threads["r1"]!;
  const runs = (wire: ReturnType<typeof thread>) => runsOf(sessionOfThread(wire), wire);
  it("draws a command item as a card to run, with no ran mark before a run", () => {
    expect(runs(thread())).toMatchObject([
      {
        command: "scripts/preview --restart",
      },
    ]);
    expect(runs(thread())[0]!.ran).toBeUndefined();
  });
  it("marks it ran, with the exit code, once a newer ran row answers the same command", () => {
    const wire = thread();
    const at = new Date().toISOString();
    wire.rows.push({
      kind: "ran",
      id: "r1-ran",
      at,
      command: " scripts/preview --restart ",
      code: 2,
      tail: "",
    });
    expect(runs(wire)[0]!.ran).toEqual({
      code: 2,
    });
  });
  it("does not count a ran row older than the item, nor one for another command", () => {
    const wire = thread();
    wire.rows.push({
      kind: "ran",
      id: "r1-old",
      at: new Date(Date.now() - 3_600_000).toISOString(),
      command: "scripts/preview --restart",
      code: 0,
      tail: "",
    });
    wire.rows.push({
      kind: "ran",
      id: "r1-other",
      at: new Date().toISOString(),
      command: "ls",
      code: 0,
      tail: "",
    });
    expect(runs(wire)[0]!.ran).toBeUndefined();
  });
});

describe("the source folder", () => {
  // Vite tries `.ts` before `.tsx`, so on a Mac `./Pair` finds pair.ts and the build breaks.
  it("has no two files that differ only in case or extension", () => {
    const stems = readdirSync(new URL(".", import.meta.url)).map((name) =>
      name.replace(/\.[^.]+$/, "").toLowerCase(),
    );
    expect(stems.filter((stem, at) => stems.indexOf(stem) !== at)).toEqual([]);
  });
});

describe("the text fields", () => {
  // happy-dom does not compute imported CSS, so the stylesheet is read as text. iOS zooms into a focused field under 16 px.
  const css = readFileSync(new URL("./pocket.css", import.meta.url), "utf8");
  const tokens = readFileSync(
    new URL("../../../packages/tokens/tokens.css", import.meta.url),
    "utf8",
  );
  it("takes --text-touch-field, which is 16 px, on every field the phone types in", () => {
    expect(tokens).toMatch(/--text-touch-field:\s*16px/);
    const rule = /main\.pk-app\s*:is\(([^)]*)\)\s*\{([^}]*)\}/.exec(css);
    expect(rule?.[2]).toMatch(/font-size:\s*var\(--text-touch-field\)/);
    for (const field of [
      "textarea",
      "input",
      "select",
      '[contenteditable="true"]',
    ])
      expect(rule?.[1]).toContain(field);
  });
});

describe("a sketch's mock on the phone", () => {
  const studio = () =>
    fixtureMacs(Date.now(), {
      studio: "https://studio",
      laptop: "https://laptop",
    })["https://studio"]!;
  const HASH = "0123456789abcdef0123456789abcdef";

  it("takes the kept hash from the ledger's sketch, since the call in the thread is the agent's words", () => {
    const thread = studio().threads["a2"]!;
    expect(JSON.stringify(thread.rows)).not.toContain(HASH);
    const card = rowsOf(sessionOfThread(thread)).find((row) => row.kind === "sketch");
    expect(
      card?.kind === "sketch" && "nodes" in card.drawing
        ? card.drawing.nodes.find((node) => node.id === "card")?.kept
        : undefined,
    ).toBe(HASH);
  });

  it("reads the kept styles by hash from its Mac, and is told when the Mac holds none", () => {
    const mac = studio();
    const read = (hash: string) =>
      answer(mac, [], "GET", `/api/sessions/kept_styles?hash=${hash}`, "", true);
    const found = read(HASH);
    expect(found.status).toBe(200);
    expect(JSON.parse(found.body).sheets[0].path).toBe("apps/pocket/src/styles.css");
    expect(read("fedcba9876543210fedcba9876543210").status).toBe(404);
  });
});

describe("what a Job waits on, on the phone", () => {
  const macs = () =>
    fixtureMacs(Date.now(), {
      studio: "https://studio",
      laptop: "https://laptop",
    });
  const wire = () => structuredClone(macs()["https://studio"]!.threads["f1"]!);
  const session = (thread = wire()) => sessionOfThread(thread);
  type Held = NonNullable<ReturnType<typeof wire>["session"]["attachments"]>[number];
  const attachment = (thread: ReturnType<typeof wire>, target: string): Held =>
    thread.session.attachments!.find((one) => one.target === target)!;
  /** How Fleet writes an answer to a page into its record: `settled` with the time. */
  const settle = (thread: ReturnType<typeof wire>, target: string) => {
    const detail = attachment(thread, target).detail!;
    detail["settled"] = "approved";
    detail["settled_at"] = new Date().toISOString();
  };
  const documents = (thread = wire()) =>
    rowsOf(session(thread)).flatMap((row) =>
      row.kind === "document"
        ? [
            row,
          ]
        : [],
    );
  const sketches = (thread = wire()) =>
    rowsOf(session(thread)).flatMap((row) =>
      row.kind === "sketch"
        ? [
            row,
          ]
        : [],
    );
  const waiting = (source: WaitingItem["source"]) =>
    session().waitingFor!.find((one) => one.source === source)!;

  describe("a document in the thread", () => {
    it("is a card for each page at its newest revision, waiting on him", () => {
      expect(documents()).toMatchObject([
        {
          title: "Slot pool",
          revision: 1,
          asked: true,
        },
        {
          title: "Phone reach",
          revision: 2,
          asked: true,
        },
      ]);
    });

    it("sits before the first row later than the time its revision was kept", () => {
      const ids = rowsOf(session()).map((row) => (row.kind === "document" ? row.title : row.id));
      expect(ids.indexOf("f1-3")).toBeLessThan(ids.indexOf("Slot pool"));
      expect(ids.indexOf("Slot pool")).toBeLessThan(ids.indexOf("f1-4"));
      expect(ids.indexOf("f1-4")).toBeLessThan(ids.indexOf("Phone reach"));
      expect(ids.indexOf("Phone reach")).toBeLessThan(ids.indexOf("f1-5"));
    });

    it("carries how he answered it", () => {
      const thread = wire();
      settle(thread, "/Users/user/armada/.armada/documents/slot-pool.html");
      const cards = documents(thread);
      expect(cards).toHaveLength(2);
      expect(cards[0]).toMatchObject({
        title: "Slot pool",
        settled: {
          how: "approved",
        },
      });
      expect(cards[1]!.settled).toBeUndefined();
    });

    it("answers with the revision the ledger holds, and an empty one for a page it never gave", () => {
      const held = session();
      const page = "/Users/user/armada/.armada/documents/phone-reach.html";
      expect(
        documentAnswerOf(held, page, {
          choice: 0,
        }),
      ).toEqual({
        session_id: "f1",
        document: `document:${page}`,
        revision: "2",
        how: "approved",
      });
      expect(
        documentAnswerOf(held, page, {
          text: "Cut the table.",
        }),
      ).toMatchObject({
        revision: "2",
        how: "sent_back",
        words: "Cut the table.",
      });
      expect(
        documentAnswerOf(held, "/nowhere.html", {
          choice: 0,
        }).revision,
      ).toBe("");
    });

    it("reads the newest revision to draw, and waits only while it is asked and not settled", () => {
      const page = "/Users/user/armada/.armada/documents/phone-reach.html";
      expect(documentOf(session(), page)).toMatchObject({
        title: "Phone reach",
        revision: 2,
        waits: true,
      });
      expect(documentOf(session(), page)!.revisions.at(-1)).toContain("phone-reach-r2");
      const thread = wire();
      settle(thread, page);
      expect(documentOf(session(thread), page)).toMatchObject({
        waits: false,
        settled: {
          how: "approved",
        },
      });
      expect(documentOf(session(), "/nowhere.html")).toBeUndefined();
    });
  });

  describe("a sketch's card", () => {
    it("holds that he was asked on the latest drawing only", () => {
      const thread = wire();
      thread.rows.push({
        kind: "tool",
        id: "f1-redraw",
        at: new Date().toISOString(),
        text: `show_sketch ${JSON.stringify({
          title: "Slot layout",
          scene: {
            nodes: [
              {
                id: "n0",
                kind: "box",
                x: 0,
                y: 0,
                body: "Redrawn",
              },
            ],
            edges: [],
          },
        })}`,
      });
      const layout = sketches(thread).filter((row) => row.title === "Slot layout");
      expect(layout).toHaveLength(2);
      expect(layout[0]!.replaced).toBe(true);
      expect(layout[0]!.asked).toBeUndefined();
      expect(layout[1]!.replaced).toBeUndefined();
      expect(layout[1]!.asked).toBe(true);
    });

    it("carries how he answered it", () => {
      const thread = wire();
      settle(thread, "sketch:Phone home");
      expect(sketches(thread).find((row) => row.title === "Phone home")).toMatchObject({
        asked: true,
        settled: {
          how: "approved",
        },
      });
    });

    it("answers with the drawing the ledger holds, and an empty one for a sketch it never gave", () => {
      expect(
        sketchAnswerOf(session(), "sketch:Phone home", {
          choice: 0,
        }),
      ).toEqual({
        session_id: "f1",
        sketch: "sketch:Phone home",
        drawn: "phone-home-1",
        how: "approved",
      });
      expect(
        sketchAnswerOf(session(), "sketch:Phone home", {
          text: "Wider.",
        }),
      ).toMatchObject({
        drawn: "phone-home-1",
        how: "sent_back",
        words: "Wider.",
      });
      expect(
        sketchAnswerOf(session(), "No such sketch", {
          choice: 0,
        }).drawn,
      ).toBe("");
    });
  });

  describe("an answer to a page", () => {
    const machine = {
      origin: "https://studio",
      name: "studio",
    } as unknown as Parameters<typeof attemptPage>[1];
    const refused = (status: number, body: string) => async () => {
      throw new Refused(status, body);
    };
    const fleet = (status: number, code: string, message = "Fleet's own words") =>
      refused(
        status,
        JSON.stringify({
          code,
          message,
        }),
      );

    it("is null once Fleet takes it", async () => {
      const reread = vi.fn();
      expect(await attemptPage(async () => undefined, machine, reread)).toBeNull();
      expect(reread).not.toHaveBeenCalled();
    });

    it.each([
      [
        "fleet.sketch_redrawn",
        "The Job drew this again. The new drawing is shown.",
      ],
      [
        "fleet.document_revised",
        "The Job revised this page. The new revision is shown.",
      ],
    ])("reads the Job again and says it changed on a 409 %s", async (code, says) => {
      const reread = vi.fn();
      expect(await attemptPage(fleet(409, code), machine, reread)).toBe(says);
      expect(reread).toHaveBeenCalledTimes(1);
    });

    it.each([
      [
        422,
        "fleet.sketch_answer_empty",
      ],
      [
        409,
        "fleet.sketch_not_open",
      ],
      [
        422,
        "fleet.document_not_open",
      ],
    ])("shows Fleet's message, and reads nothing again, on a %s %s", async (status, code) => {
      const reread = vi.fn();
      expect(await attemptPage(fleet(status, code), machine, reread)).toBe("Fleet's own words");
      expect(reread).not.toHaveBeenCalled();
    });

    it("keeps the Gateway's own 409, which has no code, a plain refusal", async () => {
      const reread = vi.fn();
      expect(
        await attemptPage(refused(409, "This Job is not on its start card."), machine, reread),
      ).toBe("This Job is not on its start card.");
      expect(reread).not.toHaveBeenCalled();
    });

    it("says the Mac did not answer when nothing was refused", async () => {
      expect(
        await attemptPage(
          async () => {
            throw new TypeError("Failed to fetch");
          },
          machine,
          vi.fn(),
        ),
      ).toContain("studio");
    });
  });

  describe("draftsOf", () => {
    it("draws a workflow change where the item is held, and marks it proposed since it has two options", () => {
      expect(draftsOf(session())).toMatchObject([
        {
          id: "f1-10",
          title: "Check before every walk",
          proposed: true,
        },
      ]);
    });

    it("draws none off the row alone, once the item is gone", () => {
      const held = session();
      expect(
        draftsOf({
          ...held,
          waitingFor: held.waitingFor!.filter((one) => one.id !== "draft:f1-10"),
        }),
      ).toEqual([]);
      expect(held.rows.some((row) => row.kind === "draft" && row.id === "f1-10")).toBe(true);
    });

    it("does not mark one proposed whose item has a single option", () => {
      const held = session();
      const [open] = draftsOf({
        ...held,
        waitingFor: held.waitingFor!.map((one) =>
          one.id === "draft:f1-10"
            ? {
                ...one,
                options: [
                  {
                    label: "Approve",
                  },
                ],
              }
            : one,
        ),
      });
      expect(open).toBeDefined();
      expect(open!.proposed).toBeUndefined();
    });

    it("leaves an ordinary draft unmarked, so it has no Dismiss", () => {
      const b3 = sessionOfThread(macs()["https://laptop"]!.threads["b3"]!);
      const open = draftsOf(b3);
      expect(open).toHaveLength(1);
      expect(open[0]!.proposed).toBeUndefined();
    });

    it("draws none for a dead Job", () => {
      expect(
        draftsOf({
          ...session(),
          dead: {},
        } as unknown as Parameters<typeof draftsOf>[0]),
      ).toEqual([]);
    });
  });

  describe("pressesOf", () => {
    it("opens the Job a held child's act names, and offers no dismiss", () => {
      const press = pressesOf(session()).find((one) => one.item.source === "held_child")!;
      expect(press.opens).toBe("d1");
      expect(press.dismissible).toBe(false);
    });

    it("dismisses a Fleet call", () => {
      const press = pressesOf(session()).find((one) => one.item.source === "fleet_call")!;
      expect(press.dismissible).toBe(true);
    });

    it("takes nothing else, and nothing from a dead Job", () => {
      expect(pressesOf(session()).map((one) => one.item.source)).toEqual([
        "held_child",
        "fleet_call",
      ]);
      expect(
        pressesOf({
          ...session(),
          dead: {},
        } as unknown as Parameters<typeof pressesOf>[0]),
      ).toEqual([]);
    });
  });

  describe("onMacOf", () => {
    it("takes each open walk and each question in one, and says to open them on the Mac", () => {
      expect(onMacOf(session()).map((one) => one.source)).toEqual([
        "walk",
        "window_ask",
      ]);
      expect(OPEN_ON_MAC).toBe("Open on your Mac");
    });

    it("takes none from a dead Job", () => {
      expect(
        onMacOf({
          ...session(),
          dead: {},
        } as unknown as Parameters<typeof onMacOf>[0]),
      ).toEqual([]);
    });

    it("keeps the question in a walk out of what is answered beside the thread", () => {
      expect(asksOf(session()).map((one) => one.source)).not.toContain("window_ask");
    });
  });

  describe("a browser's question", () => {
    it("is answered beside the thread by its choices alone", () => {
      const ask = waiting("browser_ask");
      expect(asksOf(session()).map((one) => one.source)).toContain("browser_ask");
      expect(takesWords(ask)).toBe(false);
      expect(
        choicesOf(ask).map((one) => [
          one.label,
          one.choice,
        ]),
      ).toEqual([
        [
          "Allow",
          0,
        ],
        [
          "Block",
          1,
        ],
      ]);
    });
  });

  describe("routeOf", () => {
    it("reads a document's page under a Job", () => {
      expect(
        routeOf(
          `/jobs/${encodeURIComponent("https://studio.tail0.ts.net")}/f1/document?page=${encodeURIComponent("/a/b.html")}`,
        ),
      ).toEqual({
        screen: "document",
        origin: "https://studio.tail0.ts.net",
        id: "f1",
        page: "/a/b.html",
      });
    });
  });
});
