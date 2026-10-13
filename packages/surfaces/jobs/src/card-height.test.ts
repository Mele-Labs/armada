// How many lines a card's name takes, which is what the canvas places by.
// Arithmetic, so it runs in node: `linesIn` takes the width of a string as an
// argument and these use one px per character; `nameLines` has no canvas to ask
// here and falls back to its per-character estimate, so its figures are the
// estimate's (a task's room is 144px, 124px beside the Commander's mark).

import { describe, expect, it } from "vitest";

import type { WorkflowStepCardProps } from "@armada/components";

import { heightBeyond, linesIn, nameLines } from "./card-height";

const ONE_PX = (text: string) => text.length;

function task(name: string, extra: Partial<WorkflowStepCardProps> = {}): WorkflowStepCardProps {
  return { kind: "task", name, activity: "not_started", said: "not started", ...extra };
}

describe("linesIn counts the lines a name takes in a room", () => {
  it("takes one line for a title that fits", () => {
    expect(linesIn("Fix the parser", 20, ONE_PX)).toBe(1);
    expect(linesIn("Fix the parser", 14, ONE_PX)).toBe(1);
  });

  it("wraps a long title onto several lines, at its spaces", () => {
    expect(linesIn("aaaa bbbb cccc dddd", 9, ONE_PX)).toBe(2);
    expect(linesIn("aaaa bbbb cccc dddd eeee", 9, ONE_PX)).toBe(3);
    expect(linesIn("aaaa bbbb cccc dddd eeee", 4, ONE_PX)).toBe(5);
  });

  it("breaks inside a single word wider than a line", () => {
    // 25 characters in 10: ten, ten, five.
    expect(linesIn("abcdefghijklmnopqrstuvwxy", 10, ONE_PX)).toBe(3);
    // The word is cut where it overflows, and what follows it starts on the line that is left.
    expect(linesIn("ab abcdefghijklmnopqrst", 10, ONE_PX)).toBe(3);
  });
});

describe("nameLines counts a card's name at the card's width", () => {
  it("takes one line for a short title and several for a long one", () => {
    expect(nameLines(task("Fix the parser"))).toBe(1);
    expect(nameLines(task("Rewrite the store's retry policy so a stalled write is not repeated forever"))).toBeGreaterThan(2);
  });

  it("gives a step more room than a task", () => {
    const name = "Review the change before it lands";
    expect(nameLines({ ...task(name), kind: "step" })).toBeLessThan(nameLines(task(name)));
  });

  it("loses room to the Commander's mark, which can cost a line", () => {
    // Fifteen characters: one line in a task's 144px, two in the 124px the mark leaves.
    const name = "Rewrite parsers";
    expect(nameLines(task(name))).toBe(1);
    expect(nameLines(task(name, { by: { who: "commander" } }))).toBe(2);
  });
});

describe("heightBeyond is the height over a base number of lines", () => {
  it("is nothing at the base, a line's leading for each line more, and negative for fewer", () => {
    const one = task("Fix the parser");
    const many = task("Rewrite the store's retry policy so a stalled write is not repeated");
    expect(heightBeyond(one, 1)).toBe(0);
    expect(heightBeyond(one, 2)).toBeLessThan(0);
    expect(heightBeyond(many, 1)).toBe((nameLines(many) - 1) * 22);
  });

  it("counts an identifier's lines at the shorter leading", () => {
    const id = task("a_very_long_step_identifier_that_wraps", { nameIsAnIdentifier: true });
    expect(heightBeyond(id, 1)).toBe((nameLines(id) - 1) * 20);
  });
});
