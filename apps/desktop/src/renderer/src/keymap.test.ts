// The keymap in `@armada/components`: what a press means once a person has rebound keys, read the way
// every handler in Bridge now asks. In node: nothing here needs a document.

import { afterEach, describe, expect, it } from "vitest";
import {
  ACTIONS,
  KEYMAP_GROUPS,
  actsInGroup,
  chordOf,
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

  it("lets the deck's `t` stand beside a call's reply key, each where it listens, and neither is the other's conflict", () => {
    const timeline = ACTIONS.find((one) => one.id === "deck_timeline")!;
    const reply = ACTIONS.find((one) => one.id === "call_reply")!;
    expect(timeline.unbuilt).toBeNull();
    expect(timeline.shortcut).toBe(reply.shortcut);
    expect(conflictsOf("deck_timeline", defaultSlotsOf("deck_timeline")[0]!)).toEqual([]);
    expect(conflictsOf("call_reply", defaultSlotsOf("call_reply")[0]!).map((one) => one.id)).not.toContain("deck_timeline");
  });
});

describe("a press, as the shipped keys answer it", () => {
  it("takes Control for ⌘, reads shift exactly, and Option by the key's place", () => {
    expect(isPressed("command_palette", press("k", { meta: true }))).toBe(true);
    expect(isPressed("command_palette", press("k", { ctrl: true }))).toBe(true);
    expect(isPressed("command_palette", press("k"))).toBe(false);
    expect(isPressed("refresh", press("r", { meta: true, shift: true }))).toBe(true);
    expect(isPressed("refresh", press("r", { meta: true }))).toBe(false);
    expect(isPressed("capture_note", press("ç", { meta: true, alt: true, code: "KeyC" }))).toBe(true);
    expect(isPressed("key_sheet", press("?", { shift: true }))).toBe(true);
    expect(pressedSlot("history", press("]", { meta: true }))).toBe(1);
    expect(pressedDigit("deck_tabs", press("3", { meta: true }))).toBe(3);
    expect(pressedDigit("deck_tabs", press("8", { meta: true }))).toBe(8);
    expect(pressedDigit("deck_tabs", press("9", { meta: true }))).toBeNull();
  });

  it("queues a Session on ⇧⌘Enter, and leaves ⌘Enter to send and plain Enter to start", () => {
    expect(isPressed("queue_session", press("Enter", { meta: true, shift: true }))).toBe(true);
    expect(isPressed("queue_session", press("Enter", { ctrl: true, shift: true }))).toBe(true);
    expect(isPressed("queue_session", press("Enter", { meta: true }))).toBe(false);
    expect(isPressed("queue_session", press("Enter"))).toBe(false);
    expect(isPressed("send_message", press("Enter", { meta: true, shift: true }))).toBe(false);
  });
});

describe("the keys on a Job's thread cards", () => {
  it("answers ⌥↵, ⌥E, ⌥R and ⌥X with yes, change, send back and no", () => {
    expect(isPressed("thread_yes", press("Enter", { alt: true }))).toBe(true);
    expect(isPressed("thread_change", press("e", { alt: true }))).toBe(true);
    expect(isPressed("thread_send_back", press("®", { alt: true, code: "KeyR" }))).toBe(true);
    expect(isPressed("thread_no", press("≈", { alt: true, code: "KeyX" }))).toBe(true);
    expect(isPressed("thread_yes", press("Enter"))).toBe(false);
    expect(isPressed("thread_no", press("x"))).toBe(false);
  });

  it("reads a dead key pressed with Option by its place, so ⌥E on a US Mac is ⌥E", () => {
    expect(chordOf(press("Dead", { alt: true, code: "KeyE" }))).toEqual({ ctrl: false, alt: true, meta: false, shift: false, key: "e" });
    expect(isPressed("thread_change", press("Dead", { alt: true, code: "KeyE" }))).toBe(true);
    expect(isPressed("thread_change", press("Dead", { alt: true, code: "KeyU" }))).toBe(false);
  });

  it("reads a dead key as no chord without Option, or with no place to read it by", () => {
    expect(chordOf(press("Dead", { code: "KeyE" }))).toBeNull();
    expect(chordOf(press("Dead", { alt: true }))).toBeNull();
    expect(chordOf(press("Dead", { alt: true, code: "Space" }))).toBeNull();
  });

  it("lists the four and the hand off band's r and d under Job thread", () => {
    const group = KEYMAP_GROUPS.find((one) => one.title === "Job thread");
    expect(actsInGroup(group!).map((act) => act.id)).toEqual(["thread_yes", "thread_change", "thread_send_back", "thread_no", "handoff_retry", "handoff_dispatch"]);
    expect(isPressed("handoff_retry", press("r"))).toBe(true);
    expect(isPressed("handoff_dispatch", press("d"))).toBe(true);
    expect(isPressed("handoff_retry", press("r", { meta: true }))).toBe(false);
  });
});

describe("a key a person rebound", () => {
  it("answers the new key, not the old one, and draws it", () => {
    rebind("call_later", [parseSlots("q")[0]!]);
    expect(isPressed("call_later", press("q"))).toBe(true);
    expect(isPressed("call_later", press("l"))).toBe(false);
    expect(keyFor("call_later")).toBe("q");
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
    rebind("deck_tabs", parseSlots("⌃1–⌃5"));
    expect(pressedDigit("deck_tabs", press("2", { ctrl: true }))).toBe(2);
    expect(pressedDigit("deck_tabs", press("2", { meta: true }))).toBeNull();
    expect(digitKeyFor("deck_tabs", 2)).toBe("⌃2");
  });

  it("unbinds a key, and binding the shipped key back takes the change away", () => {
    rebind("call_later", [{ kind: "chord", chord: null }]);
    expect(isPressed("call_later", press("l"))).toBe(false);
    expect(keyFor("call_later")).toBe("");
    rebind("call_later", parseSlots("l"));
    expect(serializeKeyBindings({})).toBe("");
    expect(keyFor("call_later")).toBe("l");
  });

  it("names the act it now shares a key with", () => {
    rebind("call_later", parseSlots("d"));
    expect(conflictsOf("call_later", slotsOf("call_later")[0]!).map((one) => one.id)).toEqual(["call_dismiss"]);
    // `t` is a call's reply and the deck's timeline,
    // never in front at once, so never a conflict.
    expect(conflictsOf("deck_timeline", slotsOf("deck_timeline")[0]!)).toEqual([]);
  });

  it("lets Keep and Drop on a reported bug stand beside the Issues tab's own keys, where the report holds focus", () => {
    // `D` is Describe on the tab and Drop on a report: the report's scope is its own place, so neither names the other.
    expect(keyFor("issues_drop")).toBe(keyFor("issues_describe"));
    expect(conflictsOf("issues_drop", slotsOf("issues_drop")[0]!)).toEqual([]);
    expect(conflictsOf("issues_describe", slotsOf("issues_describe")[0]!)).toEqual([]);
    expect(keyFor("issues_keep")).toBe("K");
    // Pressed, each answers as its own act, and Settings → Keyboard lists them under Issues.
    expect(isPressed("issues_drop", press("D"))).toBe(true);
    expect(isPressed("issues_keep", press("K"))).toBe(true);
    const issues = KEYMAP_GROUPS.find((group) => group.title === "Issues")!;
    expect(actsInGroup(issues).map((one) => one.id)).toEqual(expect.arrayContaining(["issues_keep", "issues_drop", "issues_describe", "issues_start"]));
  });

  it("hands every change to the saver as the text Fleet keeps, and empty once all are reset", () => {
    const saved: string[] = [];
    setKeyBindingSaver((text) => saved.push(text));
    rebind("call_later", parseSlots("q"));
    expect(parseKeyBindings(saved[0]!)).toEqual({ call_later: ["q"] });
    resetAllBindings();
    expect(saved[1]).toBe("");
  });
});

describe("the text Fleet keeps", () => {
  it("reads only the shape Fleet accepts, and none of a wrong one", () => {
    expect(parseKeyBindings('{"version":1,"bindings":{"call_later":["q"]}}')).toEqual({ call_later: ["q"] });
    expect(parseKeyBindings("")).toEqual({});
    expect(parseKeyBindings("{")).toEqual({});
    expect(parseKeyBindings('{"version":2,"bindings":{"call_later":["q"]}}')).toEqual({});
    expect(parseKeyBindings('{"version":1,"bindings":{"call_later":"q","open":["p"]}}')).toEqual({});
  });
});
