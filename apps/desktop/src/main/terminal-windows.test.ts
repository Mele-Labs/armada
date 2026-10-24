import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ BrowserWindow: class {} }));

import { TERMINAL_WINDOW_SIZE, TerminalWindows, type MakeTerminalWindow, type TerminalWindowTold } from "./terminal-windows";

/** Every window the fake factory made, with what it was told and what it did. */
type Fake = { sessionId: string; bounds: Parameters<MakeTerminalWindow>[1]; told: TerminalWindowTold; focused: number; closed: boolean };

function windows(over: { x: number; y: number; width: number; height: number } | null = { x: 100, y: 50, width: 1300, height: 860 }) {
  const made: Fake[] = [];
  const make: MakeTerminalWindow = (sessionId, bounds, told) => {
    const fake: Fake = { sessionId, bounds, told, focused: 0, closed: false };
    made.push(fake);
    return {
      focus: () => void (fake.focused += 1),
      close: () => {
        fake.closed = true;
        told.closed();
      },
    };
  };
  const held = new TerminalWindows(make, () => over);
  const heard: (readonly string[])[] = [];
  held.onChanged((ids) => heard.push(ids));
  return { held, made, heard };
}

describe("a Job's terminal window", () => {
  it("opens one window per Job, over Bridge's, and a second open brings it to the front", () => {
    const { held, made, heard } = windows();
    held.show("a");
    held.show("a");
    expect(made).toHaveLength(1);
    expect(made[0]?.focused).toBe(1);
    expect(made[0]?.bounds).toEqual({ ...TERMINAL_WINDOW_SIZE, x: 100 + (1300 - 900) / 2, y: 50 + (860 - 560) / 2 });
    held.show("b");
    expect(made.map((one) => one.sessionId)).toEqual(["a", "b"]);
    expect(heard).toEqual([["a"], ["a", "b"]]);
    expect(held.has("b")).toBe(true);
  });

  it("tells every listener when a window closes, the person's close or a move back", () => {
    const { held, made, heard } = windows();
    held.show("a");
    held.show("b");
    made[0]?.told.closed();
    held.close("b");
    expect(made[1]?.closed).toBe(true);
    expect(heard.slice(2)).toEqual([["b"], []]);
    expect(held.sessionIds()).toEqual([]);
    held.close("nobody");
    expect(heard).toHaveLength(4);
  });

  it("opens every Job's window at the size the person last gave one, and opens it again after a close", () => {
    const { held, made } = windows(null);
    held.show("a");
    expect(made[0]?.bounds).toEqual(TERMINAL_WINDOW_SIZE);
    made[0]?.told.resized({ width: 1000, height: 700 });
    made[0]?.told.closed();
    held.show("b");
    held.show("a");
    expect(made.map((one) => [one.sessionId, one.bounds])).toEqual([
      ["a", TERMINAL_WINDOW_SIZE],
      ["b", { width: 1000, height: 700 }],
      ["a", { width: 1000, height: 700 }],
    ]);
  });
});
