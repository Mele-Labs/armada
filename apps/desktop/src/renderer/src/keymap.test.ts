// The keymap in `@armada/components`: what a press means once a person has rebound keys, read the way
// every handler in Bridge now asks. In node: nothing here needs a document.

import { afterEach, describe, expect, it } from "vitest";
import {
  ACTIONS,
  conflictsOf,
  defaultSlotsOf,
  digitKeyFor,
  formatSlot,
  isPressed,
  keyFor,
  parseChord,
  parseKeyBindings,
  parseSlots,
  pressedDigit,
  pressedSlot,
  rebind,
  resetAllBindings,
  serializeKeyBindings,
  setKeyBindingSaver,
  setKeyBindings,
  slotsOf,
} from "@armada/components";

const press = (key: string, mods: { meta?: boolean; ctrl?: boolean; alt?: boolean; shift?: boolean; code?: string } = {}) => ({
  key,
  metaKey: mods.meta ?? false,
  ctrlKey: mods.ctrl ?? false,
  altKey: mods.alt ?? false,
  shiftKey: mods.shift ?? false,
  ...(mods.code === undefined ? {} : { code: mods.code }),
});

afterEach(() => {
  setKeyBindingSaver(null);
  setKeyBindings({});
});

describe("reading the registry's spellings", () => {
  it("reads every shipped binding back to the same spelling", () => {
    for (const act of ACTIONS) {
      const slots = parseSlots(act.shortcut);
      expect(slots.every((slot) => formatSlot(slot) !== ""), act.id).toBe(true);
      expect(keyFor(act.id)).toBe(act.shortcut);
    }
  });

  it("reads a shifted letter, a symbol, and a named key", () => {
    expect(parseChord("B")).toEqual({ ctrl: false, alt: false, shift: true, meta: false, key: "b" });
    expect(parseChord("⇧⌘R")).toEqual({ ctrl: false, alt: false, shift: true, meta: true, key: "r" });
    expect(parseChord("⌘K")).toEqual({ ctrl: false, alt: false, shift: false, meta: true, key: "k" });
    expect(parseChord("?")?.shift).toBe(false);
    expect(parseChord("Esc")?.key).toBe("Esc");
    expect(parseSlots("⌘[ ⌘]")).toHaveLength(2);
    expect(parseSlots("j / k / ↓ / ↑")).toHaveLength(4);
    expect(parseSlots("⌘1–⌘9")[0]).toMatchObject({ kind: "digits", from: 1, to: 9 });
  });

  it("ships with no two acts on one key where both listen", () => {
    for (const act of ACTIONS) {
      for (const slot of defaultSlotsOf(act.id)) expect(conflictsOf(act.id, slot).map((one) => one.id), act.id).toEqual([]);
    }
  });
});

describe("a press, as the shipped keys answer it", () => {
  it("takes Control for ⌘, reads shift exactly, and Option by the key's place", () => {
    expect(isPressed("command_palette", press("k", { meta: true }))).toBe(true);
    expect(isPressed("command_palette", press("k", { ctrl: true }))).toBe(true);
    expect(isPressed("command_palette", press("k"))).toBe(false);
    expect(isPressed("refresh", press("r", { meta: true, shift: true }))).toBe(true);
    expect(isPressed("refresh", press("r", { meta: true }))).toBe(false);
    expect(isPressed("raise_cost_cap", press("B", { shift: true }))).toBe(true);
    expect(isPressed("report_job", press("B", { shift: true }))).toBe(false);
    expect(isPressed("capture_note", press("ç", { meta: true, alt: true, code: "KeyC" }))).toBe(true);
    expect(isPressed("key_sheet", press("?", { shift: true }))).toBe(true);
    expect(pressedSlot("history", press("]", { meta: true }))).toBe(1);
    expect(pressedDigit("bridge_surfaces", press("3", { meta: true }))).toBe(3);
    expect(pressedDigit("dashboard_filter_number", press("¡", { alt: true, code: "Digit1" }))).toBe(1);
    expect(pressedDigit("dashboard_filter_number", press("4", { alt: true, code: "Digit4" }))).toBeNull();
  });
});

describe("a key a person rebound", () => {
  it("answers the new key, not the old one, and draws it", () => {
    rebind("kill", [parseSlots("q")[0]!]);
    expect(isPressed("kill", press("q"))).toBe(true);
    expect(isPressed("kill", press("x"))).toBe(false);
    expect(keyFor("kill")).toBe("q");
  });

  it("moves one key of a pair and leaves the other", () => {
    const [, forward] = slotsOf("history");
    rebind("history", [parseSlots("⌥⌘←")[0]!, forward!]);
    expect(pressedSlot("history", press("ArrowLeft", { meta: true, alt: true }))).toBe(0);
    expect(pressedSlot("history", press("[", { meta: true }))).toBe(-1);
    expect(pressedSlot("history", press("]", { meta: true }))).toBe(1);
    expect(keyFor("history")).toBe("⌥⌘← ⌘]");
  });

  it("moves a row of digits by its modifiers", () => {
    rebind("bridge_surfaces", parseSlots("⌃1–⌃9"));
    expect(pressedDigit("bridge_surfaces", press("2", { ctrl: true }))).toBe(2);
    expect(pressedDigit("bridge_surfaces", press("2", { meta: true }))).toBeNull();
    expect(digitKeyFor("bridge_surfaces", 2)).toBe("⌃2");
  });

  it("unbinds a key, and binding the shipped key back takes the change away", () => {
    rebind("kill", [{ kind: "chord", chord: null }]);
    expect(isPressed("kill", press("x"))).toBe(false);
    expect(keyFor("kill")).toBe("");
    rebind("kill", parseSlots("x"));
    expect(serializeKeyBindings({})).toBe("");
    expect(keyFor("kill")).toBe("x");
  });

  it("names the act it now shares a key with", () => {
    rebind("kill", parseSlots("c"));
    expect(conflictsOf("kill", slotsOf("kill")[0]!).map((one) => one.id)).toEqual(["copy_debug_info"]);
    // `o` opens a row on a list and the output on a Job: never the same screen, so never a conflict.
    expect(conflictsOf("open_output", slotsOf("open_output")[0]!)).toEqual([]);
  });

  it("hands every change to the saver as the text Fleet keeps, and empty once all are reset", () => {
    const saved: string[] = [];
    setKeyBindingSaver((text) => saved.push(text));
    rebind("kill", parseSlots("q"));
    expect(parseKeyBindings(saved[0]!)).toEqual({ kill: ["q"] });
    resetAllBindings();
    expect(saved[1]).toBe("");
  });
});

describe("the text Fleet keeps", () => {
  it("reads only the shape Fleet accepts, and none of a wrong one", () => {
    expect(parseKeyBindings('{"version":1,"bindings":{"kill":["q"]}}')).toEqual({ kill: ["q"] });
    expect(parseKeyBindings("")).toEqual({});
    expect(parseKeyBindings("{")).toEqual({});
    expect(parseKeyBindings('{"version":2,"bindings":{"kill":["q"]}}')).toEqual({});
    expect(parseKeyBindings('{"version":1,"bindings":{"kill":"q","open":["p"]}}')).toEqual({});
  });
});
