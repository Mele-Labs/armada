// The layout tree a `screen` node draws: a wireframe of an app's screen, for any product. **An agent says
// what is on the screen and never where**: rows, columns and grids lay it out, and a child's `size`
// shares what is left. Like a scene, a tree arrives as `unknown` and `readScreen` is the only way in:
// all of it or nothing, no repair.

export const SCREEN_ICONS = [
  "arrow-left", "arrow-right", "arrow-up-from-line", "bell", "bookmark", "bot", "check", "chevron-down", "chevron-left", "chevron-right",
  "chevron-up", "circle-check", "circle-help", "clock", "copy", "database", "download", "ellipsis", "external-link", "eye", "file-text", "files",
  "filter", "flag", "folder", "folder-plus", "git-branch", "globe", "history", "house", "image", "key", "layers", "layout-grid", "link",
  "list-checks", "lock", "mail", "menu", "message-square", "monitor", "package", "panel-left-open", "paperclip", "pause", "pencil", "pin", "play",
  "plus", "refresh-cw", "search", "send", "settings", "sliders-horizontal", "smartphone", "terminal", "thumbs-down", "thumbs-up", "ticket",
  "trash-2", "triangle-alert", "user-check", "x",
] as const;
export type ScreenIcon = (typeof SCREEN_ICONS)[number];

export const SCREEN_GAPS = ["none", "sm", "md", "lg"] as const;
export type ScreenGap = (typeof SCREEN_GAPS)[number];
export const SCREEN_ALIGNS = ["start", "center", "end", "stretch"] as const;
export type ScreenAlign = (typeof SCREEN_ALIGNS)[number];
export const SCREEN_CHROMES = ["desktop", "browser", "phone"] as const;
export type ScreenChrome = (typeof SCREEN_CHROMES)[number];
export const SCREEN_VARIANTS = ["primary", "secondary", "ghost"] as const;
export type ScreenVariant = (typeof SCREEN_VARIANTS)[number];
export const SCREEN_FIELDS = ["input", "textarea", "select", "checkbox", "toggle", "search"] as const;
export type ScreenField = (typeof SCREEN_FIELDS)[number];
export const SCREEN_TEXTS = ["heading", "title", "body", "caption"] as const;
export type ScreenText = (typeof SCREEN_TEXTS)[number];

/** `auto` takes its content, `fill` shares what is left, and a number is the weight of its share. */
export type ScreenSize = "auto" | "fill" | number;

/** A button or an item of a bar: a bare string is its words, an object may carry a glyph with or without them. */
export type ScreenAct = string | { text?: string; icon?: ScreenIcon };
export type ScreenRow = { text: string; meta?: string; icon?: ScreenIcon; selected?: boolean };
export type ScreenMessage = { from: "me" | "them"; text: string };

type Base = {
  /** Names the element when it carries one, so a reader and a walk can find it. */
  id?: string;
  /** Joins the scene's step order: the element is ringed while the step plays and the rest of the screen stands back. */
  step?: number;
  size?: ScreenSize;
  /** Where the element sits across its parent, where it is not the default for its kind. */
  align?: ScreenAlign;
};
type Layout = { gap?: ScreenGap; pad?: ScreenGap; border?: boolean; children: ScreenElement[] };

export type ScreenElement = Base &
  (
    | ({ type: "row" | "column" } & Layout)
    | ({ type: "grid"; cols: number } & Layout)
    | { type: "window"; chrome: ScreenChrome; title?: string; url?: string; children: ScreenElement[] }
    | { type: "nav"; brand?: string; items?: ScreenAct[]; active?: string; actions?: ScreenAct[]; vertical?: boolean }
    | { type: "panel"; title?: string; icon?: ScreenIcon; actions?: ScreenAct[]; gap?: ScreenGap; pad?: ScreenGap; children?: ScreenElement[] }
    | { type: "tabs"; items: string[]; active?: string }
    | { type: "list"; rows: ScreenRow[] }
    | { type: "table"; columns: string[]; rows: string[][] }
    | { type: "button"; text?: string; variant?: ScreenVariant; icon?: ScreenIcon }
    | { type: "field"; label?: string; kind?: ScreenField; value?: string; placeholder?: string }
    | { type: "text"; style?: ScreenText; text?: string; lines?: number }
    | { type: "image"; label?: string }
    | { type: "avatar"; text?: string }
    | { type: "badge"; text: string }
    | { type: "divider" }
    | { type: "spacer" }
    | { type: "chat"; messages: ScreenMessage[] }
    | { type: "modal"; title?: string; children: ScreenElement[] }
  );

export type ScreenType = ScreenElement["type"];

const FAIL = Symbol("not a screen");
type Read = (value: unknown) => unknown;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null && !Array.isArray(value);
const text: Read = (value) => (typeof value === "string" ? value : FAIL);
const flag: Read = (value) => (typeof value === "boolean" ? value : FAIL);
const num: Read = (value) => (typeof value === "number" && Number.isFinite(value) ? value : FAIL);
const whole = (least: number, most: number): Read => (value) => (typeof value === "number" && Number.isInteger(value) && value >= least && value <= most ? value : FAIL);
const choice = (list: readonly string[]): Read => (value) => (typeof value === "string" && list.includes(value) ? value : FAIL);
const many = (one: Read): Read => (value) => {
  if (!Array.isArray(value)) return FAIL;
  const out = value.map(one);
  return out.includes(FAIL) ? FAIL : out;
};
const size: Read = (value) => (value === "auto" || value === "fill" ? value : typeof value === "number" && Number.isFinite(value) && value > 0 ? value : FAIL);

/** The fields of one record, and the ones it cannot do without. Anything else on it is dropped. */
const record = (fields: Record<string, Read>, required: readonly string[]): Read => (value) => {
  if (!isRecord(value)) return FAIL;
  const out: Record<string, unknown> = {};
  for (const [key, read] of Object.entries(fields)) {
    if (value[key] === undefined) {
      if (required.includes(key)) return FAIL;
      continue;
    }
    const got = read(value[key]);
    if (got === FAIL) return FAIL;
    out[key] = got;
  }
  return out;
};

const icon = choice(SCREEN_ICONS);
const act: Read = (value) => (typeof value === "string" ? value : record({ text, icon }, [])(value));
const children: Read = (value) => many(element)(value);

const BASE: Record<string, Read> = { id: text, step: num, size, align: choice(SCREEN_ALIGNS) };
const LAYOUT: Record<string, Read> = { gap: choice(SCREEN_GAPS), pad: choice(SCREEN_GAPS), border: flag, children };

const spec = (type: ScreenType, fields: Record<string, Read>, required: readonly string[] = []): [ScreenType, Read] => [
  type,
  record({ type: choice([type]), ...BASE, ...fields }, ["type", ...required]),
];

const SPECS = new Map<string, Read>([
  spec("row", LAYOUT, ["children"]),
  spec("column", LAYOUT, ["children"]),
  spec("grid", { ...LAYOUT, cols: whole(1, 12) }, ["children", "cols"]),
  spec("window", { chrome: choice(SCREEN_CHROMES), title: text, url: text, children }, ["chrome", "children"]),
  spec("nav", { brand: text, items: many(act), active: text, actions: many(act), vertical: flag }),
  spec("panel", { title: text, icon, actions: many(act), gap: choice(SCREEN_GAPS), pad: choice(SCREEN_GAPS), children }),
  spec("tabs", { items: many(text), active: text }, ["items"]),
  spec("list", { rows: many(record({ text, meta: text, icon, selected: flag }, ["text"])) }, ["rows"]),
  spec("table", { columns: many(text), rows: many(many(text)) }, ["columns", "rows"]),
  spec("button", { text, variant: choice(SCREEN_VARIANTS), icon }),
  spec("field", { label: text, kind: choice(SCREEN_FIELDS), value: text, placeholder: text }),
  spec("text", { style: choice(SCREEN_TEXTS), text, lines: whole(1, 12) }),
  spec("image", { label: text }),
  spec("avatar", { text }),
  spec("badge", { text }, ["text"]),
  spec("divider", {}),
  spec("spacer", {}),
  spec("chat", { messages: many(record({ from: choice(["me", "them"]), text }, ["from", "text"])) }, ["messages"]),
  spec("modal", { title: text, children }, ["children"]),
]);

function element(value: unknown): unknown {
  if (!isRecord(value) || typeof value.type !== "string") return FAIL;
  const read = SPECS.get(value.type);
  return read === undefined ? FAIL : read(value);
}

/** A screen's tree, or `undefined` when any part of it is not one. */
export function readScreen(value: unknown): ScreenElement | undefined {
  const got = element(value);
  return got === FAIL ? undefined : (got as ScreenElement);
}

export const childrenOf = (el: ScreenElement): readonly ScreenElement[] => ("children" in el ? (el.children ?? []) : []);

/** Every step number an element or anything under it carries. */
export function screenSteps(el: ScreenElement): number[] {
  return [...(el.step === undefined ? [] : [el.step]), ...childrenOf(el).flatMap(screenSteps)];
}

/** Whether the element, or anything under it, plays at `step`. */
export const holdsStep = (el: ScreenElement, step: number): boolean => el.step === step || childrenOf(el).some((one) => holdsStep(one, step));

/** A phone is as wide as one, and anything else as a desktop window. */
export const screenWidth = (el: ScreenElement): number => (el.type === "window" && el.chrome === "phone" ? 390 : 960);

/** What a screen is taken to measure before the browser has measured it: for placing an arrow's words and for the frame a scene fits to. */
export const screenGuess = (el: ScreenElement, w?: number): { w: number; h: number } => {
  const across = w ?? screenWidth(el);
  return { w: across, h: el.type === "window" && el.chrome === "phone" ? across * 2 : Math.round(across * 0.625) };
};

/** What an element is called to somebody who cannot see it. */
export function screenLabel(el: ScreenElement): string {
  const named = "text" in el ? el.text : undefined;
  const titled = "title" in el ? el.title : undefined;
  const labelled = "label" in el ? el.label : undefined;
  const branded = "brand" in el ? el.brand : undefined;
  const glyph = "icon" in el ? el.icon : undefined;
  return named ?? titled ?? labelled ?? branded ?? glyph ?? el.id ?? el.type;
}
