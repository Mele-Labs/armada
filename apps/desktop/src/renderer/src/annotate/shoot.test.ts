import { describe, expect, it } from "vitest";

import { SHOOTING, withoutLayer } from "./shoot";

/** The attribute half of an element, which is all `withoutLayer` touches. */
function layer() {
  const attributes = new Set<string>();
  const element = {
    setAttribute: (name: string) => void attributes.add(name),
    removeAttribute: (name: string) => void attributes.delete(name),
  } as unknown as Element;
  return { element, out: () => attributes.has(SHOOTING) };
}

/** Frames that run only when the test says so. */
function frames() {
  const waiting: (() => void)[] = [];
  return {
    frame: (done: () => void) => void waiting.push(done),
    paint: async () => {
      waiting.splice(0).forEach((done) => done());
      await Promise.resolve();
    },
  };
}

describe("a picture without the layer", () => {
  it("is taken only after a frame has been painted with the layer out, and puts the layer back", async () => {
    const { element, out } = layer();
    const { frame, paint } = frames();
    const seen: boolean[] = [];
    const taken = withoutLayer(element, async () => void seen.push(out()), frame);

    expect(out()).toBe(true);
    await paint();
    expect(seen).toEqual([]);
    await paint();
    await taken;
    expect(seen).toEqual([true]);
    expect(out()).toBe(false);
  });

  it("keeps the layer out until the last of two overlapping shots is taken, and back after a failed one", async () => {
    const { element, out } = layer();
    const { frame, paint } = frames();
    let finish = (): void => {};
    const first = withoutLayer(element, () => new Promise<void>((done) => (finish = done)), frame);
    const second = withoutLayer(element, async () => Promise.reject(new Error("no window")), frame);
    await paint();
    await paint();
    await expect(second).rejects.toThrow("no window");
    expect(out()).toBe(true);
    finish();
    await first;
    expect(out()).toBe(false);
  });
});
