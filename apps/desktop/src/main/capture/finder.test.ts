// What main asks of a page, and how it takes the answer — `docs/practices/capture-window.md`,
// *Steps Bridge plays*. The page is a fake that answers whatever the test says, because the page may
// answer with anything: what main keeps is a box inside the viewport and nothing else.

import type { WebContents } from "electron";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("virtual:armada-walk-finder", () => ({ default: "var ArmadaFinder = {};" }));

import { FINDER_WORLD, PageFinder } from "./finder";

const VIEWPORT = { width: 1000, height: 600 };

/** A page: what it says to the probe, and what it answers each expression with. */
function fakePage(answer: (code: string) => unknown, installed = false) {
  const asked: { world: number; code: string }[] = [];
  let has = installed;
  const contents = {
    isDestroyed: () => false,
    executeJavaScriptInIsolatedWorld: vi.fn(async (world: number, scripts: { code: string }[]) => {
      const code = scripts[0]!.code;
      asked.push({ world, code });
      if (code === 'typeof ArmadaFinder === "object"') return has;
      if (code.startsWith("var ArmadaFinder")) {
        has = true;
        return undefined;
      }
      return answer(code);
    }),
  } as unknown as WebContents;
  return { contents, asked, newDocument: () => void (has = false) };
}

const finderOn = (page: { contents: WebContents }) => new PageFinder(() => page.contents, () => VIEWPORT);

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("asking the page", () => {
  it("runs in the finder's own world and installs the finder once per document", async () => {
    const page = fakePage(() => ({ x: 10, y: 20, width: 30, height: 40 }));
    const finder = finderOn(page);
    await finder.arrive({ role: "button", name: "Add" }, 1000);
    await finder.box();
    await finder.box();
    expect(page.asked.every((one) => one.world === FINDER_WORLD)).toBe(true);
    expect(page.asked.filter((one) => one.code.startsWith("var ArmadaFinder"))).toHaveLength(1);

    page.newDocument();
    await finder.box();
    expect(page.asked.filter((one) => one.code.startsWith("var ArmadaFinder"))).toHaveLength(2);
  });

  it("serialises the target as JSON, so a name cannot become code", async () => {
    const page = fakePage(() => null);
    const finder = finderOn(page);
    const name = '"}); window.stolen = 1; ({"';
    await finder.arrive({ role: "button", name }, 500);
    expect(page.asked.at(-1)!.code).toBe(`ArmadaFinder.finder.arrive(${JSON.stringify({ role: "button", name })}, 500)`);
    await finder.words('");alert(1);//');
    expect(page.asked.at(-1)!.code).toBe(`ArmadaFinder.finder.words(${JSON.stringify('");alert(1);//')})`);
  });

  it("asks nothing of a page it may not ask: gone, or off the origin the window is pinned to", async () => {
    const finder = new PageFinder(() => null, () => VIEWPORT);
    expect(await finder.arrive({ role: "button" }, 100)).toBeNull();
    expect(await finder.box()).toBeNull();
    expect(await finder.focus()).toBeNull();
    expect(await finder.words("x")).toBeNull();
  });

  it("lets go of a page that never answers", async () => {
    const forever = fakePage(() => new Promise(() => undefined));
    const answered = finderOn(forever).arrive({ role: "button" }, 1000);
    await vi.advanceTimersByTimeAsync(1000 + 2000 + 10);
    expect(await answered).toBeNull();
  });

  it("answers nothing when the page navigated under the ask", async () => {
    const page = fakePage(() => {
      throw new Error("Execution context was destroyed");
    });
    expect(await finderOn(page).box()).toBeNull();
  });
});

describe("taking the answer", () => {
  it("keeps a box inside the viewport, whatever the page said", async () => {
    const clipped = fakePage(() => ({ x: 900, y: 10, width: 99999, height: 30 }));
    expect(await finderOn(clipped).box()).toEqual({ x: 900, y: 10, width: 100, height: 30 });
    const below = fakePage(() => ({ x: -50, y: 700, width: 20, height: 20 }));
    expect(await finderOn(below).box()).toBeNull();
  });

  it("answers no box for anything that is not one", async () => {
    for (const nonsense of ["a string", 7, null, undefined, [], { x: "1", y: "2", width: "3", height: "4" }, { x: 1, y: 1, width: 0, height: 5 }]) {
      expect(await finderOn(fakePage(() => nonsense)).box()).toBeNull();
    }
  });

  it("says what the held element is only in the three words it knows", async () => {
    for (const [said, kind] of [["field", "field"], ["select", "select"], ["other", "other"], ["button", null], [3, null], [null, null]] as const) {
      expect(await finderOn(fakePage(() => said)).focus()).toBe(kind);
    }
  });

  it("keeps the points a selection runs between inside the viewport, and refuses a half answer", async () => {
    const across = fakePage(() => ({ from: { x: -5, y: 10 }, to: { x: 5000, y: 20 } }));
    expect(await finderOn(across).words("bolts")).toEqual({ from: { x: 0, y: 10 }, to: { x: 1000, y: 20 } });
    const half = fakePage(() => ({ from: { x: 1, y: 1 } }));
    expect(await finderOn(half).words("bolts")).toBeNull();
  });
});
