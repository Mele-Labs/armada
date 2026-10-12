// Finding the place a Session asked about, against a stub page — `layer.test.ts`'s reason: main has
// no `lib.dom`, and the stub is exactly what `locate.ts` declares it may touch.

import { afterEach, expect, it } from "vitest";

import { findsThePlace, locateSource } from "./locate";

type Stub = { text: string; box: { x: number; y: number; width: number; height: number }; scrolled: number };

function page(nodes: Stub[], refuses = false): void {
  (globalThis as Record<string, unknown>)["document"] = {
    querySelectorAll: (selector: string) => {
      if (refuses) throw new Error(`not a selector: ${selector}`);
      return nodes.map((one) => ({
        textContent: one.text,
        getBoundingClientRect: () => one.box,
        scrollIntoView: () => {
          one.scrolled += 1;
        },
      }));
    },
  };
}

const stub = (text: string, y: number): Stub => ({ text, box: { x: 10.4, y, width: 80, height: 20.6 }, scrolled: 0 });

afterEach(() => {
  delete (globalThis as Record<string, unknown>)["document"];
});

it("picks the match that shows the text, and scrolls it into view only when asked", () => {
  const nodes = [stub("Overview", 0), stub("  Stale\n counts ", 400)];
  page(nodes);
  expect(findsThePlace("h2", "stale counts", true)).toEqual({ box: { x: 10, y: 400, width: 80, height: 21 } });
  expect(nodes[1]?.scrolled).toBe(1);
  findsThePlace("h2", "stale counts", false);
  expect(nodes[1]?.scrolled).toBe(1);
});

it("falls back to the first match, and answers nothing where nothing matches or the selector will not parse", () => {
  page([stub("Overview", 5), stub("Other", 9)]);
  expect(findsThePlace("h2", "not on the page", false)).toEqual({ box: { x: 10, y: 5, width: 80, height: 21 } });
  page([]);
  expect(findsThePlace("h2", null, true)).toBeNull();
  page([stub("x", 0)], true);
  expect(findsThePlace("h2[", null, true)).toBeNull();
});

it("carries every argument as a value, so nothing the page holds changes what is asked", () => {
  const source = locateSource('a[title="x"]"); evil(', null, true);
  expect(source).toContain(JSON.stringify('a[title="x"]"); evil('));
  page([stub("x", 3)]);
  // The source is one expression; evaluated, it answers as the function does.
  expect(new Function(`return ${source}`)()).toEqual({ box: { x: 10, y: 3, width: 80, height: 21 } });
});
