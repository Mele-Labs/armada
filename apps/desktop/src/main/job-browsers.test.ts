import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ BrowserWindow: class {}, session: {}, app: {}, WebContentsView: class {} }));

import type { SendSessionNote } from "@armada/protocol";
import type { JobBrowserState } from "../shared/api/job-browser";
import { ringOn, type Ring } from "../shared/ring";
import { NO_SETTINGS } from "../shared/site-permissions";
import { JobBrowsers, type BrowserPage, type Host, type JobBrowserMake, type PageTold } from "./job-browsers";

/** A page that records which host holds it; the rest of `JobPage` answers nothing. */
function fakePage(): BrowserPage & { at: Host | null; moves: (Host | "away")[] } {
  const page = {
    at: null as Host | null,
    moves: [] as (Host | "away")[],
    url: () => "http://localhost:5173/cart",
    title: () => "Cart",
    load: async () => ({ loaded: true as const }),
    find: async () => null,
    focus: async () => null,
    send: () => {},
    selectAll: () => {},
    settle: async () => {},
    snapshot: async () => "",
    capture: async () => false,
    evaluate: async () => ({}),
    placeIn: (host: Host) => {
      page.at = host;
      page.moves.push(host);
    },
    leave: (host: Host) => {
      if (page.at !== host) return;
      page.at = null;
      page.moves.push("away");
    },
    hostOf: () => page.at,
    hide: () => {},
    go: () => {},
    back: () => false,
    forward: () => false,
    viewport: () => ({ width: 800, height: 600 }),
    picture: async () => null,
    ask: async () => null,
    settings: () => ({ sites: {}, defaults: NO_SETTINGS.defaults }),
    choose: () => {},
    setDefault: () => {},
  };
  return page;
}

function browsers() {
  const pages: ReturnType<typeof fakePage>[] = [];
  const tellers: PageTold[] = [];
  const windows: { host: Host; closing: () => void; closed: boolean }[] = [];
  const make: JobBrowserMake = {
    page: (_sessionId, told) => {
      const page = fakePage();
      pages.push(page);
      tellers.push(told);
      return page;
    },
    window: (_sessionId, told) => {
      const one = { host: { window: windows.length }, closing: told.closing, closed: false };
      windows.push(one);
      return {
        host: one.host,
        focus: () => {},
        close: () => {
          one.closed = true;
          told.closing();
        },
      };
    },
  };
  const shown: string[] = [];
  const states: JobBrowserState[] = [];
  const held = new JobBrowsers(make, {
    changed: (state) => void states.push(state),
    show: (sessionId) => void shown.push(sessionId),
    capture: () => {},
    answer: async () => {},
    note: async () => ({ ok: true }),
  });
  return { held, pages, tellers, windows, shown, states };
}

describe("a page asking for a permission", () => {
  it("waits on the owner's answer, shown in the Job browser's state", async () => {
    const { held, tellers, states } = browsers();
    held.show("01JOB");
    const answer = tellers[0]!.ask("local_network", "login.example.com");
    expect(states.at(-1)?.site.pending).toEqual({ permission: "local_network", host: "login.example.com" });
    held.answer("01JOB", "allow");
    expect(await answer).toBe("allow");
    expect(states.at(-1)?.site.pending).toBeNull();
  });

  it("is dismissed when the owner hands the page back, and by a second ask while one waits", async () => {
    const { held, tellers } = browsers();
    held.show("01JOB");
    held.drive("01JOB");
    const first = tellers[0]!.ask("camera", "login.example.com");
    expect(await tellers[0]!.ask("microphone", "login.example.com")).toBe("dismiss");
    held.drive("01JOB");
    expect(await first).toBe("dismiss");
  });

  it("passes a choice and a default to the page", () => {
    const { held, pages } = browsers();
    held.show("01JOB");
    const chosen: unknown[] = [];
    pages[0]!.choose = (...args) => void chosen.push(args);
    pages[0]!.setDefault = (...args) => void chosen.push(args);
    held.choose("01JOB", "login.example.com", "camera", "block");
    held.setDefault("01JOB", "location", "allow");
    expect(chosen).toEqual([["login.example.com", "camera", "block"], ["location", "allow"]]);
  });
});

describe("a Job's browser moving between the side panel and its window", () => {
  it("keeps the one page it has, from the side panel into its window and back", () => {
    const { held, pages, windows, shown, states } = browsers();
    const bridge = { bridge: true };
    const box = { x: 0, y: 40, width: 800, height: 600 };
    held.show("01JOB");
    held.place(bridge, "01JOB", box);
    held.toWindow("01JOB");
    // The side panel closes as the window opens, and the window's pane reports where the page goes.
    held.unplace(bridge, "01JOB");
    held.place(windows[0]!.host, "01JOB", box);
    expect(states.at(-1)?.windowed).toBe(true);
    // Move to the side panel: the window closes, and Bridge's side panel shows the same page.
    held.show("01JOB");
    held.place(bridge, "01JOB", box);
    expect(pages).toHaveLength(1);
    expect(pages[0]!.moves).toEqual([bridge, "away", windows[0]!.host, "away", bridge]);
    expect(windows[0]!.closed).toBe(true);
    expect(shown).toEqual(["01JOB", "01JOB"]);
    expect(states.at(-1)?.windowed).toBe(false);
  });

  it("is put away, not lost, when the owner closes its window", () => {
    const { held, pages, windows, states } = browsers();
    held.show("01JOB");
    held.toWindow("01JOB");
    held.place(windows[0]!.host, "01JOB", { x: 0, y: 40, width: 800, height: 600 });
    windows[0]!.closing();
    expect(pages[0]!.at).toBeNull();
    expect(held.list()).toEqual([expect.objectContaining({ sessionId: "01JOB", windowed: false })]);
    expect(states.at(-1)?.windowed).toBe(false);
  });

  it("hands the page to the owner and back, and says so", () => {
    const { held, states } = browsers();
    held.show("01JOB");
    held.drive("01JOB");
    expect(states.at(-1)?.driver).toBe("owner");
    held.drive("01JOB");
    expect(states.at(-1)?.driver).toBe("agent");
  });
});

describe("a note taken on a Job's browser", () => {
  it("sends the whole frozen page with the part he picked ringed in its pixels, tagged 1", async () => {
    const page = fakePage();
    // A Retina page: its picture twice the viewport's width.
    page.picture = async () => ({ png: Buffer.from("whole page"), ring: (box) => ringOn(box, 800, 1600) });
    page.ask = async () => ({ capture: { element: { tag: "button", text: "Pay" }, selector: "main > button", markup: "<button>Pay</button>", bounds: { x: 100, y: 50, width: 80, height: 20 }, location: "/cart" } });
    const notes: SendSessionNote[] = [];
    const burned: [string, Ring, number][] = [];
    const held = new JobBrowsers(
      { page: () => page, window: () => ({ host: {}, focus: () => {}, close: () => {} }) },
      { changed: () => {}, show: () => {}, capture: () => {}, answer: async () => {}, note: async (body) => (notes.push(body), { ok: true }) },
      (png, ring, number) => (burned.push([Buffer.from(png).toString(), ring, number]), Buffer.from("ringed")),
    );
    held.show("01JOB");
    await held.freeze("01JOB");
    await held.hold("01JOB", 120, 60);
    expect(await held.note("01JOB", "Too small")).toEqual({ ok: true });
    expect(burned).toEqual([["whole page", { x: 200, y: 100, width: 160, height: 40, scale: 2 }, 1]]);
    expect(notes[0]?.crop).toEqual({ media_type: "image/png", data: Buffer.from("ringed").toString("base64") });
  });
});
