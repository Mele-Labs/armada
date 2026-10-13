// How wide a step or task card has to be for its whole name to sit on one line.
//
// **Measured, not guessed.** The name is run through a canvas context set to
// the font the card draws it in, read off the same tokens the stylesheet reads.
// Where there is no canvas to ask (a unit test under jsdom), a per-character
// estimate stands in, generous enough not to clip. The width is a number here
// because React Flow places by number and a `var()` cannot reach it.

import type { WorkflowStepCardProps } from "@armada/components";

/**
 * What a card spends beside its name: `--space-3` of padding each side, the
 * border, and the 16px mark with its `--space-2` gap — and, on a task the
 * Commander has a hand on, that mark's 12px glyph and its gap.
 */
const CHROME = 2 * 12 + 2 + 16 + 8;
const BY_MARK = 8 + 12;
/** Rounded up to the four-unit grid, with a little over for sub-pixel rounding. */
const GRID = 4;
const SLACK = 2;

/** A card's token width, the least it is drawn at: `--w-workflow-node` and `--w-workflow-task-node`. */
const FLOOR = { step: 260, task: 196 } as const;

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

/** The width of `text` as a card's name draws it: sans at `--text-sm`, or mono at `--text-xs` for an identifier. */
export function nameWidth(text: string, identifier: boolean): number {
  const size = Number.parseFloat(token(identifier ? "--text-xs" : "--text-sm", identifier ? "14px" : "15px"));
  const rule = ruler();
  if (rule !== null) {
    const family = token(identifier ? "--font-mono" : "--font-sans", identifier ? "monospace" : "sans-serif");
    rule.font = `${token("--weight-medium", "500")} ${size}px ${family}`;
    const measured = rule.measureText(text).width;
    if (measured > 0) return measured;
  }
  return text.length * size * (identifier ? EM.mono : EM.sans);
}

/** The width one card needs for its name on a line, never less than its kind's token width. */
export function fitOf(card: WorkflowStepCardProps): number {
  const kind = card.kind === "task" ? "task" : "step";
  const wanted =
    nameWidth(card.name, card.nameIsAnIdentifier === true) + CHROME + (card.by === undefined ? 0 : BY_MARK) + SLACK;
  return Math.max(FLOOR[kind], Math.ceil(wanted / GRID) * GRID);
}

/**
 * The cards, each given the column's width: **the widest of them**. One edge
 * down the column, and an edge that leaves a card's middle (the spine's) lands
 * on the same line for every card in it.
 */
export function inOneColumn<T extends { card: WorkflowStepCardProps }>(nodes: readonly T[]): T[] {
  const width = nodes.reduce((widest, node) => Math.max(widest, fitOf(node.card)), 0);
  return nodes.map((node) => ({ ...node, card: { ...node.card, width } }));
}
