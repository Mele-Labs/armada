// How many lines a step or task card's name takes at the card's fixed width,
// and so how much taller than a one-line card it stands.
//
// **Measured, not guessed.** The name is broken into lines the way the card's
// stylesheet breaks it — at spaces, and inside a word only where one will not
// fit a line alone (`overflow-wrap: anywhere`) — with each candidate line run
// through a canvas context set to the font the card draws it in, read off the
// same tokens. Where there is no canvas to ask (a unit test under jsdom), a
// per-character estimate stands in. A height is a number here because React
// Flow places by number and a `var()` cannot reach it.

import type { WorkflowStepCardProps } from "@armada/components";
import TOKENS from "@armada/tokens/tokens.json";

/** A card's token width: `--w-workflow-node` and `--w-workflow-task-node`. */
const WIDTH = { step: 260, task: 196 } as const;

/**
 * What a card spends beside its name: `--space-3` of padding each side, the
 * border, and the 16px mark with its `--space-2` gap — and, on a task the
 * Commander has a hand on, that mark's 12px glyph and its gap.
 */
const CHROME = 2 * 12 + 2 + 16 + 8;
const BY_MARK = 8 + 12;
/** Taken off the room, so a measure a hair short of the browser's never loses a line. */
const SLACK = 2;

/** A name's line: `--leading-sm`, or `--leading-xs` for an identifier. */
const LEADING = { name: 22, identifier: 20 } as const;

/** Estimated em per character, for a face with no canvas to measure it. */
const EM = { sans: 0.58, mono: 0.62 } as const;

let context: CanvasRenderingContext2D | null | undefined;

function ruler(): CanvasRenderingContext2D | null {
  if (context !== undefined) return context;
  try {
    context = typeof document === "undefined" ? null : document.createElement("canvas").getContext("2d");
  } catch {
    context = null;
  }
  return context;
}

/** A custom property off the root, or `fallback` where there is no document to read it from. */
function token(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value === "" ? fallback : value;
}

/** A token's shipped value, for where no stylesheet is loaded to read it off. */
function shipped(name: string): string {
  return TOKENS.tokens.find((one) => one.name === name)?.value ?? "";
}

/** A function that gives the width of a string as a card's name draws it. */
function rulerFor(identifier: boolean): (text: string) => number {
  const sizeToken = identifier ? "--text-xs" : "--text-sm";
  const size = Number.parseFloat(token(sizeToken, shipped(sizeToken)));
  const rule = ruler();
  if (rule !== null) {
    const family = token(identifier ? "--font-mono" : "--font-sans", identifier ? "monospace" : "sans-serif");
    rule.font = `${token("--weight-medium", "500")} ${size}px ${family}`;
    if (rule.measureText("m").width > 0) return (text) => rule.measureText(text).width;
  }
  return (text) => text.length * size * (identifier ? EM.mono : EM.sans);
}

/** How many lines `text` takes in `room` px, by `width` of a string. */
export function linesIn(text: string, room: number, width: (text: string) => number): number {
  let lines = 1;
  let line = "";
  for (const word of text.split(" ")) {
    const joined = line === "" ? word : `${line} ${word}`;
    if (width(joined) <= room) {
      line = joined;
      continue;
    }
    // The word goes to a line of its own; one wider than a line is cut where it overflows.
    if (line !== "") lines += 1;
    let rest = word;
    while (width(rest) > room && rest.length > 1) {
      let cut = 1;
      while (cut < rest.length - 1 && width(rest.slice(0, cut + 1)) <= room) cut += 1;
      rest = rest.slice(cut);
      lines += 1;
    }
    line = rest;
  }
  return lines;
}

/** The lines a step or task card's name takes at its token width. */
export function nameLines(card: WorkflowStepCardProps): number {
  const kind = card.kind === "task" ? "task" : "step";
  const room = WIDTH[kind] - CHROME - (card.by === undefined ? 0 : BY_MARK) - SLACK;
  return linesIn(card.name, room, rulerFor(card.nameIsAnIdentifier === true));
}

/** How far taller than a card with `base` lines of name this one stands. Negative where it is shorter. */
export function heightBeyond(card: WorkflowStepCardProps, base: number): number {
  const leading = card.nameIsAnIdentifier === true ? LEADING.identifier : LEADING.name;
  return (nameLines(card) - base) * leading;
}
