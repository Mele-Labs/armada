// What the owner taught Armada, from anything that happens in a Job: selected words, a command, a Fleet notice, a plan task, a
// Check, a pull request. A teaching can change three things: **Agents** (a note, which becomes a Logbook entry), **Bridge**
// (how Bridge draws it: for now words matching a pattern become links) and **Fleet** (what it does with a notice).
//
// A pattern is taught once and an address is found per repository, because a Jira site is different in each: the source finds
// it (the repository's own settings, then its recent pull requests) and asks the person only when it finds none.
//
// **Held in this window only, and offered only where a window provides a source**: Fleet's read of what is said is not built yet,
// so a real window has none and draws no Teach. The mock provides one (`mock/teach.ts`) and starts every window with nothing taught.

import { createContext, useContext, useEffect, useMemo, useSyncExternalStore } from "react";
import type { SessionThreadRow, TeachReach, TeachSourceKind } from "@armada/components";

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

/** What Fleet understood. Any of the three parts, only those the teaching changes. */
export type TeachAnswer = {
  agents?: string;
  /** Words matching `pattern` are links. `example` is one match; `site` is where this repository's matches go, if found. */
  bridge?: { name: string; pattern: string; example: string; site?: Site };
  /** What Fleet does with a notice whose text matches `match`. `says` is that in words. */
  fleet?: { name: string; match: string; does: FleetDoes; says: string; scope: "job" | "repository" | "all" };
};

/** One thing taught. */
export type Lesson = { id: string; reach: TeachReach; teaching: Teaching; say: string; answer: TeachAnswer; at: string };

export type TeachSource = {
  /** Fleet reads what the person said about the teaching. */
  read: (input: { teaching: Teaching; say: string; reach: TeachReach }) => Promise<TeachAnswer>;
  /** Fleet finds a repository's site, or asks the person and answers through `onSite` when they have. */
  site: (manifestId: string, repository: string) => Promise<Site | undefined>;
  onSite: (listen: (manifestId: string, site: Site) => void) => () => void;
  /** A lesson was kept. */
  saved?: (lesson: Lesson) => void;
};

type Held = { source: TeachSource | undefined; lessons: readonly Lesson[]; preview: Lesson | undefined; sites: Readonly<Record<string, Site>>; asked: ReadonlySet<string> };

const NOTHING: Held = { source: undefined, lessons: [], preview: undefined, sites: {}, asked: new Set() };
let held: Held = NOTHING;
let unlisten: (() => void) | undefined;
const listeners = new Set<() => void>();
const set = (next: Partial<Held>) => {
  held = { ...held, ...next };
  listeners.forEach((on) => on());
};

/** The window's source, or none; a new source starts from nothing taught. */
export function provideTeaching(source: TeachSource | undefined): void {
  unlisten?.();
  held = { ...NOTHING, source };
  unlisten = source?.onSite((manifestId, site) => set({ sites: { ...held.sites, [manifestId]: site } }));
  listeners.forEach((on) => on());
}

/** Asks Fleet to read what was said. Throws where the window has no source, which the Teach act's absence already rules out. */
export function readTeaching(input: Parameters<TeachSource["read"]>[0]): Promise<TeachAnswer> {
  if (held.source === undefined) return Promise.reject(new Error("nothing reads what is taught here"));
  return held.source.read(input);
}

let minted = 0;

export function saveLesson(input: Omit<Lesson, "id" | "at">): Lesson {
  minted += 1;
  const lesson: Lesson = { ...input, id: `lesson-${minted}`, at: new Date().toISOString() };
  const site = lesson.answer.bridge?.site;
  const manifestId = lesson.teaching.manifestId;
  set({
    lessons: [...held.lessons, lesson],
    preview: undefined,
    ...(site === undefined || manifestId === undefined ? {} : { sites: { ...held.sites, [manifestId]: site } }),
  });
  held.source?.saved?.(lesson);
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

/** What the panel shows back, drawn in the thread behind it before it is kept. */
export function previewLesson(lesson: Lesson | undefined): void {
  if (held.preview === lesson) return;
  set({ preview: lesson });
}

const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));

/** Whether this window can read what is taught: Teach is offered where it can. */
export function useTeachingOffered(): boolean {
  return useSyncExternalStore(subscribe, () => held.source !== undefined);
}

export function useLessons(): readonly Lesson[] {
  return useSyncExternalStore(subscribe, () => held.lessons);
}

/** Where an address came from, in the words a tooltip says. */
export function siteSaid(from: SiteFrom): string {
  return from.kind === "settings" ? "Found in this repository's settings" : from.kind === "pull_request" ? `Found in pull request #${from.number}` : "You said";
}

/** The address one match goes to under a site. */
export const addressOf = (site: Site, match: string): string => `${site.address}/browse/${match}`;

const reaches = (one: Lesson, manifestId: string | undefined) => one.reach === "all" || one.teaching.manifestId === manifestId;

/**
 * The rules that draw words as links in a repository's Sessions: those taught there or everywhere, and the one the panel is
 * showing back. The address is the repository's site, found once and kept; a repository with none found has its keys plain
 * until its site is known.
 */
export function useTaughtLinks(manifestId: string | undefined, repository: string | undefined): readonly { pattern: string; address: string; from: string }[] {
  const all = useSyncExternalStore(subscribe, () => held);
  const rules = useMemo(
    () => [...all.lessons, ...(all.preview === undefined ? [] : [all.preview])].filter((one) => one.answer.bridge !== undefined && reaches(one, manifestId)),
    [all.lessons, all.preview, manifestId],
  );
  const wanted = rules.length > 0 && manifestId !== undefined && repository !== undefined;
  useEffect(() => {
    const source = held.source;
    if (!wanted || source === undefined || held.sites[manifestId] !== undefined || held.asked.has(manifestId)) return;
    held = { ...held, asked: new Set([...held.asked, manifestId]) };
    void source.site(manifestId, repository).then((site) => site !== undefined && set({ sites: { ...held.sites, [manifestId]: site } }));
  }, [wanted, manifestId, repository]);
  return useMemo(
    () =>
      rules.flatMap((one) => {
        const site = all.sites[manifestId ?? ""] ?? (one.teaching.manifestId === manifestId ? one.answer.bridge!.site : undefined);
        return site === undefined ? [] : [{ pattern: one.answer.bridge!.pattern, address: `${site.address}/browse/{match}`, from: siteSaid(site.from) }];
      }),
    [rules, all.sites, manifestId],
  );
}

/** A rule for notices: Fleet batches or silences those whose text matches, written after the notice the person taught from. */
export type NoticeRule = { name: string; match: string; does: FleetDoes; after: string };

/** The rules that reach a Session: taught in this Job, in its repository, or everywhere. */
export function useNoticeRules(sessionId: string, manifestId: string | undefined): readonly NoticeRule[] {
  const lessons = useLessons();
  return useMemo(
    () =>
      lessons.flatMap((one) => {
        const fleet = one.answer.fleet;
        if (fleet === undefined || one.teaching.at === undefined) return [];
        const there = fleet.scope === "all" || (fleet.scope === "job" ? one.teaching.sessionId === sessionId : one.teaching.manifestId === manifestId);
        return there ? [{ name: fleet.name, match: fleet.match, does: fleet.does, after: one.teaching.at }] : [];
      }),
    [lessons, sessionId, manifestId],
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
export function useTeachFrom(here: TeachHere): ((kind: TeachSourceKind, text: string, at?: string) => void) | undefined {
  const offered = useTeachingOffered();
  return useMemo(
    () => (!offered ? undefined : (kind, text, at) => void openInPanel({ kind: "teach", ...here, source: kind, text, ...(at === undefined ? {} : { at }) })),
    [offered, here],
  );
}

/** The same, for a row deep in this Job's panels, which reads the Job from `TeachHereContext`. Absent outside any Job. */
export function useTeach(): ((kind: TeachSourceKind, text: string, at?: string) => void) | undefined {
  const here = useContext(TeachHereContext);
  const open = useTeachFrom(here ?? { sessionId: "", jobTitle: "", repository: "" });
  return here === null ? undefined : open;
}

/** The words of a quote the selection hands over, without the `> ` that leads each line. */
export const wordsOfQuote = (quote: string): string => quote.replace(/^> ?/gm, "").trim();
