import { describe, expect, it, vi } from "vitest";

import type { KeptStyles } from "@armada/protocol";

import { createKeptStyles } from "./kept-styles";

const HASH = "0123456789abcdef0123456789abcdef";
const SET: KeptStyles = {
  sheets: [
    { path: "packages/tokens/tokens.css", css: ":root{--a:1}", edited: false },
    { path: "packages/components/src/JobCard.css", css: ".card{color:red}", edited: true },
  ],
  fonts: [{ family: "IBM Plex Sans", weight: "100 900", style: "italic", bytes: btoa("wOF2") }],
};

describe("the kept style set of a mock", () => {
  it("decodes a font's bytes and keeps its weight as written", async () => {
    const resolve = createKeptStyles(async () => SET, () => null);
    const read = await resolve({ id: "m", styles: [], kept: HASH });
    expect(read.fonts).toHaveLength(1);
    const font = read.fonts[0]!;
    expect(font.weight).toBe("100 900");
    expect(font.style).toBe("italic");
    expect(Array.from(font.bytes as Uint8Array)).toEqual([0x77, 0x4f, 0x46, 0x32]);
  });

  it("gives the sheets in the order they apply, and their paths as what it was drawn with", async () => {
    const resolve = createKeptStyles(async () => SET, () => null);
    const read = await resolve({ id: "m", styles: [], kept: HASH });
    expect(read.css).toEqual([":root{--a:1}", ".card{color:red}"]);
    expect(read.drawnWith).toEqual(["packages/tokens/tokens.css", "packages/components/src/JobCard.css"]);
  });

  it("reads once for ten frames that name one hash", async () => {
    const read = vi.fn(async () => SET);
    const resolve = createKeptStyles(read, () => null);
    await Promise.all(Array.from({ length: 10 }, (_, at) => resolve({ id: `m${String(at)}`, styles: [], kept: HASH })));
    expect(read).toHaveBeenCalledTimes(1);
    expect(read).toHaveBeenCalledWith(HASH);
  });

  it("sets the current theme on the frame's root", async () => {
    const resolve = createKeptStyles(async () => SET, () => "light");
    expect((await resolve({ id: "m", styles: [], kept: HASH })).root).toEqual({ "data-theme": "light" });
  });

  it("answers no stylesheet where the node has no kept hash, without asking Fleet", async () => {
    const read = vi.fn();
    const resolve = createKeptStyles(read, () => "dark");
    expect(await resolve({ id: "m", styles: ["a.css"], kept: undefined })).toEqual({ css: [], fonts: [], drawnWith: [] });
    expect(read).not.toHaveBeenCalled();
  });

  it("rejects a read that failed, and asks again for the next frame", async () => {
    const read = vi.fn<(hash: string) => Promise<KeptStyles | undefined>>();
    read.mockResolvedValueOnce(undefined);
    read.mockResolvedValueOnce(SET);
    const resolve = createKeptStyles(read, () => null);
    await expect(resolve({ id: "m", styles: [], kept: HASH })).rejects.toThrow();
    expect((await resolve({ id: "m", styles: [], kept: HASH })).drawnWith).toHaveLength(2);
    expect(read).toHaveBeenCalledTimes(2);
  });
});
