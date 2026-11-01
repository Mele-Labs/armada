// A note left on Bridge's own UI, in development, for a coding agent to act on.
// #1226.
//
// **Not on `window.armada`.** That surface is Fleet's seam, and the fake of it
// that runs the renderer in a browser (#1223) must not have to answer this. The
// layer reaches its files through `window.armadaDev` inside Electron, and over
// HTTP from a vite dev server in a browser — both write the same files, in the
// shape below.
//
// Nothing here touches the DOM or Node, because main, the preload, the renderer
// and a vite plugin all read it.

import { isRing, type Ring } from "./ring";

/** Where the notes land, from the repository root. One JSON file per note. */
export const ANNOTATIONS_DIR = [".armada", "annotations"] as const;

/** The path a vite dev server answers on, for the renderer in a browser. */
export const ANNOTATIONS_PATH = "/__armada/annotations";

/**
 * The attribute a development build stamps each element's own JSX onto, as
 * `<path>:<line>` from the repository root. `annotations-source.ts` writes it
 * and the layer reads it at pin time. Here because both ends of that are
 * elsewhere, and neither may own the spelling alone.
 */
export const SOURCE_ATTRIBUTE = "data-armada-source";

/**
 * The argument main passes the window when the app is not packaged. The
 * preload exposes `armadaDev` only when it sees it, so a packaged build has no
 * handler, no preload entry and no reason to load the layer's chunk.
 */
export const ANNOTATE_FLAG = "--armada-annotate";

export const ANNOTATION_CHANNELS = {
  list: "annotations:list",
  save: "annotations:save",
  remove: "annotations:remove",
  root: "annotations:root",
  capture: "annotations:capture",
  saveShot: "annotations:save-shot",
  shot: "annotations:shot",
  saveExample: "annotations:save-example",
  example: "annotations:example",
  removeExample: "annotations:remove-example",
  /** Main to the window: Fleet rewrote a note file, so the layer reads them again. */
  changed: "annotations:changed",
} as const;

/** A note's picture is `<id>.png`, beside its `<id>.json`. */
export const shotName = (id: string): string => `${id}.png`;

/** A picture the person added to a note to show what he means: `<id>-example-<n>.png`, beside its `<id>.json`. */
export const exampleName = (id: string, n: number): string => `${id}-example-${n}.png`;

/** Whether `name` is one of note `id`'s example pictures, and so a file name that cannot leave the directory. */
export function isExampleOf(id: unknown, name: unknown): name is string {
  if (!isAnnotationId(id) || typeof name !== "string") return false;
  const prefix = `${id}-example-`;
  return name.startsWith(prefix) && /^[0-9]{1,3}\.png$/.test(name.slice(prefix.length));
}

/** Larger than a whole Retina window's PNG; a body past it is not a capture. */
export const SHOT_LIMIT = 32 * 1024 * 1024;

const PNG_SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];

/** Whether these bytes start as a PNG does and fit under `SHOT_LIMIT`. Main and the vite plugin write nothing else. */
export function isPng(bytes: Uint8Array): boolean {
  return bytes.length > PNG_SIGNATURE.length && bytes.length <= SHOT_LIMIT && PNG_SIGNATURE.every((byte, at) => bytes[at] === byte);
}

export type AnnotationStatus = "open" | "done";

export type Box = { x: number; y: number; width: number; height: number };

/** Where a note was sent, #1250: the Session it went to as a message. */
export type Sent = { at: string; sessionId: string; title: string };

/** What a sent note is called where it went: its Session's title. */
export const sentAs = (sent: Sent): string => sent.title;

/** What a whole-window note carries about what Bridge held as it was taken, for debugging that state. */
export type AppContext = {
  /** Bridge's connection to Fleet, as its `state` word. */
  connection: string;
  /** Doctor's read: its one-line summary and each module's result. */
  doctor: { summary: string; rows: { module: string; result: string; detail: string }[] };
  /** The Session whose page was open, as Fleet last said it. */
  session?: { id: string; title?: string; state: string; harness: string; cwd: string; endReason?: string };
};

/** Where an element's JSX is: a path from the repository root, and a line. #1584. */
export type Source = { file: string; line: number };

/** Which of the note card's tools made a change. */
export type ChangeKind = "size" | "colour" | "fill" | "spacing" | "text";

export const CHANGE_KINDS: readonly ChangeKind[] = ["size", "colour", "fill", "spacing", "text"];

/**
 * One edit the person made to the element on the page, to show what he wants. `property` is the CSS
 * property, or `text` for its words. `before` is what the page drew before it, as computed; `after` what
 * he picked, as computed; `token` the design token he picked it as, where he picked one.
 */
export type StyleChange = { kind: ChangeKind; property: string; before: string; after: string; token?: string };

/**
 * One note. Written for the agent that reads it, so each field is something a
 * search for the code can start from.
 */
export type Annotation = {
  /** Also the file name, less `.json`. Sorts by when it was written. */
  id: string;
  status: AnnotationStatus;
  /**
   * `window` where the note is on the whole window rather than one element: its picture is the
   * window, and `selector`, `element` and the component chain are the page's body. Absent otherwise.
   */
  scope?: "window";
  /** Where it went, once sent to Fleet. Absent until then. */
  sent?: Sent;
  /** What the person said. */
  text: string;
  /** The innermost React component under the click, or null if none was found. */
  component: string | null;
  /**
   * Where the JSX that drew the element is. Written by the build, read off the
   * element — **absent, never guessed**, where nothing above it was stamped:
   * the mock stamps and Bridge under `pnpm dev` does not.
   */
  source?: Source;
  /** The components above it, nearest first. */
  owners: string[];
  /**
   * Which chain `owners` is. `owner` is who rendered it, which React keeps only
   * in a development build; `parent` is the tree it sits in, which every build
   * has. `pnpm dev` runs a production React, so Bridge under it says `parent`.
   */
  ownersFrom: "owner" | "parent";
  /** A CSS selector the layer found the element by, and finds it by again. */
  selector: string;
  element: { tag: string; text: string; label: string | null };
  /** The rail item marked current when the note was written, or null. */
  screen: string | null;
  /** The open dialog or sheet the element sits in, by its accessible name. */
  layer: string | null;
  /** The named places around the element and the nearest heading above it, innermost first. Absent on a note written before they were read. */
  nearby?: string[];
  /** Path, query and hash of the page. */
  location: string;
  /** `?scenario=` in the mock (#1223), or null. */
  scenario: string | null;
  /** What the page marked selected, current, open or pressed, each `role "name"`. Absent on a note written before it was read. */
  selected?: string[];
  /** What Bridge held as a whole-window note was taken. Absent on a note on one element. */
  context?: AppContext;
  /** The element's box in the window, in CSS pixels. */
  box: Box;
  window: { width: number; height: number };
  /**
   * The picture of the window taken as the note was saved, the layer left out of it: always
   * `shotName(id)`, beside this file. **Absent** on a note written before pictures were kept, and
   * where nothing could capture one, as in a browser.
   */
  screenshot?: string;
  /**
   * Where the element was on `screenshot`, in its pixels: ringed on it, with the note's number, as it is sent. Absent on a
   * whole-window note, and on one whose picture was of the element alone, from before pictures were of the window.
   */
  ring?: Ring;
  /** What the person changed on the element with the card's tools, one per property, in the order first made. Absent where none. */
  changes?: StyleChange[];
  /** Pictures he added to show what he means, each `exampleName(id, n)` beside this file. Absent where none. */
  examples?: string[];
  createdAt: string;
  updatedAt: string;
};

/** What `window.armadaDev` carries: operations on one note and its picture, and two a Send needs. */
export type AnnotationsDevApi = {
  list: () => Promise<Annotation[]>;
  save: (note: Annotation) => Promise<void>;
  remove: (id: string) => Promise<void>;
  /** The repository the notes are about, which is the one a sent note's Job belongs to. */
  root: () => Promise<string | null>;
  /** A PNG of this part of the window, or null where nothing can capture it. */
  capture: (box: Box) => Promise<ArrayBuffer | null>;
  /** Keeps a PNG as the note's picture, `shotName(id)`. */
  saveShot: (id: string, png: ArrayBuffer) => Promise<void>;
  /** The note's picture, or null where it has none. */
  shot: (id: string) => Promise<ArrayBuffer | null>;
  /** Keeps a PNG as one of the note's example pictures, `name` being `exampleName(id, n)`. */
  saveExample: (id: string, name: string, png: ArrayBuffer) => Promise<void>;
  /** One of the note's example pictures, or null where it is gone. */
  example: (id: string, name: string) => Promise<ArrayBuffer | null>;
  removeExample: (id: string, name: string) => Promise<void>;
  /**
   * Calls `listener` each time Fleet rewrites a note file: an agent marked one done, or a Job's close
   * marked its notes done or let them go. Returns its remover. Absent in a browser, where nothing tells.
   */
  onChanged?: (listener: () => void) => () => void;
};

/** An id is a file name, so it is held to what cannot leave the directory. */
export function isAnnotationId(value: unknown): value is string {
  return typeof value === "string" && /^[A-Za-z0-9_-]{1,80}$/.test(value);
}

/** `20260916-222105-k3f9`: sortable, readable in `ls`, and unique enough by hand. */
export function annotationId(at: Date, random: () => number = Math.random): string {
  const iso = at.toISOString();
  const stamp = `${iso.slice(0, 10).replaceAll("-", "")}-${iso.slice(11, 19).replaceAll(":", "")}`;
  const suffix = Math.floor(random() * 36 ** 4)
    .toString(36)
    .padStart(4, "0");
  return `${stamp}-${suffix}`;
}

const isString = (v: unknown): v is string => typeof v === "string";
const isNumber = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
const isNullableString = (v: unknown): v is string | null => v === null || isString(v);
const isRecord = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);

function isSource(v: unknown): v is Source {
  return isRecord(v) && isString(v["file"]) && isNumber(v["line"]);
}

function isSent(v: unknown): v is Sent {
  if (!isRecord(v) || !isString(v["at"])) return false;
  return isString(v["sessionId"]) && isString(v["title"]);
}

function isContext(v: unknown): v is AppContext {
  if (!isRecord(v) || !isString(v["connection"]) || !isRecord(v["doctor"])) return false;
  const doctor = v["doctor"];
  return (
    isString(doctor["summary"]) &&
    Array.isArray(doctor["rows"]) &&
    doctor["rows"].every((row) => isRecord(row) && isString(row["module"]) && isString(row["result"]) && isString(row["detail"])) &&
    (v["session"] === undefined || (isRecord(v["session"]) && isString(v["session"]["id"]) && isString(v["session"]["state"])))
  );
}

function isBox(v: unknown): v is Box {
  return isRecord(v) && isNumber(v["x"]) && isNumber(v["y"]) && isNumber(v["width"]) && isNumber(v["height"]);
}

function isStyleChange(v: unknown): v is StyleChange {
  return (
    isRecord(v) &&
    CHANGE_KINDS.includes(v["kind"] as ChangeKind) &&
    isString(v["property"]) &&
    isString(v["before"]) &&
    isString(v["after"]) &&
    (v["token"] === undefined || isString(v["token"]))
  );
}

/**
 * Whether a value read off the wire or the disk is a note. Main and the vite
 * plugin both refuse anything else, so a malformed body writes nothing.
 */
export function isAnnotation(value: unknown): value is Annotation {
  if (!isRecord(value)) return false;
  const element = value["element"];
  const window = value["window"];
  return (
    isAnnotationId(value["id"]) &&
    (value["status"] === "open" || value["status"] === "done") &&
    (value["scope"] === undefined || value["scope"] === "window") &&
    (value["sent"] === undefined || isSent(value["sent"])) &&
    isString(value["text"]) &&
    isNullableString(value["component"]) &&
    (value["source"] === undefined || isSource(value["source"])) &&
    Array.isArray(value["owners"]) &&
    value["owners"].every(isString) &&
    (value["ownersFrom"] === "owner" || value["ownersFrom"] === "parent") &&
    isString(value["selector"]) &&
    isRecord(element) &&
    isString(element["tag"]) &&
    isString(element["text"]) &&
    isNullableString(element["label"]) &&
    isNullableString(value["screen"]) &&
    isNullableString(value["layer"]) &&
    (value["nearby"] === undefined || (Array.isArray(value["nearby"]) && value["nearby"].every(isString))) &&
    isString(value["location"]) &&
    isNullableString(value["scenario"]) &&
    (value["selected"] === undefined || (Array.isArray(value["selected"]) && value["selected"].every(isString))) &&
    (value["context"] === undefined || isContext(value["context"])) &&
    isBox(value["box"]) &&
    isRecord(window) &&
    isNumber(window["width"]) &&
    isNumber(window["height"]) &&
    (value["screenshot"] === undefined || value["screenshot"] === shotName(value["id"] as string)) &&
    (value["ring"] === undefined || isRing(value["ring"])) &&
    (value["changes"] === undefined || (Array.isArray(value["changes"]) && value["changes"].every(isStyleChange))) &&
    (value["examples"] === undefined || (Array.isArray(value["examples"]) && value["examples"].every((name) => isExampleOf(value["id"], name)))) &&
    isString(value["createdAt"]) &&
    isString(value["updatedAt"])
  );
}

/**
 * The file's contents. Keys in the order the type declares them, so the first
 * lines an agent reads are the note and where it points.
 */
export function serializeAnnotation(note: Annotation): string {
  const ordered: Annotation = {
    id: note.id,
    status: note.status,
    ...(note.scope === undefined ? {} : { scope: note.scope }),
    ...(note.sent === undefined ? {} : { sent: note.sent }),
    text: note.text,
    component: note.component,
    ...(note.source === undefined ? {} : { source: note.source }),
    owners: note.owners,
    ownersFrom: note.ownersFrom,
    selector: note.selector,
    element: note.element,
    screen: note.screen,
    layer: note.layer,
    ...(note.nearby === undefined ? {} : { nearby: note.nearby }),
    location: note.location,
    scenario: note.scenario,
    ...(note.selected === undefined ? {} : { selected: note.selected }),
    ...(note.context === undefined ? {} : { context: note.context }),
    box: note.box,
    window: note.window,
    ...(note.screenshot === undefined ? {} : { screenshot: note.screenshot }),
    ...(note.ring === undefined ? {} : { ring: note.ring }),
    ...(note.changes === undefined ? {} : { changes: note.changes }),
    ...(note.examples === undefined ? {} : { examples: note.examples }),
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
  };
  return `${JSON.stringify(ordered, null, 2)}\n`;
}

/** Oldest first, which is the order they were written in and the pins' numbers. */
export function byCreation(a: Annotation, b: Annotation): number {
  return a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : a.id < b.id ? -1 : 1;
}

/**
 * What a sent note asks Fleet for, as the describe-the-work path reads a
 * request: the person's words first, then where in Bridge they point, so the
 * Drone can find the code without the person there to ask.
 */
export function requestOf(note: Annotation): string {
  if (note.scope === "window") return windowRequestOf(note);
  const components = [note.component, ...note.owners].filter((name): name is string => name !== null);
  const lines = [
    note.text,
    "",
    "This is feedback on Bridge's own UI, left with the annotation layer.",
    `- Component: ${components.length > 0 ? components.slice(0, 6).join(" inside ") : note.element.tag}`,
    `- Element: <${note.element.tag}>${note.element.text === "" ? "" : ` reading "${note.element.text}"`}`,
    `- Selector: ${note.selector}`,
  ];
  if (note.source !== undefined) lines.push(`- Source: ${note.source.file}:${note.source.line}`);
  if (note.screen !== null) lines.push(`- Screen: ${note.screen}`);
  if (note.layer !== null) lines.push(`- Inside: ${note.layer}`);
  if (note.scenario !== null) lines.push(`- Mock scenario: ${note.scenario}`);
  lines.push(`- Window: ${note.window.width}×${note.window.height}`, `- Note file: .armada/annotations/${note.id}.json`);
  const changed = changeLines(note);
  if (changed.length > 0) lines.push("", ...changed);
  return lines.join("\n");
}

const quoted = (change: StyleChange, value: string): string => (change.kind === "text" ? JSON.stringify(value) : value === "" ? "none" : value);

/**
 * What the person changed on the element with the card's tools, to show what he wants: a heading naming
 * the selector, then a line per property, before → after and the token he picked. None where he changed nothing.
 */
export function changeLines(note: Pick<Annotation, "selector" | "changes">): string[] {
  const changes = note.changes ?? [];
  if (changes.length === 0) return [];
  return [
    `Changes he made on the page to show what he wants, on ${note.selector}:`,
    ...changes.map((change) => `- ${change.property}: ${quoted(change, change.before)} → ${quoted(change, change.after)}${change.token === undefined ? "" : ` (${change.token})`}`),
  ];
}

/** What a whole-window note records of the window. A failure sent to a Session records the same, so both write it with `windowLines`. */
export type WindowFacts = Pick<Annotation, "screen" | "layer" | "location" | "scenario" | "window" | "context">;

/** A Session as a context records it. */
export function sessionFacts(session: { id: string; title?: string; state: string; harness: string; cwd: string; end_reason?: string }): NonNullable<AppContext["session"]> {
  return {
    id: session.id,
    ...(session.title === undefined ? {} : { title: session.title }),
    state: session.state,
    harness: session.harness,
    cwd: session.cwd,
    ...(session.end_reason === undefined ? {} : { endReason: session.end_reason }),
  };
}

/** The line stating an open Session, labelled by whose it is. */
export function sessionLine(label: string, session: NonNullable<AppContext["session"]>): string {
  const facts = [`state ${session.state}`, `harness ${session.harness}`, `cwd ${session.cwd}`, session.endReason === undefined ? null : `ended: ${session.endReason}`];
  return `- ${label}: ${session.title === undefined ? "" : `"${session.title}" `}(${session.id}) — ${facts.filter((f) => f !== null).join(", ")}`;
}

/** The page and the state Bridge held, as labelled lines. */
export function windowLines(note: WindowFacts): string[] {
  const lines = [`- Screen: ${note.screen ?? "none marked"}`];
  if (note.layer !== null) lines.push(`- Inside: ${note.layer}`);
  lines.push(`- Location: ${note.location}`);
  if (note.scenario !== null) lines.push(`- Mock scenario: ${note.scenario}`);
  lines.push(`- Window: ${note.window.width}×${note.window.height}`);
  const context = note.context;
  if (context !== undefined) {
    const { session, doctor } = context;
    if (session !== undefined) lines.push(sessionLine("Job open", session));
    lines.push(`- Connection to Fleet: ${context.connection}`, `- Doctor: ${doctor.summary}`);
    for (const row of doctor.rows) lines.push(`  - ${row.module}: ${row.result}${row.detail === "" ? "" : ` — ${row.detail}`}`);
  }
  return lines;
}

/** A whole-window note's request: what the person said, the page, and the state Bridge held. */
function windowRequestOf(note: Annotation): string {
  const lines = [note.text, "", "This is feedback on the state of Bridge's whole window, left with the annotation layer. The screenshot is the window."];
  lines.push(...windowLines(note), `- Note file: .armada/annotations/${note.id}.json`);
  return lines.join("\n");
}
