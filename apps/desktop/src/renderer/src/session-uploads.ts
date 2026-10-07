// What a Session's message carries beyond its words, as Fleet takes it: a name, a media type and
// base64. A picked file is read as it is; a drawn sketch is sent as the picture it renders to, which
// is what a model can look at. Where the window has no canvas to draw on, the sketch goes as the
// words in its boxes and the joins between them, so a sketch is never silently dropped.

import type { SessionUpload } from "@armada/protocol";
import type { DrawnSketch, SentFile, SessionSketch } from "@armada/screens/src/draft/sessions";

/** Base64 of bytes, in slices so a large picture does not overflow the argument list. */
export function base64Of(bytes: Uint8Array): string {
  let text = "";
  for (let at = 0; at < bytes.length; at += 0x8000) text += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  return btoa(text);
}

/** A picked file as an upload, or nothing for a chip that never had its file. */
export async function uploadOfFile(sent: SentFile): Promise<SessionUpload | undefined> {
  if (sent.file === undefined) return undefined;
  return {
    name: sent.name,
    media_type: sent.file.type === "" ? "application/octet-stream" : sent.file.type,
    data: base64Of(new Uint8Array(await sent.file.arrayBuffer())),
  };
}

const BOX_WIDTH = 160;
const LINE = 16;
const PAD = 12;
const MARGIN = 24;

/** The words wrapped to a box's width, by measuring. */
function wrapped(context: CanvasRenderingContext2D, body: string): string[] {
  const lines: string[] = [];
  let line = "";
  for (const word of body.split(/\s+/).filter((one) => one !== "")) {
    const next = line === "" ? word : `${line} ${word}`;
    if (line !== "" && context.measureText(next).width > BOX_WIDTH - PAD * 2) {
      lines.push(line);
      line = word;
    } else line = next;
  }
  return line === "" ? lines : [...lines, line];
}

/** The sketch as words: each box, and what it joins. What a window with no canvas sends. */
export function describeSketch(drawing: SessionSketch): string {
  const said = new Map(drawing.boxes.map((box) => [box.id, box.body.trim() === "" ? "(empty box)" : box.body.trim()]));
  return [
    ...drawing.boxes.map((box) => `Box: ${said.get(box.id)}`),
    ...drawing.lines.map((join) => `Join: ${said.get(join.from) ?? join.from} -> ${said.get(join.to) ?? join.to}`),
    ...((drawing.strokes ?? []).length === 0 ? [] : [`${drawing.strokes!.length} lines drawn by hand`]),
  ].join("\n");
}

/** The sketch drawn to a PNG, or `undefined` where this window cannot draw one. */
async function pictureOf(drawing: SessionSketch): Promise<Uint8Array | undefined> {
  const probe = document.createElement("canvas").getContext("2d");
  if (probe === null) return undefined;
  probe.font = "small sans-serif";
  const boxes = drawing.boxes.map((box) => {
    const lines = wrapped(probe, box.body);
    return { ...box, lines, height: PAD * 2 + Math.max(lines.length, 1) * LINE };
  });
  const points = [
    ...boxes.flatMap((box) => [{ x: box.x, y: box.y }, { x: box.x + BOX_WIDTH, y: box.y + box.height }]),
    ...(drawing.strokes ?? []).flatMap((stroke) => stroke.points),
  ];
  if (points.length === 0) return undefined;
  const left = Math.min(...points.map((one) => one.x)) - MARGIN;
  const top = Math.min(...points.map((one) => one.y)) - MARGIN;
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil(Math.max(...points.map((one) => one.x)) + MARGIN - left);
  canvas.height = Math.ceil(Math.max(...points.map((one) => one.y)) + MARGIN - top);
  const context = canvas.getContext("2d");
  if (context === null) return undefined;
  context.fillStyle = "white";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.font = "small sans-serif";
  context.strokeStyle = "black";
  context.fillStyle = "black";
  context.lineWidth = 1.5;
  const centre = (id: string) => {
    const box = boxes.find((one) => one.id === id);
    return box === undefined ? undefined : { x: box.x + BOX_WIDTH / 2 - left, y: box.y + box.height / 2 - top };
  };
  for (const join of drawing.lines) {
    const from = centre(join.from);
    const to = centre(join.to);
    if (from === undefined || to === undefined) continue;
    context.beginPath();
    context.moveTo(from.x, from.y);
    context.lineTo(to.x, to.y);
    context.stroke();
  }
  for (const stroke of drawing.strokes ?? []) {
    context.beginPath();
    stroke.points.forEach((point, at) => (at === 0 ? context.moveTo(point.x - left, point.y - top) : context.lineTo(point.x - left, point.y - top)));
    context.stroke();
  }
  for (const box of boxes) {
    context.fillStyle = "white";
    context.fillRect(box.x - left, box.y - top, BOX_WIDTH, box.height);
    context.strokeRect(box.x - left, box.y - top, BOX_WIDTH, box.height);
    context.fillStyle = "black";
    box.lines.forEach((line, at) => context.fillText(line, box.x - left + PAD, box.y - top + PAD + (at + 1) * LINE - 4));
  }
  const blob = await new Promise<Blob | null>((done) => canvas.toBlob(done, "image/png"));
  return blob === null ? undefined : new Uint8Array(await blob.arrayBuffer());
}

/** A drawn sketch as an upload: its picture, or its words where there is nothing to draw on. */
export async function uploadOfSketch(sketch: DrawnSketch): Promise<SessionUpload> {
  const picture = await pictureOf(sketch.drawing).catch(() => undefined);
  const name = sketch.title.replace(/[^\w .-]/g, "_");
  return picture === undefined
    ? { name: `${name}.txt`, media_type: "text/plain", data: base64Of(new TextEncoder().encode(describeSketch(sketch.drawing))) }
    : { name: `${name}.png`, media_type: "image/png", data: base64Of(picture) };
}
