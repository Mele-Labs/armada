// The chords the capture window takes before its page sees them: ⌥⌘C, the
// registry's, and ⌥⌘A, the annotation chord a person already presses on Bridge.
// Left to the page, ⌥⌘A opened a walked mock's own layer, which wrote its note
// into a worktree nothing reads.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

const { isCaptureBinding } = await import("./window");

const press = (code: string, over: Partial<Electron.Input> = {}): Electron.Input =>
  ({ type: "keyDown", code, meta: true, alt: true, control: false, shift: false, ...over }) as Electron.Input;

describe("the capture window's chords", () => {
  it("takes ⌥⌘C and ⌥⌘A", () => {
    expect(isCaptureBinding(press("KeyC"))).toBe(true);
    expect(isCaptureBinding(press("KeyA"))).toBe(true);
  });

  it("leaves everything else to the page", () => {
    expect(isCaptureBinding(press("KeyA", { shift: true }))).toBe(false);
    expect(isCaptureBinding(press("KeyA", { alt: false }))).toBe(false);
    expect(isCaptureBinding(press("KeyB"))).toBe(false);
    expect(isCaptureBinding(press("KeyA", { type: "keyUp" }))).toBe(false);
  });
});
