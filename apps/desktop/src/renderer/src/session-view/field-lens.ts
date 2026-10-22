// The diff lens on a Job's Field: which of its drones are writing which file, the change Fleet answers for each, the card's file and the
// earlier files behind it. Asks Fleet through `lens-asks.ts` only while the Field is live, shown and a drone writes. A refused file draws
// no lens and says nothing. `docs/concepts/cockpit.md`, *A Job open*.

import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import type { TimelineEvent } from "@armada/protocol";

import { insideOf } from "../cockpit/deck/areas";
import type { Burst } from "../cockpit/deck/afterglow";
import type { DroneIn } from "../cockpit/deck/layout";
import { cardOf, lensOf, newestOf, writersOf, type LensDiff, type LensHunk, type Writer } from "../cockpit/deck/lens";
import { useSessionsDraft } from "../sessions-draft";
import { LensAsks } from "./lens-asks";

/** A file lifted on the floor: the file, the drone writing it, whether it is the card's, and its change. */
export type LensIn = { path: string; key: string; card: boolean; diff: LensDiff };

/** The newest hunk of the card's file, how many of its lines were read before, and a number that is new with each answer. */
export type Newest = { hunk: LensHunk; read: number; n: number };

/** What the card shows: the file, its change, its newest hunk, and the earlier files the Job wrote, newest first. */
export type CardIn = { path: string; diff: LensDiff; newest: Newest | undefined; tabs: readonly { path: string; diff: LensDiff | undefined }[] };

const NO_LENS: { slabs: LensIn[]; card: CardIn | undefined } = { slabs: [], card: undefined };

/** The most files whose last answer is kept, for the tabs and for `seen`. */
const KEPT = 24;
/** The earlier files stacked behind the card. */
const TABS = 2;

/**
 * What the lens draws from what it holds: a slab for each file being written whose change has been answered and not refused, the card
 * on the file of the drone it follows, and behind it the files the Job wrote last before it.
 */
export function lensFrom(i: { writers: readonly Writer[]; pinned: string | undefined; kept: ReadonlyMap<string, LensDiff>; newest: ReadonlyMap<string, Newest>; refused: ReadonlySet<string>; bursts: readonly Burst[] }): { slabs: LensIn[]; card: CardIn | undefined } {
  const card = cardOf(i.writers, i.pinned);
  const slabs: LensIn[] = [];
  for (const one of i.writers) {
    const diff = i.kept.get(one.path);
    if (diff === undefined || i.refused.has(one.path) || slabs.some((it) => it.path === one.path)) continue;
    slabs.push({ path: one.path, key: one.key, card: card?.path === one.path, diff });
  }
  const shown = card === undefined ? undefined : slabs.find((one) => one.path === card.path);
  if (shown === undefined) return { slabs, card: undefined };
  const earlier: string[] = [];
  for (let k = i.bursts.length - 1; k >= 0 && earlier.length < TABS; k -= 1) {
    const burst = i.bursts[k]!;
    if (burst.act !== "write") continue;
    for (const path of burst.paths) if (path !== shown.path && !earlier.includes(path) && earlier.length < TABS) earlier.push(path);
  }
  return { slabs, card: { path: shown.path, diff: shown.diff, newest: i.newest.get(shown.path), tabs: earlier.map((path) => ({ path, diff: i.kept.get(path) })) } };
}

/** The Job's own drones writing a file now, on the floor. Another Job's never write here. */
export const writingOf = (drones: readonly DroneIn[]): { key: string; path: string }[] =>
  drones.flatMap((one) => (one.ghost !== true && one.off === undefined && one.doing === "write" && one.file !== undefined ? [{ key: one.key, path: one.file }] : []));

/** Whether the element has a size and the window is not hidden: folded away or hidden, the lens asks nothing. */
export function useShown(ref: RefObject<HTMLElement | null>): boolean {
  const [sized, setSized] = useState(true);
  const [visible, setVisible] = useState(() => typeof document === "undefined" || document.visibilityState !== "hidden");
  useEffect(() => {
    const one = ref.current;
    if (one === null) return;
    const read = () => {
      const box = one.getBoundingClientRect();
      setSized(box.width > 0 && box.height > 0);
    };
    read();
    const watch = new ResizeObserver(read);
    watch.observe(one);
    const seen = () => setVisible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", seen);
    return () => {
      watch.disconnect();
      document.removeEventListener("visibilitychange", seen);
    };
  }, [ref]);
  return sized && visible;
}

/** The lens of a Job's Field. `active` is the Field live and shown; while it is not, nothing is asked and nothing lifts. */
export function useLens(input: { session: string; drones: readonly DroneIn[]; pinned: string | undefined; events: readonly TimelineEvent[]; roots: readonly string[]; bursts: readonly Burst[]; active: boolean }): { slabs: readonly LensIn[]; card: CardIn | undefined } {
  const { session, drones, pinned, events, roots, bursts, active } = input;
  const fileChange = useSessionsDraft()?.fileChange;
  const kept = useRef(new Map<string, LensDiff>());
  const newest = useRef(new Map<string, Newest>());
  const [refused, setRefused] = useState<ReadonlySet<string>>(new Set());
  const [answers, bump] = useState(0);

  const writing = writingOf(drones);
  const writingKey = writing.map((one) => `${one.key}=${one.path}`).join("|");
  const started = useRef(new Map<string, Writer>());
  const writers = useMemo(() => {
    started.current = writersOf(started.current, writing, Date.now());
    return [...started.current.values()];
  }, [writingKey]);

  // One asker for the Job; a new Job, or a draft that serves no reads, starts afresh.
  const asks = useRef<LensAsks>(undefined);
  useEffect(() => {
    kept.current = new Map();
    newest.current = new Map();
    setRefused(new Set());
    if (fileChange === undefined) return;
    let n = 0;
    const one = new LensAsks(
      (path, seen) => fileChange(session, path, seen),
      (path) => kept.current.get(path)?.stamp,
      (path, read) => {
        if (read === undefined) {
          setRefused((was) => (was.has(path) ? was : new Set([...was, path])));
          return;
        }
        // Read once, here: the counts, the rows and the newest hunk stand until the next answer.
        const diff = lensOf(read);
        const was = kept.current.get(path);
        const fresh = newestOf(was, diff);
        if (fresh !== undefined) newest.current.set(path, { ...fresh, n: ++n });
        else if (diff.hunks.length === 0) newest.current.delete(path);
        kept.current.delete(path);
        kept.current.set(path, diff);
        if (kept.current.size > KEPT) kept.current.delete(kept.current.keys().next().value!);
        setRefused((had) => (had.has(path) ? new Set([...had].filter((it) => it !== path)) : had));
        bump((k) => k + 1);
      },
    );
    asks.current = one;
    return () => {
      one.dispose();
      asks.current = undefined;
    };
  }, [fileChange, session]);

  const pathsKey = [...new Set(writers.map((one) => one.path))].sort().join("|");
  useEffect(() => {
    asks.current?.want(active ? pathsKey.split("|").filter((one) => one !== "") : []);
  }, [active, pathsKey, fileChange, session]);

  // A `session.edited` entry naming a file being written asks for it, as its two seconds allow. Entries already there when the Field opened ask nothing.
  const read = useRef<number | undefined>(undefined);
  const rootsKey = roots.join("|");
  useEffect(() => {
    const newestId = events.reduce((most, one) => Math.max(most, one.id), -Infinity);
    const since = read.current;
    read.current = newestId;
    if (since === undefined) return;
    for (const one of events) {
      if (one.id <= since || one.kind !== "session.edited") continue;
      for (const path of one.items ?? (one.detail === undefined ? [] : [one.detail])) {
        const named = insideOf(path, roots);
        if (named !== undefined) asks.current?.edited(named);
      }
    }
  }, [events, rootsKey]);

  // `kept` and `newest` move with `answers`, which every answer bumps.
  return useMemo(() => (active ? lensFrom({ writers, pinned, kept: kept.current, newest: newest.current, refused, bursts }) : NO_LENS), [active, writers, pinned, refused, bursts, answers]);
}
