// The gaps between a column's panels are handles: dragged or stepped, the two panels either side trade height; the sizes are kept for
// each layout; Home, or a double press on the gap, puts the two back; a foot of keys alone is never sized. Browser: heights are read off the page.

import { act, useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { useSplits } from "./splits";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  localStorage.clear();
});

type Foot = "keys" | "field" | undefined;

function Grid({ layout, bandOnly = false, foot }: { layout: string; bandOnly?: boolean; foot?: Foot }) {
  const grid = useRef<HTMLDivElement>(null);
  const splits = useSplits(grid, layout);
  return (
    <div ref={grid} className="deck-grid" style={{ position: "relative", display: "grid", gridTemplateColumns: "1fr", width: 300, height: 400 }}>
      <div className="deck-left" style={{ display: "flex", flexDirection: "column", gap: 8, height: 400 }}>
        <section className="sv-panel" data-area="alpha" aria-label="Alpha" style={{ height: 196 }} />
        <section className="sv-panel" data-area="beta" aria-label="Beta" style={{ height: 196 }} />
        {bandOnly ? <section className="sv-panel" data-area="band" data-band-only="" aria-label="Band" style={{ height: 34 }} /> : null}
        {foot === undefined ? null : (
          <div className="deck-foot" style={{ height: 74 }}>
            {foot === "field" ? <section className="sv-panel" data-area="field" aria-label="Field" style={{ height: 74 }} /> : null}
          </div>
        )}
      </div>
      {splits}
    </div>
  );
}

function mount(layout: string, bandOnly = false, foot?: Foot): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Grid layout={layout} bandOnly={bandOnly} foot={foot} />));
  roots.push({ root, host });
  return host;
}

const handle = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="separator"]');
const height = (host: HTMLElement, name: string) => host.querySelector<HTMLElement>(`[aria-label="${name}"]`)!.getBoundingClientRect().height;
const key = (on: Element, name: string) => act(() => void on.dispatchEvent(new KeyboardEvent("keydown", { key: name, bubbles: true, cancelable: true })));
const kept = (layout: string) => localStorage.getItem(`armada.bridge.deck-splits.${layout}`);

test("a gap between two panels is a handle named for both", async () => {
  const host = mount("session");
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  expect(handle(host)!.getAttribute("aria-label")).toBe("Resize Alpha and Beta");
  expect(handle(host)!.getAttribute("aria-orientation")).toBe("horizontal");
});

test("stepping a handle trades height between the panels either side, and the sizes are kept for the layout", async () => {
  const host = mount("session");
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  const before = [height(host, "Alpha"), height(host, "Beta")];
  key(handle(host)!, "ArrowDown");
  await vi.waitFor(() => expect(height(host, "Alpha")).toBeGreaterThan(before[0]!));
  expect(height(host, "Beta")).toBeLessThan(before[1]!);
  expect(JSON.parse(kept("session")!).left.size.alpha).toBeGreaterThan(JSON.parse(kept("session")!).left.size.beta);
  expect(kept("idle")).toBeNull();
});

test("sizes kept for one layout are not the other layout's", async () => {
  const session = mount("session");
  await vi.waitFor(() => expect(handle(session)).not.toBeNull());
  key(handle(session)!, "ArrowDown");
  await vi.waitFor(() => expect(kept("session")).not.toBeNull());
  const idle = mount("idle");
  await vi.waitFor(() => expect(handle(idle)).not.toBeNull());
  expect(Math.abs(height(idle, "Alpha") - height(idle, "Beta"))).toBeLessThan(2);
});

test("Home, and a double press on the gap, put the two panels back to their own sizes", async () => {
  const host = mount("session");
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  const start = height(host, "Alpha");
  key(handle(host)!, "ArrowDown");
  await vi.waitFor(() => expect(height(host, "Alpha")).toBeGreaterThan(start + 5));
  key(handle(host)!, "Home");
  await vi.waitFor(() => expect(Math.abs(height(host, "Alpha") - start)).toBeLessThan(2));

  key(handle(host)!, "ArrowUp");
  await vi.waitFor(() => expect(height(host, "Alpha")).toBeLessThan(start - 5));
  act(() => void handle(host)!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
  await vi.waitFor(() => expect(Math.abs(height(host, "Alpha") - start)).toBeLessThan(2));
});

test("a panel folded to its band stands outside the shares and has no handle beside it", async () => {
  const host = mount("session");
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  act(() => host.querySelector('[aria-label="Beta"]')!.setAttribute("data-collapsed", ""));
  await vi.waitFor(() => expect(handle(host)).toBeNull());
});

test("a panel that is only a band stands outside the shares and has no handle beside it", async () => {
  const host = mount("session", true);
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  expect(host.querySelectorAll('[role="separator"]')).toHaveLength(1);
  expect(handle(host)!.getAttribute("aria-label")).toBe("Resize Alpha and Beta");
});

// A share kept from the deck's old, taller foot drew the foot of keys 168px tall on a big screen, its keys floating in the middle.
test("a foot of keys alone stands outside the shares, even with a share kept for it", async () => {
  localStorage.setItem("armada.bridge.deck-splits.idle", JSON.stringify({ left: { size: { alpha: 0.4, beta: 0.3, "deck-foot": 0.3 }, natural: {} } }));
  const host = mount("idle", false, "keys");
  await vi.waitFor(() => expect(handle(host)).not.toBeNull());
  expect([...host.querySelectorAll('[role="separator"]')].map((one) => one.getAttribute("aria-label"))).toEqual(["Resize Alpha and Beta"]);
  expect(host.querySelector(".deck-foot")!.hasAttribute("data-sized")).toBe(false);
});

test("a foot holding the Field takes its share and has a handle above it", async () => {
  localStorage.setItem("armada.bridge.deck-splits.idle", JSON.stringify({ left: { size: { alpha: 0.4, beta: 0.3, "deck-foot": 0.3 }, natural: {} } }));
  const host = mount("idle", false, "field");
  await vi.waitFor(() => expect(host.querySelectorAll('[role="separator"]')).toHaveLength(2));
  expect(host.querySelector(".deck-foot")!.hasAttribute("data-sized")).toBe(true);
});
