// The sketch as a PNG: what a Drone reads when a request carries a picture.
//
// **Written, and not staged.** This turns a `Drawing` into PNG bytes and does
// nothing else; putting them on disk and on the wire is #1545's, with the
// schema lock. Until that lands nothing calls it — the owner's call of
// 1 Oct 2026, taking the cost of code that waits.
//
// **The pad is the reference.** A box is the card the pad draws — its fill,
// the well inside it and the words in the well — a join is React Flow's bezier
// between the two facing sides with the closed arrow at its end, a stroke is
// the pen's line curved through its midpoints, and a picture is drawn at its
// own place and size. Every colour and size is read off the tokens the pad is
// painted with, so the picture a Drone reads is the one a person drew.

import { facingSides } from "@armada/components";

import type { Drawing, SketchPicture, SketchPoint, SketchShape } from "./sketch";

/**
 * The pad's paint, resolved. **Read off the tokens by `sketchInkOf`** rather
 * than written here, so a token changing changes the PNG with the pad.
 */
export type SketchInk = {
  canvas: string;
  boxFill: string;
  well: string;
  wellEdge: string;
  words: string;
  join: string;
  pen: string;
  font: string;
  textSize: number;
  leading: number;
  boxWidth: number;
  /** The box's padding, and the well's. */
  inset: number;
  hairline: number;
  penWidth: number;
  radius: number;
  /** How long a join's arrowhead is. */
  arrow: number;
  /** Room round the picture's outermost edge. */
  margin: number;
};

/** Which token each part of the ink is, by name. The one place the two are joined. */
const TOKENS = {
  canvas: "--surface-canvas",
  boxFill: "--bg-raised",
  well: "--bg-sunken",
  wellEdge: "--border-default",
  words: "--fg-default",
  join: "--border-strong",
  pen: "--fg-muted",
  font: "--font-sans",
  textSize: "--text-sm",
  leading: "--leading-sm",
  boxWidth: "--w-sketch-box",
  inset: "--space-2",
  hairline: "--border-width",
  penWidth: "--edge-active",
  radius: "--radius-md",
  arrow: "--space-1",
  margin: "--space-6",
} as const satisfies Record<keyof SketchInk, string>;

/** The ink, read off a computed style — the document's, wherever the tokens are loaded. */
export function sketchInkOf(style: CSSStyleDeclaration): SketchInk {
  const read = (name: string) => style.getPropertyValue(name).trim();
  const size = (name: string) => Number.parseFloat(read(name));
  return {
    canvas: read(TOKENS.canvas),
    boxFill: read(TOKENS.boxFill),
    well: read(TOKENS.well),
    wellEdge: read(TOKENS.wellEdge),
    words: read(TOKENS.words),
    join: read(TOKENS.join),
    pen: read(TOKENS.pen),
    font: read(TOKENS.font),
    textSize: size(TOKENS.textSize),
    leading: size(TOKENS.leading),
    boxWidth: size(TOKENS.boxWidth),
    inset: size(TOKENS.inset),
    hairline: size(TOKENS.hairline),
    penWidth: size(TOKENS.penWidth),
    radius: size(TOKENS.radius),
    arrow: size(TOKENS.arrow),
    margin: size(TOKENS.margin),
  };
}

/** The PNG, and its size in pixels — the two halves `StagedFrame` carries beside a path. */
export type SketchPng = { bytes: Uint8Array<ArrayBuffer>; width: number; height: number };

/** How many rows a box's well shows when it holds less — the pad's `rows={2}`. */
const WELL_ROWS = 2;

/** React Flow's own bend for a bezier edge. */
const CURVATURE = 0.25;

type Rect = { x: number; y: number; width: number; height: number };
type Side = "left" | "right" | "top" | "bottom";
type Pen = OffscreenCanvasRenderingContext2D;

/** The words broken into lines that fit `room`, the way the well wraps them. */
function wrapped(pen: Pen, body: string, room: number): string[] {
  const lines: string[] = [];
  for (const paragraph of body.split("\n")) {
    let line = "";
    for (const word of paragraph.split(/\s+/).filter((one) => one !== "")) {
      const tried = line === "" ? word : `${line} ${word}`;
      if (pen.measureText(tried).width <= room || line === "") {
        line = tried;
      } else {
        lines.push(line);
        line = word;
      }
    }
    lines.push(line);
  }
  return lines;
}

/** Where a box sits and how tall its words make it. */
function laidOut(pen: Pen, ink: SketchInk, shape: SketchShape) {
  const wellWidth = ink.boxWidth - ink.inset * 2;
  const room = wellWidth - ink.inset * 2 - ink.hairline * 2;
  const lines = shape.body.trim() === "" ? [] : wrapped(pen, shape.body, room);
  const wellHeight =
    Math.max(WELL_ROWS, lines.length) * ink.leading + ink.inset * 2 + ink.hairline * 2;
  const rect: Rect = { x: shape.x, y: shape.y, width: ink.boxWidth, height: wellHeight + ink.inset * 2 };
  return { rect, lines, well: { x: shape.x + ink.inset, y: shape.y + ink.inset, width: wellWidth, height: wellHeight } };
}

function middleOf(rect: Rect, side: Side): SketchPoint {
  switch (side) {
    case "left":
      return { x: rect.x, y: rect.y + rect.height / 2 };
    case "right":
      return { x: rect.x + rect.width, y: rect.y + rect.height / 2 };
    case "top":
      return { x: rect.x + rect.width / 2, y: rect.y };
    case "bottom":
      return { x: rect.x + rect.width / 2, y: rect.y + rect.height };
  }
}

/** React Flow's `calculateControlOffset`, so the curve bends as the pad's does. */
function offset(distance: number): number {
  return distance >= 0 ? 0.5 * distance : CURVATURE * 25 * Math.sqrt(-distance);
}

function control(from: SketchPoint, side: Side, to: SketchPoint): SketchPoint {
  switch (side) {
    case "left":
      return { x: from.x - offset(from.x - to.x), y: from.y };
    case "right":
      return { x: from.x + offset(to.x - from.x), y: from.y };
    case "top":
      return { x: from.x, y: from.y - offset(from.y - to.y) };
    case "bottom":
      return { x: from.x, y: from.y + offset(to.y - from.y) };
  }
}

/** The handle id `facingSides` answers, as the side it names. */
function sideOf(handle: string): Side {
  return handle.slice(2) as Side;
}

/** One join: the curve, then the closed arrow at its end. */
function joinPath(from: Rect, to: Rect) {
  const sides = facingSides(
    { position: from, measured: from },
    { position: to, measured: to },
  );
  const out = sideOf(sides.sourceHandle);
  const into = sideOf(sides.targetHandle);
  const start = middleOf(from, out);
  const end = middleOf(to, into);
  return { start, end, c1: control(start, out, end), c2: control(end, into, start) };
}

/** The pen's own curve: through the midpoints, as `Ink`'s `pathOf` draws it. */
function strokeAlong(pen: Pen, points: readonly SketchPoint[]): void {
  const first = points[0];
  if (first === undefined) return;
  pen.beginPath();
  pen.moveTo(first.x, first.y);
  for (let at = 1; at < points.length - 1; at += 1) {
    const here = points[at]!;
    const next = points[at + 1]!;
    pen.quadraticCurveTo(here.x, here.y, (here.x + next.x) / 2, (here.y + next.y) / 2);
  }
  const last = points[points.length - 1]!;
  if (points.length > 1) pen.lineTo(last.x, last.y);
  pen.stroke();
}

function rounded(pen: Pen, rect: Rect, radius: number): void {
  pen.beginPath();
  pen.roundRect(rect.x, rect.y, rect.width, rect.height, radius);
}

/** A picture's bytes, decoded. Through an `<img>`, because `img-src` is what admits `blob:`. */
async function decoded(picture: SketchPicture): Promise<HTMLImageElement> {
  const image = new Image();
  image.src = picture.src;
  await image.decode();
  return image;
}

/**
 * The drawing as a PNG, `scale` pixels to each unit of the pad.
 *
 * **Framed on what is drawn**, with the margin round it: the pad's own
 * coordinates are negative as often as not, so the PNG starts at the drawing's
 * top-left rather than at the pad's origin. Painted in the pad's order —
 * joins under the boxes and pictures, ink over everything.
 */
export async function sketchPng(
  drawing: Drawing,
  ink: SketchInk,
  scale = 2,
): Promise<SketchPng> {
  const measure = new OffscreenCanvas(1, 1).getContext("2d");
  if (measure === null) throw new Error("No 2D context to measure the sketch with.");
  const font = `${String(ink.textSize)}px ${ink.font}`;
  if (typeof document !== "undefined") await document.fonts.load(font);
  measure.font = font;

  const boxes = drawing.shapes.map((shape) => ({ id: shape.id, ...laidOut(measure, ink, shape) }));
  const pictures = await Promise.all(
    drawing.pictures.map(async (picture) => ({ picture, image: await decoded(picture) })),
  );
  const placed = new Map<string, Rect>([
    ...boxes.map((box) => [box.id, box.rect] as const),
    ...drawing.pictures.map((one) => [one.id, one as Rect] as const),
  ]);
  const joins = drawing.joins.flatMap((join) => {
    const from = placed.get(join.from);
    const to = placed.get(join.to);
    return from === undefined || to === undefined ? [] : [joinPath(from, to)];
  });

  // Everything that paints, as points, for the frame.
  const corners: SketchPoint[] = [
    ...[...placed.values()].flatMap((rect) => [
      { x: rect.x, y: rect.y },
      { x: rect.x + rect.width, y: rect.y + rect.height },
    ]),
    ...drawing.strokes.flatMap((stroke) => stroke.points),
    ...joins.flatMap((join) => [join.c1, join.c2]),
  ];
  // An empty pad frames the origin, so it is the margin and nothing else.
  const xs = corners.length === 0 ? [0] : corners.map((one) => one.x);
  const ys = corners.length === 0 ? [0] : corners.map((one) => one.y);
  const framed = { left: Math.min(...xs) - ink.margin, top: Math.min(...ys) - ink.margin };
  const width = Math.ceil((Math.max(...xs) + ink.margin - framed.left) * scale);
  const height = Math.ceil((Math.max(...ys) + ink.margin - framed.top) * scale);
  const canvas = new OffscreenCanvas(width, height);
  const pen = canvas.getContext("2d");
  if (pen === null) throw new Error("No 2D context to draw the sketch with.");

  pen.fillStyle = ink.canvas;
  pen.fillRect(0, 0, width, height);
  pen.scale(scale, scale);
  pen.translate(-framed.left, -framed.top);

  // Joins first, so a box sits over the end of a line rather than under it.
  pen.strokeStyle = ink.join;
  pen.fillStyle = ink.join;
  pen.lineWidth = ink.hairline;
  for (const join of joins) {
    pen.beginPath();
    pen.moveTo(join.start.x, join.start.y);
    pen.bezierCurveTo(join.c1.x, join.c1.y, join.c2.x, join.c2.y, join.end.x, join.end.y);
    pen.stroke();
    const angle = Math.atan2(join.end.y - join.c2.y, join.end.x - join.c2.x);
    const back = (turn: number) => ({
      x: join.end.x - ink.arrow * Math.cos(angle + turn),
      y: join.end.y - ink.arrow * Math.sin(angle + turn),
    });
    const one = back(Math.PI / 5);
    const two = back(-Math.PI / 5);
    pen.beginPath();
    pen.moveTo(join.end.x, join.end.y);
    pen.lineTo(one.x, one.y);
    pen.lineTo(two.x, two.y);
    pen.closePath();
    pen.fill();
  }

  pen.font = font;
  pen.textBaseline = "middle";
  for (const box of boxes) {
    pen.fillStyle = ink.boxFill;
    rounded(pen, box.rect, ink.radius);
    pen.fill();
    pen.fillStyle = ink.well;
    pen.strokeStyle = ink.wellEdge;
    pen.lineWidth = ink.hairline;
    rounded(pen, box.well, ink.radius);
    pen.fill();
    pen.stroke();
    pen.fillStyle = ink.words;
    box.lines.forEach((line, at) => {
      pen.fillText(
        line,
        box.well.x + ink.hairline + ink.inset,
        box.well.y + ink.hairline + ink.inset + ink.leading * (at + 0.5),
      );
    });
  }

  for (const { picture, image } of pictures) {
    pen.save();
    rounded(pen, picture, ink.radius);
    pen.clip();
    pen.drawImage(image, picture.x, picture.y, picture.width, picture.height);
    pen.restore();
  }

  pen.strokeStyle = ink.pen;
  pen.lineWidth = ink.penWidth;
  pen.lineCap = "round";
  pen.lineJoin = "round";
  for (const stroke of drawing.strokes) strokeAlong(pen, stroke.points);

  const png = await canvas.convertToBlob({ type: "image/png" });
  return { bytes: new Uint8Array(await png.arrayBuffer()), width, height };
}
