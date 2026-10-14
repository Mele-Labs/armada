// The two shapes of ink, apart from `Ink.tsx` so a module that only reads a drawing (the scene format)
// needs no JSX to do it.

/** One place on the pad, in the same coordinates a box's `x` and `y` are in. */
export type SketchPoint = { x: number; y: number };

/** One line drawn by hand, kept as the line rather than as a picture of it. */
export type SketchStroke = { id: string; points: readonly SketchPoint[] };
