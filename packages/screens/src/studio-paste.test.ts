// Each shape a real ⌘V handed Chromium on macOS, 1 Oct 2026, and what it lands as.

import { describe, expect, test } from "vitest";

import { isPath, landingOf, type Pasted } from "./studio-paste";

const text = (said: string): Pasted => ({ text: said, files: [] });
const ADDRESS = "https://example.invalid/armada/pull/1721";
const ON_DISK = "/Users/user/Development/armada/crates/fleet/src/briefing.rs";

describe("what a real paste lands as", () => {
  test("an address from the address bar, or Copy Link, is a Link", () => {
    expect(landingOf(text(ADDRESS))).toEqual({ kind: "link", address: ADDRESS });
  });

  test("a sentence copied off a page is a Note, its HTML ignored", () => {
    expect(landingOf(text("Plain sentence to copy from the page."))).toEqual({
      kind: "note",
      said: "Plain sentence to copy from the page.",
    });
  });

  test("a path copied in a terminal is a File, trimmed of the line's tail", () => {
    expect(landingOf(text(`${ON_DISK}\n   `))).toEqual({ kind: "file", path: ON_DISK });
    expect(landingOf(text("crates/fleet/src/briefing.rs"))).toEqual({
      kind: "file",
      path: "crates/fleet/src/briefing.rs",
    });
  });

  test("a file copied in Finder is a File at its path on disk, and carries no text", () => {
    expect(landingOf({ text: "", files: [{ type: "", path: ON_DISK }] })).toEqual({ kind: "file", path: ON_DISK });
  });

  test("an image file copied in Finder is still a File: what is on the clipboard beside it is its icon", () => {
    const png = "/Users/user/Development/armada/packages/brand/covers/armada-cover-hero.png";
    expect(landingOf({ text: "", files: [{ type: "image/png", path: png }] })).toEqual({ kind: "file", path: png });
  });

  test("a screenshot is a picture: an image with no path on disk", () => {
    expect(landingOf({ text: "", files: [{ type: "image/png", path: "" }] })).toEqual({ kind: "picture" });
  });

  test("a picture wins over text beside it", () => {
    expect(landingOf({ text: "a caption", files: [{ type: "image/png", path: "" }] })).toEqual({ kind: "picture" });
  });

  test("nothing on the clipboard lands nothing", () => {
    expect(landingOf(text("  \n"))).toBeNull();
    expect(landingOf({ text: "", files: [{ type: "", path: "" }] })).toBeNull();
  });
});

describe("what reads as a path", () => {
  test.each(["/Users/user/notes.md", "~/Desktop/notes.md", "crates/fleet/src/briefing.rs", "./src/main.rs", "crates/fleet/"])(
    "%s is a path",
    (path) => expect(isPath(path)).toBe(true),
  );

  test.each(["and/or", "1/2", "TCP/IP", "src", "see crates/fleet/src/briefing.rs", "/", "e.g./i.e.", "a\n/b"])(
    "%s is not",
    (words) => expect(isPath(words)).toBe(false),
  );
});
