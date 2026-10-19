// The claim, written before the panel: a walk window a Session opened shows that Session beside the
// page, and a Studio's shows nothing beside it. The owner, 11 Oct
// 2026 — `docs/practices/capture-window.md`, *The panel beside the page*.

import { describe, expect, it } from "vitest";

import { panelFor, panelQuery, walkLayout } from "./layout";

describe("which panel a walk window opens", () => {
  it("a Session's window holds that Session", () => {
    expect(panelFor({ session: { id: "s1", title: "Checkout copy" } })).toEqual({ kind: "session", id: "s1" });
  });

  it("a Studio's window has no panel", () => {
    expect(panelFor({ studio: { id: "st", name: null } })).toBeNull();
  });

  it("the panel's document is told its subject by a query, and nothing else", () => {
    expect(panelQuery({ kind: "session", id: "s1" })).toEqual({ session: "s1" });
  });
});

describe("where each view sits", () => {
  const frame = { width: 1280, height: 900, strip: 44, covered: false, panel: 480 };

  it("the panel takes the right edge, whole height, and the bar and page share what is left", () => {
    const at = walkLayout(frame, 380);
    expect(at.panel).toEqual({ x: 800, y: 0, width: 480, height: 900 });
    expect(at.bar).toEqual({ x: 0, y: 0, width: 800, height: 44 });
    expect(at.page).toEqual({ x: 0, y: 44, width: 800, height: 856 });
  });

  it("covering the page for capture or a question never covers the panel", () => {
    const at = walkLayout({ ...frame, covered: true }, 380);
    expect(at.bar).toEqual({ x: 0, y: 0, width: 800, height: 900 });
    expect(at.panel.x).toBe(800);
  });

  it("an open menu of the bar's brings the bar down over the column as far as it draws, never the panel or past the frame", () => {
    const at = walkLayout({ ...frame, hang: 320.4 }, 380);
    expect(at.bar).toEqual({ x: 0, y: 0, width: 800, height: 321 });
    expect(at.page).toEqual({ x: 0, y: 44, width: 800, height: 856 });
    expect(walkLayout({ ...frame, hang: 2000 }, 380).bar.height).toBe(900);
    expect(walkLayout({ ...frame, hang: 20 }, 380).bar.height).toBe(44);
  });

  it("closed, or with no panel, the page has the whole width", () => {
    const at = walkLayout({ ...frame, panel: 0 }, 380);
    expect(at.panel).toEqual({ x: 1280, y: 0, width: 0, height: 0 });
    expect(at.page.width).toBe(1280);
    expect(at.bar.width).toBe(1280);
  });

  it("a narrow window gives way from the panel, so the page keeps its floor", () => {
    const at = walkLayout({ ...frame, width: 700 }, 380);
    expect(at.page.width).toBe(380);
    expect(at.panel).toEqual({ x: 380, y: 0, width: 320, height: 900 });
  });

  it("a window narrower than the floor draws no panel at all", () => {
    expect(walkLayout({ ...frame, width: 300 }, 380).panel.width).toBe(0);
  });
});
