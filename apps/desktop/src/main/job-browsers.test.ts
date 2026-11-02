import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  BrowserWindow: class {},
  session: {},
  app: {},
  WebContentsView: class {},
}));

import type { AnswerBrowserRequest, BrowserRequest, SendSessionNote } from "@armada/protocol";
import type { JobBrowserState } from "../shared/api/job-browser";
import { ringOn, type Ring } from "../shared/ring";
import { NO_SETTINGS } from "../shared/site-permissions";
import {
  JobBrowsers,
  jobsPage,
  type BrowserPage,
  type Host,
  type JobBrowserMake,
  type PageTold,
} from "./job-browsers";

/** A page that records which host holds it and where; the rest of `JobPage` answers nothing. */
let pageNumber = 0;
function fakePage(): BrowserPage & {
  name: string;
  at: Host | null;
  moves: (Host | "away")[];
  bounds: unknown[];
  destroyed: boolean;
} {
  const page = {
    name: `page ${(pageNumber += 1)}`,
    at: null as Host | null,
    moves: [] as (Host | "away")[],
    bounds: [] as unknown[],
    destroyed: false,
    url: () => "http://localhost:5173/cart",
    title: () => "Cart",
    load: async () => ({
      loaded: true as const,
    }),
    find: async () => null,
    focus: async () => null,
    send: () => {},
    selectAll: () => {},
    settle: async () => {},
    snapshot: async () => page.name,
    capture: async () => false,
    evaluate: async () => ({}),
    placeIn: (host: Host, bounds: unknown) => {
      page.at = host;
      page.moves.push(host);
      page.bounds.push(bounds);
    },
    leave: (host: Host) => {
      if (page.at !== host) return;
      page.at = null;
      page.moves.push("away");
    },
    hostOf: () => page.at,
    destroy: () => {
      page.destroyed = true;
      page.at = null;
    },
    hide: () => {},
    go: () => {},
    back: () => false,
    forward: () => false,
    viewport: () => ({
      width: 800,
      height: 600,
    }),
    picture: async () => null,
    ask: async () => null,
    settings: () => ({
      sites: {},
      defaults: NO_SETTINGS.defaults,
    }),
    choose: () => {},
    setDefault: () => {},
  };
  return page;
}

function browsers() {
  const pages: ReturnType<typeof fakePage>[] = [];
  const tellers: PageTold[] = [];
  const windows: {
    host: Host;
    tabId: string;
    closing: () => void;
    closed: boolean;
  }[] = [];
  const make: JobBrowserMake = {
    page: (_sessionId, told) => {
      const page = fakePage();
      pages.push(page);
      tellers.push(told);
      return page;
    },
    window: (_sessionId, tabId, told) => {
      const one = {
        host: {
          window: windows.length,
        },
        tabId,
        closing: told.closing,
        closed: false,
      };
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
  const answered: AnswerBrowserRequest[] = [];
  const held = new JobBrowsers(make, {
    changed: (state) => void states.push(state),
    show: (sessionId) => void shown.push(sessionId),
    capture: () => {},
    answer: async (body) => void answered.push(body),
    note: async () => ({
      ok: true,
    }),
  });
  return {
    held,
    pages,
    tellers,
    windows,
    shown,
    states,
    answered,
  };
}

describe("a page asking for a permission", () => {
  it("waits on the owner's answer, shown in the Job browser's state", async () => {
    const { held, tellers, states } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    const answer = tellers[0]!.ask("local_network", "login.example.com");
    expect(states.at(-1)?.site.pending).toEqual({
      permission: "local_network",
      host: "login.example.com",
    });
    held.answer("01JOB", "t1", "allow");
    expect(await answer).toBe("allow");
    expect(states.at(-1)?.site.pending).toBeNull();
  });

  it("is dismissed when the owner hands the page back, and by a second ask while one waits", async () => {
    const { held, tellers } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    held.drive("01JOB", "t1");
    const first = tellers[0]!.ask("camera", "login.example.com");
    expect(await tellers[0]!.ask("microphone", "login.example.com")).toBe("dismiss");
    held.drive("01JOB", "t1");
    expect(await first).toBe("dismiss");
  });

  it("passes a choice and a default to the page", () => {
    const { held, pages } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    const chosen: unknown[] = [];
    pages[0]!.choose = (...args) => void chosen.push(args);
    pages[0]!.setDefault = (...args) => void chosen.push(args);
    held.choose("01JOB", "t1", "login.example.com", "camera", "block");
    held.setDefault("01JOB", "t1", "location", "allow");
    expect(chosen).toEqual([
      [
        "login.example.com",
        "camera",
        "block",
      ],
      [
        "location",
        "allow",
      ],
    ]);
  });
});

describe("a Browser tab's page moving between its pane and a window", () => {
  it("keeps the one page it has, from its pane into a window and back", () => {
    const { held, pages, windows, states } = browsers();
    const bridge = {
      bridge: true,
    };
    const box = {
      x: 0,
      y: 40,
      width: 800,
      height: 600,
    };
    held.tabs("01JOB", [
      "t1",
    ]);
    held.place(bridge, "01JOB", "t1", box);
    held.toWindow("01JOB", "t1");
    // The pane draws nothing of the page once it is in a window, and the window's pane reports where the page goes.
    held.unplace(bridge, "01JOB", "t1");
    held.place(windows[0]!.host, "01JOB", "t1", box);
    expect(states.at(-1)).toMatchObject({
      tabId: "t1",
      windowed: true,
    });
    // Move to the pane: the window closes, and the pane shows the same page.
    held.show("01JOB", "t1");
    held.place(bridge, "01JOB", "t1", box);
    expect(pages).toHaveLength(1);
    expect(pages[0]!.moves).toEqual([
      bridge,
      "away",
      windows[0]!.host,
      "away",
      bridge,
    ]);
    expect(windows[0]!.closed).toBe(true);
    expect(states.at(-1)?.windowed).toBe(false);
  });

  it("moves only that tab's page, and the other tab's stays in its pane", () => {
    const { held, pages, windows, states } = browsers();
    const bridge = {
      bridge: true,
    };
    const box = {
      x: 0,
      y: 40,
      width: 800,
      height: 600,
    };
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.place(bridge, "01JOB", "t1", box);
    held.place(bridge, "01JOB", "t2", box);
    held.unplace(bridge, "01JOB", "t2");
    held.toWindow("01JOB", "t2");
    held.place(windows[0]!.host, "01JOB", "t2", box);
    expect(windows).toHaveLength(1);
    expect(windows[0]!.tabId).toBe("t2");
    expect(pages[0]!.at).toBe(bridge);
    expect(pages[1]!.at).toBe(windows[0]!.host);
    expect(states.filter((one) => one.windowed).map((one) => one.tabId)).toEqual([
      "t2",
    ]);
  });

  it("is put away, not lost, when the owner closes its window", () => {
    const { held, pages, windows, states } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    held.toWindow("01JOB", "t1");
    held.place(windows[0]!.host, "01JOB", "t1", {
      x: 0,
      y: 40,
      width: 800,
      height: 600,
    });
    windows[0]!.closing();
    expect(pages[0]!.at).toBeNull();
    expect(pages[0]!.destroyed).toBe(false);
    expect(held.list()).toEqual([
      expect.objectContaining({
        sessionId: "01JOB",
        tabId: "t1",
        windowed: false,
      }),
    ]);
    expect(states.at(-1)?.windowed).toBe(false);
  });

  it("hands the page to the owner and back, and says so", () => {
    const { held, states } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    held.drive("01JOB", "t1");
    expect(states.at(-1)?.driver).toBe("owner");
    held.drive("01JOB", "t1");
    expect(states.at(-1)?.driver).toBe("agent");
  });
});

describe("a page for each Browser tab", () => {
  const box = {
    x: 0,
    y: 40,
    width: 800,
    height: 600,
  };

  it("makes two pages for two tabs, each placed in its own pane", () => {
    const { held, pages } = browsers();
    const [left, right] = [
      {
        pane: "left",
      },
      {
        pane: "right",
      },
    ];
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.place(left, "01JOB", "t1", {
      ...box,
      width: 400,
    });
    held.place(right, "01JOB", "t2", {
      ...box,
      x: 400,
      width: 400,
    });
    expect(pages).toHaveLength(2);
    expect([
      pages[0]!.at,
      pages[1]!.at,
    ]).toEqual([
      left,
      right,
    ]);
    expect(pages[0]!.bounds).toEqual([
      {
        ...box,
        width: 400,
      },
    ]);
    expect(pages[1]!.bounds).toEqual([
      {
        ...box,
        x: 400,
        width: 400,
      },
    ]);
    expect(held.list().map((one) => one.tabId)).toEqual([
      "t1",
      "t2",
    ]);
  });

  it("follows a pane resized or zoomed: each rect the pane reports is the page's bounds, in the same host", () => {
    const { held, pages } = browsers();
    const bridge = {
      bridge: true,
    };
    held.tabs("01JOB", [
      "t1",
    ]);
    held.place(bridge, "01JOB", "t1", box);
    held.place(bridge, "01JOB", "t1", {
      ...box,
      width: 400,
    });
    held.place(bridge, "01JOB", "t1", {
      x: 0,
      y: 0,
      width: 1440,
      height: 900,
    });
    expect(pages[0]!.bounds).toEqual([
      box,
      {
        ...box,
        width: 400,
      },
      {
        x: 0,
        y: 0,
        width: 1440,
        height: 900,
      },
    ]);
    expect(pages[0]!.at).toBe(bridge);
  });

  it("tells the renderer of each page as it is made, so its pane has something to draw", () => {
    const { held, states } = browsers();
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    expect(states.map((one) => one.tabId)).toEqual([
      "t1",
      "t2",
    ]);
  });

  it("keeps a page hidden, not destroyed, while another tab is in front, and places the same page again", () => {
    const { held, pages } = browsers();
    const bridge = {
      bridge: true,
    };
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.place(bridge, "01JOB", "t1", box);
    // t2 comes to the front of the pane: t1's body unmounts and puts its page away.
    held.unplace(bridge, "01JOB", "t1");
    held.place(bridge, "01JOB", "t2", box);
    expect(pages[0]!.at).toBeNull();
    expect(pages[0]!.destroyed).toBe(false);
    held.unplace(bridge, "01JOB", "t2");
    held.place(bridge, "01JOB", "t1", box);
    expect(pages).toHaveLength(2);
    expect(pages[0]!.at).toBe(bridge);
    expect(pages[0]!.moves).toEqual([
      bridge,
      "away",
      bridge,
    ]);
  });

  it("destroys the page of a tab that is closed, and only that page", () => {
    const { held, pages } = browsers();
    const bridge = {
      bridge: true,
    };
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.place(bridge, "01JOB", "t1", box);
    held.tabs("01JOB", [
      "t1",
    ]);
    expect(pages.map((one) => one.destroyed)).toEqual([
      false,
      true,
    ]);
    expect(held.list().map((one) => one.tabId)).toEqual([
      "t1",
    ]);
    // A tab closed from a pane it was not in front of is told the same way, and a page that is not there takes no act.
    held.place(bridge, "01JOB", "t2", box);
    expect(pages[1]!.at).toBeNull();
  });

  it("closes the window of a tab that is closed", () => {
    const { held, pages, windows } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    held.toWindow("01JOB", "t1");
    held.tabs("01JOB", []);
    expect(windows[0]!.closed).toBe(true);
    expect(pages[0]!.destroyed).toBe(true);
  });

  it("destroys every page of a Job that is gone, and no other Job's", () => {
    const { held, pages } = browsers();
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    held.tabs("02JOB", [
      "t1",
    ]);
    held.keep(
      new Set([
        "02JOB",
      ]),
    );
    expect(pages.map((one) => one.destroyed)).toEqual([
      true,
      true,
      false,
    ]);
    expect(held.list().map((one) => one.sessionId)).toEqual([
      "02JOB",
    ]);
  });

  it("gives a tab named twice one page", () => {
    const { held, pages } = browsers();
    held.tabs("01JOB", [
      "t1",
    ]);
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    expect(pages).toHaveLength(2);
  });
});

describe("the Job's page", () => {
  it("is the first shown tab in pane order, else the one shown last, else the first", () => {
    const tab = (id: string, shown = false) => ({
      id,
      shown,
    });
    expect(
      jobsPage(
        [
          tab("t1"),
          tab("t2", true),
          tab("t3", true),
        ],
        "t3",
      ),
    ).toBe("t2");
    expect(
      jobsPage(
        [
          tab("t1"),
          tab("t2"),
          tab("t3"),
        ],
        "t3",
      ),
    ).toBe("t3");
    expect(
      jobsPage(
        [
          tab("t1"),
          tab("t2"),
        ],
        "gone",
      ),
    ).toBe("t1");
    expect(jobsPage([], "t1")).toBeUndefined();
  });

  /** An agent's snapshot, answered by whichever page the Job's page is. */
  async function snapshotted(held: JobBrowsers, answered: AnswerBrowserRequest[]): Promise<string> {
    const request: BrowserRequest = {
      session_id: "01JOB",
      request_id: `r${answered.length}`,
      act: {
        kind: "snapshot",
      },
    };
    await held.request(request);
    const answer = answered.at(-1) as {
      answer?: {
        snapshot?: string;
      };
    };
    return answer.answer?.snapshot ?? "";
  }

  it("is the tab the owner looks at, and keeps the last one he looked at when he looks at none", async () => {
    const { held, pages, answered } = browsers();
    const bridge = {
      bridge: true,
    };
    held.tabs("01JOB", [
      "t1",
      "t2",
    ]);
    expect(await snapshotted(held, answered)).toBe(pages[0]!.name);
    held.place(bridge, "01JOB", "t2", {
      x: 0,
      y: 40,
      width: 800,
      height: 600,
    });
    expect(await snapshotted(held, answered)).toBe(pages[1]!.name);
    held.unplace(bridge, "01JOB", "t2");
    expect(await snapshotted(held, answered)).toBe(pages[1]!.name);
  });

  it("is the page an agent opened for a Job with no tab, which the first tab then takes over", async () => {
    const { held, pages, shown, states, answered } = browsers();
    const request: BrowserRequest = {
      session_id: "01JOB",
      request_id: "r1",
      act: {
        kind: "open",
        url: "http://localhost:5173",
      },
    };
    await held.request(request);
    expect(pages).toHaveLength(1);
    expect(shown).toEqual([
      "01JOB",
    ]);
    expect(answered).toHaveLength(1);
    expect(held.list().map((one) => one.tabId)).toEqual([
      "",
    ]);
    held.tabs("01JOB", [
      "t1",
    ]);
    expect(pages).toHaveLength(1);
    expect(held.list().map((one) => one.tabId)).toEqual([
      "t1",
    ]);
    expect(states.at(-1)?.tabId).toBe("t1");
  });
});

describe("a note taken on a Job's browser", () => {
  it("sends the whole frozen page with the part he picked ringed in its pixels, tagged 1", async () => {
    const page = fakePage();
    // A Retina page: its picture twice the viewport's width.
    page.picture = async () => ({
      png: Buffer.from("whole page"),
      ring: (box) => ringOn(box, 800, 1600),
    });
    page.ask = async () => ({
      capture: {
        element: {
          tag: "button",
          text: "Pay",
        },
        selector: "main > button",
        markup: "<button>Pay</button>",
        bounds: {
          x: 100,
          y: 50,
          width: 80,
          height: 20,
        },
        location: "/cart",
      },
    });
    const notes: SendSessionNote[] = [];
    const burned: [
      string,
      Ring,
      number,
    ][] = [];
    const held = new JobBrowsers(
      {
        page: () => page,
        window: () => ({
          host: {},
          focus: () => {},
          close: () => {},
        }),
      },
      {
        changed: () => {},
        show: () => {},
        capture: () => {},
        answer: async () => {},
        note: async (body) => (
          notes.push(body),
          {
            ok: true,
          }
        ),
      },
      (png, ring, number) => (
        burned.push([
          Buffer.from(png).toString(),
          ring,
          number,
        ]),
        Buffer.from("ringed")
      ),
    );
    held.tabs("01JOB", [
      "t1",
    ]);
    await held.freeze("01JOB", "t1");
    await held.hold("01JOB", "t1", 120, 60);
    expect(await held.note("01JOB", "t1", "Too small")).toEqual({
      ok: true,
    });
    expect(burned).toEqual([
      [
        "whole page",
        {
          x: 200,
          y: 100,
          width: 160,
          height: 40,
          scale: 2,
        },
        1,
      ],
    ]);
    expect(notes[0]?.crop).toEqual({
      media_type: "image/png",
      data: Buffer.from("ringed").toString("base64"),
    });
  });
});
