// Mod and Report on every context menu opened at a point, for the element last right-pressed.

import { useEffect, useState } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import { DropdownMenu } from "@armada/components";

import "./styles/index.css";
import type { AnnotationsDevApi } from "../../shared/annotations";
import { startModJob } from "./mod-job";
import { PointActsProvided } from "./point-acts";

vi.mock("./mod-job", () => ({ startModJob: vi.fn(async () => undefined) }));

let mounted: { root: Root; host: HTMLElement }[] = [];

beforeEach(() => {
  // A build with the dev layer, so Report is offered; a test that wants a shipped build says so before it mounts.
  window.armadaDev = {} as AnnotationsDevApi;
  vi.mocked(startModJob).mockClear();
});

afterEach(() => {
  vi.unstubAllEnvs();
  window.armadaDev = undefined;
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-pointed]").forEach((one) => one.remove());
});

/** An app menu of its own, opened where any context event lands, as the Cockpit's row menu is. */
function AppMenu({ own }: { own: (id: string) => void }) {
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    const opened = (event: MouseEvent): void => setAt({ x: event.clientX, y: event.clientY });
    // And by a key or a button, with no right press under it.
    const byKey = (): void => setAt({ x: 40, y: 40 });
    window.addEventListener("contextmenu", opened);
    window.addEventListener("open-by-key", byKey);
    return () => {
      window.removeEventListener("contextmenu", opened);
      window.removeEventListener("open-by-key", byKey);
    };
  }, []);
  if (at === null) return null;
  return (
    <DropdownMenu
      open
      at={at}
      triggerLabel="Row"
      entries={[{ kind: "item", id: "pause", label: "Pause" }]}
      onOpenChange={(open) => {
        if (!open) setAt(null);
      }}
      onSelect={own}
    />
  );
}

/** Mounted, with the provider's listeners on: a press before the effect runs finds nobody listening. */
async function mount(own: (id: string) => void = () => undefined): Promise<void> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  flushSync(() =>
    root.render(
      <PointActsProvided>
        <AppMenu own={own} />
      </PointActsProvided>,
    ),
  );
  mounted.push({ root, host });
  await new Promise((done) => setTimeout(done, 0));
}

function element(tag = "button"): HTMLElement {
  const one = document.createElement(tag);
  one.setAttribute("data-pointed", "");
  one.textContent = "Pointed";
  document.body.append(one);
  return one;
}

const rightPress = (on: EventTarget): boolean => on.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 40, clientY: 40 }));

/** What Report sends, in the window event the annotation layer listens for. */
function reports(): { asked: ReturnType<typeof vi.fn>; stop: () => void } {
  const asked = vi.fn();
  window.addEventListener("armada:report", asked);
  return { asked, stop: () => window.removeEventListener("armada:report", asked) };
}

test("an app menu lists Mod and Report after its own entries, below a rule", async () => {
  await mount();
  rightPress(element());
  await expect.element(page.getByRole("menuitem", { name: "Mod" })).toBeVisible();
  expect([...document.querySelectorAll('[role="menuitem"]')].map((one) => one.textContent?.trim())).toEqual(["Pause", "Mod", "Report"]);
  expect(document.querySelectorAll('[role="separator"]')).toHaveLength(1);
});

test("a build without the dev layer offers Mod and no Report", async () => {
  window.armadaDev = undefined;
  vi.stubEnv("DEV", false);
  await mount();
  rightPress(element());
  await expect.element(page.getByRole("menuitem", { name: "Mod" })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "Report" }).query()).toBeNull();
});

test("choosing Report asks to report the element that was right-pressed, and not the menu's own handler", async () => {
  const own = vi.fn();
  await mount(own);
  const pointed = element();
  rightPress(pointed);
  const { asked, stop } = reports();
  await page.getByRole("menuitem", { name: "Report" }).click();
  stop();
  expect(asked).toHaveBeenCalledTimes(1);
  expect((asked.mock.calls[0]?.[0] as CustomEvent<Element>).detail).toBe(pointed);
  expect(own).not.toHaveBeenCalled();
  expect(startModJob).not.toHaveBeenCalled();
});

test("choosing Mod starts a Mod Job on that element", async () => {
  const own = vi.fn();
  await mount(own);
  const pointed = element();
  rightPress(pointed);
  await page.getByRole("menuitem", { name: "Mod" }).click();
  expect(startModJob).toHaveBeenCalledTimes(1);
  expect(startModJob).toHaveBeenCalledWith(pointed);
  expect(own).not.toHaveBeenCalled();
});

test("the menu's own entry still goes to the menu", async () => {
  const own = vi.fn();
  await mount(own);
  rightPress(element());
  const { asked, stop } = reports();
  await page.getByRole("menuitem", { name: "Pause" }).click();
  stop();
  expect(own).toHaveBeenCalledWith("pause");
  expect(asked).not.toHaveBeenCalled();
  expect(startModJob).not.toHaveBeenCalled();
});

test("an SVG pressed on is taken whole, as the glyph and not one path in it", async () => {
  await mount();
  const glyph = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  glyph.setAttribute("data-pointed", "");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  glyph.append(path);
  document.body.append(glyph);
  rightPress(path);
  await page.getByRole("menuitem", { name: "Mod" }).click();
  expect(startModJob).toHaveBeenCalledWith(glyph);
});

test("a menu opened by key, with no right press before it, is for the element last focused", async () => {
  await mount();
  const focused = element();
  focused.focus();
  window.dispatchEvent(new Event("open-by-key"));
  await page.getByRole("menuitem", { name: "Mod" }).click();
  expect(startModJob).toHaveBeenCalledWith(focused);
});

test("a press or a key outside a menu forgets the last right press, so a menu opened by key is not read as that one", async () => {
  await mount();
  const pressed = element();
  const other = element();
  rightPress(pressed);
  other.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
  // Nothing was focused: the menu below has no element to act on.
  window.dispatchEvent(new Event("open-by-key"));
  await page.getByRole("menuitem", { name: "Mod" }).click();
  expect(startModJob).not.toHaveBeenCalled();
});

test("a right press inside the annotation layer is not noted", async () => {
  await mount();
  const layer = document.createElement("div");
  layer.setAttribute("data-armada-annotate", "");
  layer.setAttribute("data-pointed", "");
  const inside = document.createElement("button");
  layer.append(inside);
  document.body.append(layer);
  rightPress(inside);
  await page.getByRole("menuitem", { name: "Mod" }).click();
  expect(startModJob).not.toHaveBeenCalled();
});

test("an element that has left the page since is not acted on", async () => {
  await mount();
  const pointed = element();
  rightPress(pointed);
  pointed.remove();
  const { asked, stop } = reports();
  await page.getByRole("menuitem", { name: "Report" }).click();
  stop();
  expect(asked).not.toHaveBeenCalled();
});
