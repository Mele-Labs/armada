import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({
  BrowserWindow: class {},
}));

import {
  TERMINAL_WINDOW_SIZE,
  TerminalWindows,
  type MakeTerminalWindow,
  type TerminalWindowTold,
} from "./terminal-windows";
import type { TerminalWindowId } from "../shared/api/sessions";

/** Every window the fake factory made, with what it was told and what it did. */
type Fake = {
  id: TerminalWindowId;
  bounds: Parameters<MakeTerminalWindow>[1];
  told: TerminalWindowTold;
  focused: number;
  closed: boolean;
};

function windows(
  over: {
    x: number;
    y: number;
    width: number;
    height: number;
  } | null = {
    x: 100,
    y: 50,
    width: 1300,
    height: 860,
  },
) {
  const made: Fake[] = [];
  const make: MakeTerminalWindow = (id, bounds, told) => {
    const fake: Fake = {
      id,
      bounds,
      told,
      focused: 0,
      closed: false,
    };
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
  const heard: (readonly TerminalWindowId[])[] = [];
  held.onChanged((ids) => heard.push(ids));
  return {
    held,
    made,
    heard,
  };
}

const A: TerminalWindowId = {
  sessionId: "a",
};
const B: TerminalWindowId = {
  sessionId: "b",
};

describe("a terminal's window", () => {
  it("opens one window per terminal, over Bridge's, and a second open brings it to the front", () => {
    const { held, made, heard } = windows();
    held.show(A);
    held.show(A);
    expect(made).toHaveLength(1);
    expect(made[0]?.focused).toBe(1);
    expect(made[0]?.bounds).toEqual({
      ...TERMINAL_WINDOW_SIZE,
      x: 100 + (1300 - 900) / 2,
      y: 50 + (860 - 560) / 2,
    });
    held.show(B);
    expect(made.map((one) => one.id)).toEqual([
      A,
      B,
    ]);
    expect(heard).toEqual([
      [
        A,
      ],
      [
        A,
        B,
      ],
    ]);
    expect(held.has(B)).toBe(true);
  });

  it("tells every listener when a window closes, the person's close or a move back", () => {
    const { held, made, heard } = windows();
    held.show(A);
    held.show(B);
    made[0]?.told.closed();
    held.close(B);
    expect(made[1]?.closed).toBe(true);
    expect(heard.slice(2)).toEqual([
      [
        B,
      ],
      [],
    ]);
    expect(held.ids()).toEqual([]);
    held.close({
      sessionId: "nobody",
    });
    expect(heard).toHaveLength(4);
  });

  it("opens a window for each of a Job's terminals, and the first it has no name for is a window of its own", () => {
    const { held, made, heard } = windows();
    const t1 = {
      sessionId: "a",
      terminal: "t1",
    };
    const t2 = {
      sessionId: "a",
      terminal: "t2",
    };
    held.show(t1);
    held.show(t2);
    held.show(t2);
    held.show(A);
    expect(made.map((one) => one.id)).toEqual([
      t1,
      t2,
      A,
    ]);
    expect(made[1]?.focused).toBe(1);
    held.close(t1);
    expect(held.ids()).toEqual([
      t2,
      A,
    ]);
    expect(held.has(t1)).toBe(false);
    expect(heard.at(-1)).toEqual([
      t2,
      A,
    ]);
  });

  it("opens every terminal's window at the size the person last gave one, and opens it again after a close", () => {
    const { held, made } = windows(null);
    held.show(A);
    expect(made[0]?.bounds).toEqual(TERMINAL_WINDOW_SIZE);
    made[0]?.told.resized({
      width: 1000,
      height: 700,
    });
    made[0]?.told.closed();
    held.show(B);
    held.show(A);
    expect(
      made.map((one) => [
        one.id,
        one.bounds,
      ]),
    ).toEqual([
      [
        A,
        TERMINAL_WINDOW_SIZE,
      ],
      [
        B,
        {
          width: 1000,
          height: 700,
        },
      ],
      [
        A,
        {
          width: 1000,
          height: 700,
        },
      ],
    ]);
  });
});
