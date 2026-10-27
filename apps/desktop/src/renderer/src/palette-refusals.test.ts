// What a key or a word cannot be for a palette row, and the reason each refusal gives. The panel on ⌘K and Settings' table take
// the same two functions, so a refusal here is the one both say.

import { parseChord, refuseAlias, refuseHotkey, type Chord } from "@armada/components";
import { describe, expect, test } from "vitest";

const chord = (spelled: string): Chord => parseChord(spelled)!;

describe("a key for a row", () => {
  test("needs ⌘ or ⌃", () => {
    expect(refuseHotkey(chord("J"), [])).toBe("Needs ⌘ or ⌃");
    expect(refuseHotkey(chord("⇧J"), [])).toBe("Needs ⌘ or ⌃");
    expect(refuseHotkey(chord("⌥J"), [])).toBe("Needs ⌘ or ⌃");
  });

  test("is not ⌘1 to ⌘9, which open results", () => {
    for (const digit of ["1", "5", "9"]) expect(refuseHotkey(chord(`⌘${digit}`), [])).toBe("⌘1 to ⌘9 open results");
    // Only the bare ⌘ digit: with another modifier it opens nothing.
    expect(refuseHotkey(chord("⇧⌘5"), [])).toBeUndefined();
  });

  test("is not a key an act answers, and says which act", () => {
    expect(refuseHotkey(chord("⌘K"), [])).toMatch(/^Bound to .+/);
  });

  test("is not a key the system keeps", () => {
    expect(refuseHotkey(chord("⌘C"), [])).toBe("Kept by the system");
    expect(refuseHotkey(chord("⌘Q"), [])).toBe("Kept by the system");
  });

  test("is not another row's, and names that row", () => {
    const others = [{ label: "Reviews", hotkey: "⇧⌘J" }];
    expect(refuseHotkey(chord("⇧⌘J"), others)).toBe("Bound to Reviews");
    expect(refuseHotkey(chord("⇧⌘L"), others)).toBeUndefined();
  });

  test("a free key with ⌘ or ⌃ is kept", () => {
    expect(refuseHotkey(chord("⇧⌘J"), [])).toBeUndefined();
    expect(refuseHotkey(chord("⌃⌥U"), [])).toBeUndefined();
  });
});

describe("a word for a row", () => {
  test("is one word", () => {
    expect(refuseAlias("", [])).toBe("Needs a word");
    expect(refuseAlias("two words", [])).toBe("One word");
  });

  test("is not another row's, however it is cased, and names that row", () => {
    const others = [{ label: "Reviews", alias: "rev" }];
    expect(refuseAlias("REV", others)).toBe("Used by Reviews");
    expect(refuseAlias("rv", others)).toBeUndefined();
  });
});
