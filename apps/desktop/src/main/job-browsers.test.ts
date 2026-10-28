import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ BrowserWindow: class {}, session: {}, app: {}, WebContentsView: class {} }));

import type { JobBrowserState } from "../shared/api/job-browser";
import { JobBrowsers, type BrowserPage, type Host, type JobBrowserMake } from "./job-browsers";

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
  };
  return page;
}

function browsers() {
  const pages: ReturnType<typeof fakePage>[] = [];
  const windows: { host: Host; closing: () => void; closed: boolean }[] = [];
  const make: JobBrowserMake = {
    page: () => {
      const page = fakePage();
      pages.push(page);
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
  return { held, pages, windows, shown, states };
}

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
