import { readdirSync, readFileSync } from "node:fs";
import type { WaitingItem } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { homeOf, type Mac } from "./fleet";
import { answer } from "./fixture/gateway";
import { fixtureMacs } from "./fixture/wire";
import { asksOf, choicesOf, rowsOf, runsOf, sessionOfThread, takesWords } from "./thread";
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
          needs: [
            session("a1", {
              waiting: true,
              waiting_since: "2026-10-08T11:00:00Z",
            }),
          ],
        }),
        mac("https://b", {
          paired: false,
          needs: [
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
    expect(band).toMatch(/min-height:\s*calc\(var\(--h-title-bar\) \+ env\(safe-area-inset-top\)\)/);
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
    fit(
      {
        height: 500,
        offsetTop: 40,
        scale: 1,
      },
      844,
      one.element,
    );
    expect(one.props.get("--pk-visible-height")).toBe("500px");
    expect(one.props.get("--pk-visible-top")).toBe("40px");
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
    expect(asksOf(session([...asked, ...left].map((one) => item(one)))).map((one) => one.source)).toEqual(
      asked,
    );
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
      (["delivery", "slot_choice", "task_done", "rule_act"] as const).map((one) =>
        takesWords(item(one)),
      ),
    ).toEqual([false, false, false, false]);
    expect(
      (["agent", "watch_not_ours", "words_address"] as const).map((one) => takesWords(item(one))),
    ).toEqual([true, true, true]);
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
      ).map((one) => [one.label, one.choice]),
    ).toEqual([
      ["a", 0],
      ["b", 1],
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
