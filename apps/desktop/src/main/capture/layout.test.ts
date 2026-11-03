// The claim, written before the panel: a walk window a Session opened shows that Session beside the
// page, and a Studio's shows nothing beside it. The owner, 11 Oct
// 2026 — `docs/practices/capture-window.md`, *The panel beside the page*.

import { describe, expect, it } from "vitest";

import { barRows, PANE_BAR_ROWS, panelFor, panelQuery, walkLayout } from "./layout";

describe("which panel a walk window opens", () => {
  it("a Session's window holds that Session", () => {
    expect(
      panelFor({
        session: {
          id: "s1",
          title: "Checkout copy",
        },
      }),
    ).toEqual({
      kind: "session",
      id: "s1",
    });
  });

  it("a Studio's window has no panel", () => {
    expect(
      panelFor({
        studio: {
          id: "st",
          name: null,
        },
      }),
    ).toBeNull();
  });

  it("the panel's document is told its subject by a query, and nothing else", () => {
    expect(
      panelQuery({
        kind: "session",
        id: "s1",
      }),
    ).toEqual({
      session: "s1",
    });
  });
});

describe("where each view sits", () => {
  const frame = {
    width: 1280,
    height: 900,
    strip: 44,
    covered: false,
    panel: 480,
  };

  it("the panel takes the right edge, whole height, and the bar and page share what is left", () => {
    const at = walkLayout(frame, 380);
    expect(at.panel).toEqual({
      x: 800,
      y: 0,
      width: 480,
      height: 900,
    });
    expect(at.bar).toEqual({
      x: 0,
      y: 0,
      width: 800,
      height: 44,
    });
    expect(at.page).toEqual({
      x: 0,
      y: 44,
      width: 800,
      height: 856,
    });
  });

  it("covering the page for capture or a question never covers the panel", () => {
    const at = walkLayout(
      {
        ...frame,
        covered: true,
      },
      380,
    );
    expect(at.bar).toEqual({
      x: 0,
      y: 0,
      width: 800,
      height: 900,
    });
    expect(at.panel.x).toBe(800);
  });

  it("an open menu of the bar's brings the bar down over the column as far as it draws, never the panel or past the frame", () => {
    const at = walkLayout(
      {
        ...frame,
        hang: 320.4,
      },
      380,
    );
    expect(at.bar).toEqual({
      x: 0,
      y: 0,
      width: 800,
      height: 321,
    });
    expect(at.page).toEqual({
      x: 0,
      y: 44,
      width: 800,
      height: 856,
    });
    expect(
      walkLayout(
        {
          ...frame,
          hang: 2000,
        },
        380,
      ).bar.height,
    ).toBe(900);
    expect(
      walkLayout(
        {
          ...frame,
          hang: 20,
        },
        380,
      ).bar.height,
    ).toBe(44);
  });

  it("closed, or with no panel, the page has the whole width", () => {
    const at = walkLayout(
      {
        ...frame,
        panel: 0,
      },
      380,
    );
    expect(at.panel).toEqual({
      x: 1280,
      y: 0,
      width: 0,
      height: 0,
    });
    expect(at.page.width).toBe(1280);
    expect(at.bar.width).toBe(1280);
  });

  it("a narrow window gives way from the panel, so the page keeps its floor", () => {
    const at = walkLayout(
      {
        ...frame,
        width: 700,
      },
      380,
    );
    expect(at.page.width).toBe(380);
    expect(at.panel).toEqual({
      x: 380,
      y: 0,
      width: 320,
      height: 900,
    });
  });

  it("a window narrower than the floor draws no panel at all", () => {
    expect(
      walkLayout(
        {
          ...frame,
          width: 300,
        },
        380,
      ).panel.width,
    ).toBe(0);
  });
});

describe("the strip the bar draws", () => {
  it("a queued bar with takes and a walk is two rows, and the page starts directly under it", () => {
    const rows = barRows({
      queued: true,
      takes: true,
      walk: true,
      said: false,
    });
    expect(rows).toBe(2);
    const at = walkLayout(
      {
        width: 1280,
        height: 900,
        strip: 44 * rows,
        covered: false,
        panel: 0,
      },
      380,
    );
    expect(at.page.y).toBe(at.bar.height);
  });

  it("a queued bar with neither a walk nor a status is one row; a status shares the walk's row", () => {
    expect(
      barRows({
        queued: true,
        takes: true,
        walk: false,
        said: false,
      }),
    ).toBe(1);
    expect(
      barRows({
        queued: true,
        takes: true,
        walk: true,
        said: true,
      }),
    ).toBe(2);
  });

  it("a bar that is not queued gives the takes and the walk a row each", () => {
    expect(
      barRows({
        queued: false,
        takes: true,
        walk: true,
        said: true,
      }),
    ).toBe(4);
  });

  it("a queued bar holding notes with no walk is two rows, so the pending notes status is not cut", () => {
    expect(
      barRows({
        queued: true,
        takes: false,
        walk: false,
        said: false,
        held: true,
      }),
    ).toBe(2);
    // The bar that is not queued draws no pending notes status, so they take no row there.
    expect(
      barRows({
        queued: false,
        takes: false,
        walk: false,
        said: false,
        held: true,
      }),
    ).toBe(1);
  });

  it("a page loading takes the status row on either bar, so Loading is not cut", () => {
    expect(
      barRows({
        queued: true,
        takes: false,
        walk: false,
        said: false,
        loading: true,
      }),
    ).toBe(2);
    expect(
      barRows({
        queued: false,
        takes: false,
        walk: false,
        said: false,
        loading: true,
      }),
    ).toBe(2);
    // One status row, whatever else it has to say.
    expect(
      barRows({
        queued: false,
        takes: false,
        walk: false,
        said: true,
        loading: true,
      }),
    ).toBe(2);
  });
});

// A Job's walk is laid inside the frame its pane reports in Bridge's own window, not at the window's corner.
describe("a walk laid inside a pane of Bridge's window", () => {
  const pane = {
    x: 300,
    y: 120,
    width: 900,
    height: 600,
    strip: 64,
    covered: false,
    panel: 0,
  };

  it("offsets the page and the bar by the frame's corner, and keeps their sizes", () => {
    const at = walkLayout(pane, 380);
    expect(at.page).toEqual({
      x: 300,
      y: 184,
      width: 900,
      height: 536,
    });
    expect(at.bar).toEqual({
      x: 300,
      y: 120,
      width: 900,
      height: 64,
    });
  });

  it("covers the frame and no more when the bar covers the page, and reaches as far down as a menu hangs", () => {
    expect(
      walkLayout(
        {
          ...pane,
          covered: true,
        },
        380,
      ).bar,
    ).toEqual({
      x: 300,
      y: 120,
      width: 900,
      height: 600,
    });
    expect(
      walkLayout(
        {
          ...pane,
          hang: 150.2,
        },
        380,
      ).bar.height,
    ).toBe(151);
  });

  it("is the window's own corner where the frame names none", () => {
    const { x: _x, y: _y, ...window } = pane;
    expect(walkLayout(window, 380).page).toMatchObject({
      x: 0,
      y: 64,
    });
  });

  it("draws two rows on a pane's bar whatever the walk holds", () => {
    expect(PANE_BAR_ROWS).toBe(2);
  });
});
