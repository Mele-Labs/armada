// Which keys a walk's page or bar hands back to Bridge's own window, and how: the claim is that the owner can still jump to another Job, open
// the palette and close a layer while a pane's page holds the keyboard, and that a page keeps what it needs to edit its text.

import { describe, expect, it, vi } from "vitest";

import { forwardTo, isMainKey, keyCodeOf, modifiersOf, type PaneInput } from "./pane-keys";

const press = (code: string, key: string, held: Partial<PaneInput> = {}): PaneInput => ({
  type: "keyDown",
  key,
  code,
  meta: false,
  alt: false,
  control: false,
  shift: false,
  ...held,
});
const cmd = (code: string, key: string, more: Partial<PaneInput> = {}) =>
  press(code, key, {
    meta: true,
    ...more,
  });

describe("which presses are Bridge's own window's", () => {
  it("hands over Escape alone, and ⌘ with anything that is not editing or the system's", () => {
    expect(isMainKey(press("Escape", "Escape"))).toBe(true);
    expect(isMainKey(cmd("KeyK", "k"))).toBe(true);
    expect(isMainKey(cmd("Digit1", "1"))).toBe(true);
    expect(
      isMainKey(
        cmd("ArrowDown", "ArrowDown", {
          alt: true,
        }),
      ),
    ).toBe(true);
    expect(
      isMainKey(
        cmd("BracketLeft", "[", {
          shift: true,
        }),
      ),
    ).toBe(true);
  });

  it("leaves a page its editing keys, redo included", () => {
    for (const code of [
      "KeyC",
      "KeyV",
      "KeyX",
      "KeyA",
      "KeyZ",
    ])
      expect(isMainKey(cmd(code, code.slice(3).toLowerCase()))).toBe(false);
    expect(
      isMainKey(
        cmd("KeyZ", "z", {
          shift: true,
        }),
      ),
    ).toBe(false);
  });

  it("leaves the system its keys, so the menu still gets them", () => {
    for (const code of [
      "KeyQ",
      "KeyW",
      "KeyM",
      "KeyH",
      "Comma",
      "Backquote",
      "Tab",
      "Space",
    ])
      expect(isMainKey(cmd(code, "x"))).toBe(false);
  });

  it("hands over nothing a page's text field uses without ⌘, a key coming up, or Escape with a modifier", () => {
    expect(isMainKey(press("KeyK", "k"))).toBe(false);
    expect(isMainKey(press("Enter", "Enter"))).toBe(false);
    expect(
      isMainKey(
        press("KeyK", "k", {
          control: true,
        }),
      ),
    ).toBe(false);
    expect(
      isMainKey(
        cmd("KeyK", "k", {
          control: true,
        }),
      ),
    ).toBe(false);
    expect(
      isMainKey({
        ...cmd("KeyK", "k"),
        type: "keyUp",
      }),
    ).toBe(false);
    expect(
      isMainKey(
        press("Escape", "Escape", {
          shift: true,
        }),
      ),
    ).toBe(false);
  });
});

describe("how a press is handed on", () => {
  it("names a letter or digit by its place on the keyboard, whatever ⌥ and ⇧ made of it", () => {
    expect(keyCodeOf(cmd("KeyK", "k"))).toBe("K");
    expect(
      keyCodeOf(
        cmd("KeyK", "˚", {
          alt: true,
        }),
      ),
    ).toBe("K");
    expect(
      keyCodeOf(
        cmd("Digit1", "!", {
          shift: true,
        }),
      ),
    ).toBe("1");
  });

  it("names the other keys as Electron does", () => {
    expect(keyCodeOf(cmd("ArrowDown", "ArrowDown"))).toBe("Down");
    expect(keyCodeOf(cmd("ArrowLeft", "ArrowLeft"))).toBe("Left");
    expect(keyCodeOf(press("Escape", "Escape"))).toBe("Escape");
    expect(keyCodeOf(cmd("Slash", "/"))).toBe("/");
  });

  it("lists the modifiers held and no others", () => {
    expect(modifiersOf(cmd("KeyK", "k"))).toEqual([
      "meta",
    ]);
    expect(
      modifiersOf(
        cmd("KeyK", "k", {
          alt: true,
          shift: true,
        }),
      ),
    ).toEqual([
      "meta",
      "alt",
      "shift",
    ]);
    expect(modifiersOf(press("Escape", "Escape"))).toEqual([]);
  });

  it("sends the window the press down and then up, and says it did", () => {
    const sendInputEvent = vi.fn();
    const contents = {
      isDestroyed: () => false,
      sendInputEvent,
    };
    expect(forwardTo(contents, cmd("KeyK", "k"))).toBe(true);
    expect(sendInputEvent.mock.calls).toEqual([
      [
        {
          type: "keyDown",
          keyCode: "K",
          modifiers: [
            "meta",
          ],
        },
      ],
      [
        {
          type: "keyUp",
          keyCode: "K",
          modifiers: [
            "meta",
          ],
        },
      ],
    ]);
  });

  it("sends nothing, and says so, for a press that is the page's, a window that is gone or none", () => {
    const sendInputEvent = vi.fn();
    const live = {
      isDestroyed: () => false,
      sendInputEvent,
    };
    expect(forwardTo(live, cmd("KeyC", "c"))).toBe(false);
    expect(
      forwardTo(
        {
          ...live,
          isDestroyed: () => true,
        },
        cmd("KeyK", "k"),
      ),
    ).toBe(false);
    expect(forwardTo(null, cmd("KeyK", "k"))).toBe(false);
    expect(sendInputEvent).not.toHaveBeenCalled();
  });
});
