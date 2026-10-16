// A panel folds to its header band when the band is pressed, and the fold is kept for that panel by name. Browser: a press and a
// rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";

import { useCollapse } from "./collapse";

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

function Panel({ id }: { id: string }) {
  const { collapsed, band } = useCollapse(id);
  return (
    <section aria-label={id} data-collapsed={collapsed || undefined}>
      <header {...band}>
        {id}
        <button type="button">Act</button>
      </header>
    </section>
  );
}

function mount(id: string): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Panel id={id} />));
  roots.push({ root, host });
  return host;
}

const band = (host: HTMLElement) => host.querySelector<HTMLElement>("header")!;
const folded = (host: HTMLElement) => host.querySelector("section")!.hasAttribute("data-collapsed");

test("pressing the band folds the panel and a second press opens it", () => {
  const host = mount("plan");
  expect(band(host).getAttribute("aria-expanded")).toBe("true");
  act(() => band(host).click());
  expect(folded(host)).toBe(true);
  expect(band(host).getAttribute("aria-expanded")).toBe("false");
  act(() => band(host).click());
  expect(folded(host)).toBe(false);
});

test("Enter and Space on the band fold it, a key anywhere else does not", () => {
  const host = mount("plan");
  const press = (key: string, on: Element) => act(() => void on.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true })));
  press("Enter", band(host));
  expect(folded(host)).toBe(true);
  press(" ", band(host));
  expect(folded(host)).toBe(false);
  press("a", band(host));
  press("Enter", host.querySelector("button")!);
  expect(folded(host)).toBe(false);
});

test("an act in the band is its own press and does not fold the panel", () => {
  const host = mount("plan");
  act(() => host.querySelector("button")!.click());
  expect(folded(host)).toBe(false);
});

test("which panels are folded is kept for each, by name, and comes back when the view is entered again", () => {
  const one = mount("plan");
  const other = mount("field");
  act(() => band(one).click());
  expect(localStorage.getItem("armada.panel-collapsed.plan")).toBe("1");
  expect(folded(other)).toBe(false);
  expect(folded(mount("plan"))).toBe(true);
  expect(folded(mount("field"))).toBe(false);
  act(() => band(one).click());
  expect(localStorage.getItem("armada.panel-collapsed.plan")).toBe("0");
  expect(folded(mount("plan"))).toBe(false);
});
