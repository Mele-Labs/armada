/** A part of a picture, as fractions of its width and height. */
export type NoteRegion = { x: number; y: number; w: number; h: number };

/**
 * What a note points at, as each viewer reports it. Lines are 1-based and inclusive. A crop is a PNG `data:` address. A
 * sketch is named by its title, the key the agent drew it under, and its parts by the scene's own ids. A picture's `path`
 * is where it is read from, where the viewer knows it.
 */
export type NoteAbout =
  | { kind: "sketch"; title: string; parts: readonly { id: string; label: string }[] }
  | { kind: "file"; path: string; from: number; to: number; quote: string }
  | { kind: "image"; name: string; path?: string; region?: NoteRegion; crop: string }
  | { kind: "document"; place: DocumentPlace; target: DocumentTarget };

/** Where in a Job's document a note stands: the page's title, the revision read, and the heading of the section. */
export type DocumentPlace = { page: string; revision: number; section: string };

/** What in that section a note points at: words, the parts of a sketch the page embeds, or a row of one of its tables. */
export type DocumentTarget =
  | { kind: "words"; quote: string }
  | { kind: "part"; sketch: string; parts: readonly { id: string; label: string }[] }
  | { kind: "row"; table: string; row: string };

export type HeldNote = { about: NoteAbout; text: string };

/** A picture that goes with the message, named as the text names it. */
export type NotePicture = { name: string; png: string };

export type NotesMessage = { text: string; pictures: NotePicture[] };

/** What a sent note's heading names, read back out of the message: enough to open it again. */
export type NotedRef =
  | { kind: "sketch"; title: string; parts: readonly { id: string; label: string }[] }
  | { kind: "file"; path: string; from: number; to: number }
  | { kind: "image"; path: string; region?: NoteRegion }
  | { kind: "document"; place: DocumentPlace; target: DocumentTarget };

/** One note of a sent message: its number, what it names, and the lines under its heading. */
export type NotedBlock = { n: number; ref: NotedRef; body: string };

/** The most lines of a file a note quotes. */
const QUOTED = 6;

const lines = (from: number, to: number) => (from === to ? String(from) : `${String(from)}-${String(to)}`);

const pct = (n: number) => Math.round(n * 100);

/** A word of the page set on one line, with no quote mark in it, so the heading reads back the way it was written. */
export const onOneLine = (words: string): string => words.replace(/\s+/g, " ").replace(/"/g, "'").trim();

function documentHeading(place: DocumentPlace, target: DocumentTarget): string {
  const at = `Document "${onOneLine(place.page)}", revision ${String(place.revision)}, section "${onOneLine(place.section)}": `;
  if (target.kind === "words") return `${at}words "${onOneLine(target.quote)}"`;
  if (target.kind === "row") return `${at}row "${onOneLine(target.row)}" of "${onOneLine(target.table)}"`;
  return `${at}sketch "${onOneLine(target.sketch)}", ${target.parts.length === 1 ? "part" : "parts"} ${target.parts.map((one) => `${one.id} (${one.label})`).join(", ")}`;
}

function heading(about: NoteAbout, picture: string | undefined): string {
  if (about.kind === "document") return documentHeading(about.place, about.target);
  if (about.kind === "sketch") return `Sketch "${about.title}", ${about.parts.length === 1 ? "part" : "parts"} ${about.parts.map((one) => `${one.id} (${one.label})`).join(", ")}`;
  if (about.kind === "file") return `File ${about.path}:${lines(about.from, about.to)}`;
  const r = about.region;
  const region = r === undefined ? "" : `, region x ${String(pct(r.x))}-${String(pct(r.x + r.w))}% y ${String(pct(r.y))}-${String(pct(r.y + r.h))}%`;
  return `Picture ${about.path ?? about.name}${region}${picture === undefined ? "" : `, attached as ${picture}`}`;
}

function quoted(about: NoteAbout): string[] {
  if (about.kind !== "file") return [];
  const all = about.quote.replace(/\n+$/, "").split("\n");
  const shown = all.slice(0, QUOTED).map((line) => `> ${line}`.trimEnd());
  return all.length > QUOTED ? [...shown, "> …"] : shown;
}

/**
 * Held notes as one message: a numbered block per note, what it is about on the number's line, a file's lines quoted under
 * it, then the note. **Plain text the agent reads as it stands**: a sketch by its title and its parts' ids, a file by its
 * path and lines, a picture by where it is read from, the region in percent, and the file it is attached as.
 */
export function notesMessage(notes: readonly HeldNote[]): NotesMessage {
  const pictures: NotePicture[] = [];
  const blocks = notes.map((note, at) => {
    const n = at + 1;
    let picture: string | undefined;
    if (note.about.kind === "image" && note.about.crop !== "") {
      picture = `picture-${String(pictures.length + 1)}.png`;
      pictures.push({ name: picture, png: note.about.crop });
    }
    const indent = " ".repeat(String(n).length + 2);
    const body = [...quoted(note.about), ...note.text.trim().split("\n")].map((line) => (line === "" ? "" : indent + line));
    return [`${String(n)}. ${heading(note.about, picture)}`, ...body].join("\n");
  });
  return { text: blocks.join("\n\n"), pictures };
}

const SKETCH = /^Sketch "(.*)", parts? (.+)$/;
const PART = /([^\s(),]+) \((.*?)\)(?:, |$)/g;
const FILE = /^File (.+):(\d+)(?:-(\d+))?$/;
const PICTURE = /^Picture (.+?)(?:, region x (\d+)-(\d+)% y (\d+)-(\d+)%)?(?:, attached as picture-\d+\.png)?$/;
const DOCUMENT = /^Document "([^"]*)", revision (\d+), section "([^"]*)": (.+)$/;
const DOC_WORDS = /^words "(.*)"$/;
const DOC_ROW = /^row "(.*)" of "(.*)"$/;
const DOC_SKETCH = /^sketch "(.*)", parts? (.+)$/;
const HEAD = /^(\d+)\. (Sketch|File|Picture|Document) /;

function documentOf(said: string): NotedRef | undefined {
  const found = DOCUMENT.exec(said);
  if (found === null) return undefined;
  const place = { page: found[1]!, revision: Number(found[2]), section: found[3]! };
  const words = DOC_WORDS.exec(found[4]!);
  if (words !== null) return { kind: "document", place, target: { kind: "words", quote: words[1]! } };
  const row = DOC_ROW.exec(found[4]!);
  if (row !== null) return { kind: "document", place, target: { kind: "row", row: row[1]!, table: row[2]! } };
  const sketch = DOC_SKETCH.exec(found[4]!);
  if (sketch === null) return undefined;
  const parts = [...sketch[2]!.matchAll(PART)].map((one) => ({ id: one[1]!, label: one[2]! }));
  return parts.length === 0 ? undefined : { kind: "document", place, target: { kind: "part", sketch: sketch[1]!, parts } };
}

function refOf(said: string): NotedRef | undefined {
  if (said.startsWith("Document ")) return documentOf(said);
  const sketch = SKETCH.exec(said);
  if (sketch !== null) {
    const parts = [...sketch[2]!.matchAll(PART)].map((one) => ({ id: one[1]!, label: one[2]! }));
    return parts.length === 0 ? undefined : { kind: "sketch", title: sketch[1]!, parts };
  }
  const file = FILE.exec(said);
  if (file !== null) return { kind: "file", path: file[1]!, from: Number(file[2]), to: Number(file[3] ?? file[2]) };
  const picture = PICTURE.exec(said);
  if (picture === null) return undefined;
  const [x0, x1, y0, y1] = [picture[2], picture[3], picture[4], picture[5]].map((one) => (one === undefined ? undefined : Number(one) / 100));
  const region = x0 === undefined || x1 === undefined || y0 === undefined || y1 === undefined ? undefined : { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
  return { kind: "image", path: picture[1]!, ...(region === undefined ? {} : { region }) };
}

/** A message `notesMessage` wrote, read back into its notes. Anything else, even one line of it, is not one: `undefined`. */
export function notedOf(text: string): NotedBlock[] | undefined {
  const blocks: { n: number; ref: NotedRef; body: string[]; indent: number }[] = [];
  for (const line of text.split("\n")) {
    const head = HEAD.exec(line);
    if (head !== null) {
      const ref = refOf(line.slice(head[1]!.length + 2));
      if (ref === undefined || Number(head[1]) !== blocks.length + 1) return undefined;
      blocks.push({ n: Number(head[1]), ref, body: [], indent: head[1]!.length + 2 });
      continue;
    }
    const last = blocks[blocks.length - 1];
    if (last === undefined) return undefined;
    if (line.trim() === "") last.body.push("");
    else if (line.startsWith(" ".repeat(last.indent))) last.body.push(line.slice(last.indent));
    else return undefined;
  }
  return blocks.length === 0 ? undefined : blocks.map(({ n, ref, body }) => ({ n, ref, body: body.join("\n").trim() }));
}

const cut = (words: string, at: number) => (words.length > at ? `${words.slice(0, at).trimEnd()}…` : words);

/** What a document note points at, short: the words, the parts' names, or the row's first cell. */
function targetSaid(target: DocumentTarget): string {
  if (target.kind === "words") return cut(onOneLine(target.quote), 40);
  if (target.kind === "row") return cut(onOneLine(target.row.split(" | ")[0] ?? target.row), 40);
  return target.parts.map((one) => one.label).join(", ");
}

/** What a note is about, short, for the note box and a mark's tooltip. */
export function aboutSaid(about: NoteAbout): string {
  if (about.kind === "document") return `${about.place.section}: ${targetSaid(about.target)}`;
  if (about.kind === "sketch") return about.parts.map((one) => one.label).join(", ");
  if (about.kind === "file") return `${about.path.split("/").pop() ?? about.path}:${lines(about.from, about.to)}`;
  return about.region === undefined ? about.name : `${about.name}, region`;
}

/**
 * A sent note's heading as the thread shows it, short: a sketch's title and its parts' names, a file from the Job's checkout
 * `root` where it is in it and from the home folder elsewhere, a picture by its name. The message keeps the whole path.
 */
export function notedSaid(ref: NotedRef, root?: string): string {
  if (ref.kind === "document") return `${ref.place.section}: ${targetSaid(ref.target)}`;
  if (ref.kind === "sketch") return `${ref.title}: ${ref.parts.map((one) => one.label).join(", ")}`;
  if (ref.kind === "file") return `${shortPath(ref.path, root)}:${lines(ref.from, ref.to)}`;
  const name = ref.path.split("/").pop() ?? ref.path;
  return ref.region === undefined ? name : `${name}, region`;
}

/** A path from `root` where it is in it, else from the home folder as `~`. */
export function shortPath(path: string, root?: string): string {
  const base = root?.replace(/\/+$/, "");
  if (base !== undefined && base !== "" && path.startsWith(`${base}/`)) return path.slice(base.length + 1);
  return path.replace(/^\/(Users|home)\/[^/]+(?=\/)/, "~");
}
