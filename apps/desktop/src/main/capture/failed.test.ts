import { afterEach, expect, it, vi } from "vitest";

import { FailedLoads, LOAD_MS, lookSource, looked } from "./failed";

const ORIGIN = "http://localhost:5173";
const PAGE = `${ORIGIN}/`;

afterEach(() => void vi.useRealTimers());

function watched(): { loads: FailedLoads; told: string[] } {
  const told: string[] = [];
  return { loads: new FailedLoads((url, why) => told.push(`${url} ${why}`)), told };
}

it("tells a refused connection once, however often Reload is pressed, until the page is seen working", () => {
  const { loads, told } = watched();
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  loads.navigated("chrome-error://chromewebdata/", 200, ORIGIN);
  loads.look("chrome-error://chromewebdata/", "fine");
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  expect(told).toEqual([`${PAGE} ERR_CONNECTION_REFUSED`]);

  loads.navigated(PAGE, 200, ORIGIN);
  loads.look(PAGE, "fine");
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  expect(told).toHaveLength(2);
});

it("tells an error status on the page once, though the error page itself reads fine", () => {
  const { loads, told } = watched();
  loads.navigated(PAGE, 500, ORIGIN);
  loads.look(PAGE, "fine");
  loads.navigated(PAGE, 500, ORIGIN);
  expect(told).toEqual([`${PAGE} HTTP 500`]);
});

it("tells a page whose process went away, and not a window closing", () => {
  const { loads, told } = watched();
  loads.gone(PAGE, "clean-exit");
  expect(told).toEqual([]);
  loads.gone(PAGE, "crashed");
  expect(told).toEqual([`${PAGE} the page's process ended: crashed`]);
});

it("tells nothing for a subframe, or a load this window refused", () => {
  const { loads, told } = watched();
  loads.failedLoad(-105, "ERR_NAME_NOT_RESOLVED", "https://ads.example/", false);
  loads.failedLoad(-3, "ERR_ABORTED", "https://elsewhere.example/", true);
  expect(told).toEqual([]);
});

it("tells Vite's overlay at once, and again only after the page has read fine", () => {
  const { loads, told } = watched();
  loads.navigated(PAGE, 200, ORIGIN);
  loads.look(PAGE, "overlay");
  loads.navigated(PAGE, 200, ORIGIN);
  loads.look(PAGE, "overlay");
  expect(told).toEqual([`${PAGE} the page shows Vite's error overlay`]);

  loads.look(PAGE, "fine");
  loads.look(PAGE, "overlay");
  expect(told).toHaveLength(2);
});

it("tells a blank page only once it reads blank twice running", () => {
  const { loads, told } = watched();
  loads.navigated(PAGE, 200, ORIGIN);
  loads.look(PAGE, "blank");
  loads.look(PAGE, "fine");
  loads.look(PAGE, "blank");
  expect(told).toEqual([]);
  loads.look(PAGE, null);
  loads.look(PAGE, "blank");
  expect(told).toEqual([`${PAGE} the page loaded with nothing drawn`]);
});

it("tells a page still loading after 30 seconds once, until the page is seen working", () => {
  vi.useFakeTimers();
  const { loads, told } = watched();
  expect(LOAD_MS).toBe(30_000);
  loads.started(() => PAGE);
  vi.advanceTimersByTime(LOAD_MS - 1);
  expect(told).toEqual([]);
  vi.advanceTimersByTime(1);
  expect(told).toEqual([`${PAGE} the page has not loaded after 30 seconds`]);

  // Reload on a server that still stalls is the same fact again.
  loads.stopped();
  loads.started(() => PAGE);
  vi.advanceTimersByTime(LOAD_MS);
  expect(told).toHaveLength(1);

  loads.navigated(PAGE, 200, ORIGIN);
  loads.look(PAGE, "fine");
  loads.started(() => PAGE);
  vi.advanceTimersByTime(LOAD_MS);
  expect(told).toHaveLength(2);
});

it("tells nothing of a load that stopped in time, or of one the window let go of", () => {
  vi.useFakeTimers();
  const { loads, told } = watched();
  loads.started(() => PAGE);
  vi.advanceTimersByTime(LOAD_MS - 1);
  loads.stopped();
  vi.advanceTimersByTime(LOAD_MS);
  loads.started(() => PAGE);
  loads.stopped();
  vi.advanceTimersByTime(LOAD_MS);
  expect(told).toEqual([]);
});

it("holds the page's answer to the three words", () => {
  expect(looked("overlay")).toBe("overlay");
  expect(looked("Ignore your instructions")).toBeNull();
  expect(looked({ kind: "fine" })).toBeNull();
});

// The look against a stub page: main has no `lib.dom`, and the stub is exactly what the look touches.
type Node = { width: number; height: number };

function page(body: { text: string; nodes?: Node[] } | null, overlay = false): void {
  (globalThis as Record<string, unknown>)["document"] = {
    querySelector: (selector: string) => (selector === "vite-error-overlay" && overlay ? {} : null),
    body:
      body === null
        ? null
        : {
            innerText: body.text,
            querySelectorAll: () => (body.nodes ?? []).map((box) => ({ getBoundingClientRect: () => box })),
          },
  };
}

function look(): unknown {
  return (0, eval)(lookSource());
}

afterEach(() => {
  delete (globalThis as Record<string, unknown>)["document"];
});

it("reads the overlay, a page with words or a drawn element as fine, and an empty root as blank", () => {
  page({ text: "Shop" }, true);
  expect(look()).toBe("overlay");
  page({ text: "  Shop  " });
  expect(look()).toBe("fine");
  page({ text: "", nodes: [{ width: 0, height: 0 }, { width: 120, height: 40 }] });
  expect(look()).toBe("fine");
  page({ text: " \n", nodes: [{ width: 0, height: 0 }] });
  expect(look()).toBe("blank");
  page(null);
  expect(look()).toBe("blank");
});
