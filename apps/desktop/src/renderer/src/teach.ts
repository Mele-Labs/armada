// What the owner taught Armada, from anything that happens in a Job: selected words, a command, a Fleet notice, a plan task, a
// Check, a pull request; or from Bridge itself: a panel, a row, the title bar. A teaching can change **Agents** (a note, which becomes
// a Logbook entry), **Words** (how Bridge shows them: words matching a pattern become links), **Fleet** (what it does with a notice),
// the **Theme** (a tweak to one shared colour, over whichever theme is chosen) and the **Layout** (a panel hidden). Each change but the
// note is a mod of its own kind, so a teaching that changes two makes two. A teaching on Bridge's own screen has no note for agents.
//
// A pattern is taught once and an address is found per repository, because a Jira site is different in each: Fleet finds it (the
// repository's own settings, then its recent pull requests) and asks the person only when it finds none.
//
// **Fleet reads and keeps a teaching** (`readTeaching`, `saveTeaching`), and Bridge draws what Fleet lists: the links from
// `listWordRules`, the tweaks from the mod list, the Mods page from the mod list. What this holds is the window's own: whether Teach
// is offered, the version of the rules it has saved, and the panel's preview of a link or a tweak before it is kept.
//
// **The mock is the one window with lessons.** A mock Fleet holds what it was taught (`lessons`), which the thread's notice rules and
// the Logbook's taught entries read. A real window holds none, and draws everything else from what Fleet's wire carries.

import { createContext, useContext, useMemo, useSyncExternalStore } from "react";
import type { SessionThreadRow, TeachReach, TeachSourceKind } from "@armada/components";
import type { FleetInstead } from "@armada/protocol";
import type { LayoutRegion } from "@armada/shell";

import { useModList } from "./mod-list";
import { modOpened } from "./open-mod";
import { openInPanel } from "./shared-panel/store";

/** Where Teach was pressed: what, and in which Job and repository. */
export type Teaching = {
  kind: TeachSourceKind;
  /** The text as it stands where Teach was pressed. */
  text: string;
  /** When the row was written, for a notice: only later ones are changed by what is taught. */
  at?: string;
  sessionId: string;
  jobTitle: string;
  manifestId?: string;
  /** The repository's root, which the Logbook groups by. */
  root?: string;
  /** The repository's name. */
  repository: string;
};

/** Where an address was found. */
export type SiteFrom = { kind: "settings" } | { kind: "pull_request"; number: number } | { kind: "you" };

/** A repository's site: the address its keys go under. */
export type Site = { address: string; from: SiteFrom };

export type FleetDoes = "batch" | "quiet" | "tell" | "call";

/** A theme tweak: one shared colour, set over whichever theme is chosen. */
export type Tweak = {
  /** The mod that keeps it: a slug named from it. */
  mod: string;
  /** The colour as the person would name it. */
  colour: string;
  token: string;
  was: string;
  now: string;
  /** Every other place the colour is drawn. */
  places: readonly string[];
};

/** A layout change: entries of one region not drawn. */
export type LayoutChange = { mod: string; region: LayoutRegion; hidden: readonly string[] };

/** What Fleet understood. Any of the parts, only those the teaching changes. Each part but the note is kept by a mod named by Fleet. */
export type TeachAnswer = {
  agents?: string;
  /** Words matching `pattern` are links. `example` is one match; `site` is where this repository's matches go, if found. Kept by a Words mod. */
  bridge?: { mod: string; name: string; pattern: string; example: string; site?: Site };
  /** What Fleet does with a notice whose text matches `match`, and what it does instead when a condition holds. Kept by a Fleet mod. */
  fleet?: { mod: string; name: string; match: string; does: FleetDoes; when?: FleetInstead[]; scope: "job" | "repository" | "all" };
  tweak?: Tweak;
  layout?: LayoutChange;
};

/** One thing taught. */
export type Lesson = { id: string; reach: TeachReach; teaching: Teaching; say: string; answer: TeachAnswer; at: string };

/** A pattern to draw as a link before it is kept, in the form `refScanner` takes. */
export type PreviewLink = { pattern: string; address: string; from?: string };

/** What the panel shows back, drawn behind it before it is kept: a link in the thread, a colour on the screen. */
export type Preview = { reach: TeachReach; manifestId?: string; link?: PreviewLink; tweak?: { token: string; now: string } };

type Held = {
  offered: boolean;
  lessons: readonly Lesson[];
  preview: Preview | undefined;
  /** How many teachings this window has saved, which the rules Fleet lists are read again on. */
  saved: number;
};

const NOTHING: Held = { offered: false, lessons: [], preview: undefined, saved: 0 };
let held: Held = NOTHING;
const listeners = new Set<() => void>();
const set = (next: Partial<Held>) => {
  held = { ...held, ...next };
  listeners.forEach((on) => on());
};

/** Whether this window offers Teach. A new window starts from nothing taught. */
export function provideTeaching(offered: boolean): void {
  held = { ...NOTHING, offered };
  modOpened.set(undefined);
  listeners.forEach((on) => on());
}

/** A teaching was kept: the rules Fleet lists may have changed. */
export const teachingSaved = (): void => set({ saved: held.saved + 1 });

/** How many teachings this window has kept. */
export function useTeachingsSaved(): number {
  return useSyncExternalStore(subscribe, () => held.saved);
}

let minted = 0;

/** A mock Fleet keeps a lesson. */
export function saveLesson(input: Omit<Lesson, "id" | "at">): Lesson {
  minted += 1;
  const lesson: Lesson = { ...input, id: `lesson-${minted}`, at: new Date().toISOString() };
  set({ lessons: [...held.lessons, lesson] });
  return lesson;
}

/** Takes back everything the lesson changed: the note, the links and what Fleet does. */
export function forgetLesson(id: string): void {
  set({ lessons: held.lessons.filter((one) => one.id !== id) });
}

/** The lessons as they stand, for a source that serves the Logbook. */
export const lessonsNow = (): readonly Lesson[] => held.lessons;

/** The id the Logbook entry a lesson became goes by, so Forget on the entry reaches the lesson. */
export const taughtId = (lesson: { id: string }): string => `taught:${lesson.id}`;

/** What the panel shows back, drawn behind it before it is kept. */
export function previewTeaching(preview: Preview | undefined): void {
  if (JSON.stringify(held.preview) === JSON.stringify(preview)) return;
  set({ preview });
}

const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));

/** Whether this window can read what is taught: Teach is offered where it can. */
export function useTeachingOffered(): boolean {
  return useSyncExternalStore(subscribe, () => held.offered);
}

export function useLessons(): readonly Lesson[] {
  return useSyncExternalStore(subscribe, () => held.lessons);
}

export const DOES: Record<FleetDoes, string> = { batch: "batched", quiet: "quiet", tell: "told", call: "called to you" };

/** Where a Fleet rule reaches, as `Also changes` words it. */
export const SCOPE = { job: "for this Job", repository: "for this repository", all: "for every repository" } as const;

/** Where an address was found, from the words the wire spells it in (`settings`, `pull_request #N`, `person`), as a tooltip says it. */
export function foundSaid(found: string): string {
  if (found === "settings") return "Found in this repository's settings";
  if (found === "person") return "You said";
  const number = /^pull_request #(\d+)$/.exec(found)?.[1];
  return number === undefined ? found : `Found in pull request #${number}`;
}

/** The address one match goes to under a site. */
export const addressOf = (site: Site, match: string): string => `${site.address}/browse/${match}`;

/** The link the panel is showing back, where it reaches this repository: for it alone, or for every one. */
export function usePreviewLink(manifestId: string | undefined): PreviewLink | undefined {
  const preview = useSyncExternalStore(subscribe, () => held.preview);
  return preview?.link !== undefined && (preview.reach === "all" || preview.manifestId === manifestId) ? preview.link : undefined;
}

/** The tweak the panel is showing back. */
export function usePreviewTweak(): { token: string; now: string } | undefined {
  const preview = useSyncExternalStore(subscribe, () => held.preview);
  return preview?.tweak;
}

/** A rule for notices: Fleet batches or silences those whose text matches, written after the notice the person taught from. */
export type NoticeRule = { name: string; match: string; does: FleetDoes; after: string };

/** The rules that reach a Session: taught in this Job, in its repository, or everywhere. */
export function useNoticeRules(sessionId: string, manifestId: string | undefined): readonly NoticeRule[] {
  const lessons = useLessons();
  const mods = useModList();
  const off = useMemo(() => new Set(mods.filter((one) => !one.enabled).map((one) => one.name)), [mods]);
  return useMemo(
    () =>
      lessons.flatMap((one) => {
        const fleet = one.answer.fleet;
        if (fleet === undefined || one.teaching.at === undefined || off.has(fleet.mod)) return [];
        const there = fleet.scope === "all" || (fleet.scope === "job" ? one.teaching.sessionId === sessionId : one.teaching.manifestId === manifestId);
        return there ? [{ name: fleet.name, match: fleet.match, does: fleet.does, after: one.teaching.at }] : [];
      }),
    [lessons, off, sessionId, manifestId],
  );
}

/**
 * The thread with Fleet's notices as it was taught to send them: those after the one taught from that match a rule are one row
 * for a batch, each line a notice, or gone for a quiet one. A notice no rule matches is as it came.
 */
export function batchNotices(rows: readonly SessionThreadRow[], rules: readonly NoticeRule[]): readonly SessionThreadRow[] {
  if (rules.length === 0) return rows;
  const out: SessionThreadRow[] = [];
  const batched = new Map<string, number>();
  for (const row of rows) {
    const rule =
      row.kind === "message" && row.from === "session" && row.sender.id === "fleet"
        ? rules.find((one) => Date.parse(row.at) > Date.parse(one.after) && new RegExp(one.match).test(row.text))
        : undefined;
    if (rule === undefined || (rule.does !== "batch" && rule.does !== "quiet")) {
      out.push(row);
      continue;
    }
    if (rule.does === "quiet" || row.kind !== "message" || row.from !== "session") continue;
    const into = batched.get(rule.name);
    const was = into === undefined ? undefined : out[into];
    if (into === undefined || was === undefined || was.kind !== "message" || was.from !== "session") {
      batched.set(rule.name, out.length);
      out.push({ ...row, batch: rule.name });
    } else out[into] = { ...was, text: `${was.text}\n${row.text}` };
  }
  return out;
}

/** The Job and repository Teach is pressed in. */
export type TeachHere = { sessionId: string; jobTitle: string; manifestId?: string; root?: string; repository: string };
export const TeachHereContext = createContext<TeachHere | null>(null);

/** Opens the Teach panel on something in this Job, or is absent where the window offers no Teach. */
export function useTeachFrom(here: TeachHere): ((kind: TeachSourceKind, text: string, at?: string, ids?: string[]) => void) | undefined {
  const offered = useTeachingOffered();
  return useMemo(
    () => (!offered ? undefined : (kind, text, at, ids) => void openInPanel({ kind: "teach", ...here, source: kind, text, ...(at === undefined ? {} : { at }), ...(ids === undefined ? {} : { ids }) })),
    [offered, here],
  );
}

/** The same, for a row deep in this Job's panels, which reads the Job from `TeachHereContext`. Absent outside any Job. */
export function useTeach(): ((kind: TeachSourceKind, text: string, at?: string, ids?: string[]) => void) | undefined {
  const here = useContext(TeachHereContext);
  const open = useTeachFrom(here ?? { sessionId: "", jobTitle: "", repository: "" });
  return here === null ? undefined : open;
}

/** The words of a quote the selection hands over, without the `> ` that leads each line. */
export const wordsOfQuote = (quote: string): string => quote.replace(/^> ?/gm, "").trim();

/** The id a teaching on Bridge's own interface carries in place of a Session's: Fleet is told of no Session. */
export const BRIDGE = "bridge";

/** Opens the Teach panel on something of Bridge's own, or is absent where the window offers no Teach. */
export function useTeachBridge(): ((kind: "panel" | "row" | "title_bar", text: string, ids?: string[]) => void) | undefined {
  const offered = useTeachingOffered();
  return useMemo(
    () => (!offered ? undefined : (kind, text, ids) => void openInPanel({ kind: "teach", sessionId: BRIDGE, jobTitle: "Bridge", repository: "Bridge", source: kind, text, ...(ids === undefined ? {} : { ids }) })),
    [offered],
  );
}
