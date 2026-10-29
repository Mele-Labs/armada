// A panel folds to its header band when the band is pressed, and the fold is kept for that panel by name. Browser: a press and a
// rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { useCollapse, useFlightExpanded } from "./collapse";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => localStorage.clear());
afterEach(() => {
  vi.restoreAllMocks();
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

function Expander() {
  const { expanded, toggle } = useFlightExpanded();
  return (
    <button type="button" aria-expanded={expanded} onClick={toggle}>
      Expand
    </button>
  );
}

function mountExpander(): HTMLButtonElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Expander />));
  roots.push({ root, host });
  return host.querySelector("button")!;
}

const expanded = (button: HTMLElement) => button.getAttribute("aria-expanded") === "true";

test("the flight control starts compact, and a press expands it and a second puts it back", () => {
  const button = mountExpander();
  expect(expanded(button)).toBe(false);
  act(() => button.click());
  expect(expanded(button)).toBe(true);
  act(() => button.click());
  expect(expanded(button)).toBe(false);
});

test("the flight control stays expanded when the view is entered again, and one choice serves every mount", () => {
  const button = mountExpander();
  act(() => button.click());
  expect(localStorage.getItem("armada.flight-expanded")).toBe("1");
  expect(expanded(mountExpander())).toBe(true);
  act(() => button.click());
  expect(localStorage.getItem("armada.flight-expanded")).toBe("0");
  expect(expanded(mountExpander())).toBe(false);
});

test("expanding the flight control folds no panel", () => {
  const button = mountExpander();
  act(() => button.click());
  expect(folded(mount("controls"))).toBe(false);
});

test("a blocked store only forgets whether the flight control is expanded", () => {
  vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
    throw new Error("blocked");
  });
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("blocked");
  });
  const button = mountExpander();
  expect(expanded(button)).toBe(false);
  act(() => button.click());
  expect(expanded(button)).toBe(true);
});
