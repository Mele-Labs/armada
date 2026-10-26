// A watch's state as a mark: the glyph `[conventions.watch_state_borrowing]` in `packages/icons/` lends, named by its tooltip.
// The words are here until `enum-verbs.toml` carries a `watch_state` vocabulary.

import { Ban, Check, CircleDashed, CircleDot, Clock, CornerUpRight, Unplug, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { WatchReading, WatchState, WatchSummary, WatchVerdict } from "@armada/protocol";

import "./watching-block.css";

export const WATCH_MARK: Record<WatchState, { Glyph: LucideIcon; said: string }> = {
  waiting: { Glyph: CircleDashed, said: "Waiting" },
  running: { Glyph: CircleDot, said: "Running" },
  passed: { Glyph: Check, said: "Passed" },
  failed: { Glyph: X, said: "Failed" },
  not_ours: { Glyph: CornerUpRight, said: "Not ours" },
  gave_up: { Glyph: Clock, said: "Gave up" },
  broken: { Glyph: Unplug, said: "Broken" },
  stopped: { Glyph: Ban, said: "Stopped" },
};

/** `said` names the mark where its state alone says too little: a watch that ended not ours names what failed. */
export function WatchMark({ state, said: named, className = "sv-watch-mark" }: { state: WatchState; said?: string; className?: string }) {
  const { Glyph, said: word } = WATCH_MARK[state];
  const said = named ?? word;
  return (
    <Tooltip label={said}>
      <span className={className} data-state={state} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The parts set aside that failed in the newest reading that named parts: what a watch that ended not ours names. */
export function notOursParts(watch: WatchSummary): string[] {
  const aside = new Set((watch.set_aside ?? []).map((one) => one.part));
  const read = [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "read" }> => one.kind === "read" && one.parts !== undefined);
  return (read?.parts ?? []).filter((one) => one.state === "failed" && aside.has(one.name)).map((one) => one.name);
}

/** A watch's mark in words: its state's, and for not ours what failed, as `Not ours: provision failed`. */
export const watchSaid = (watch: WatchSummary): string => {
  const parts = watch.state === "not_ours" ? notOursParts(watch) : [];
  return parts.length === 0 ? WATCH_MARK[watch.state].said : `Not ours: ${parts.join(", ")} failed`;
};

/** The newest reading that read, as Fleet took it. */
const lastRead = (watch: WatchSummary): Extract<WatchReading, { kind: "read" }> | undefined => [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "read" }> => one.kind === "read");

/** `2026-10-26T14:02:31.000Z` as `2026-10-26 14:02 UTC`: plain to whoever reads it, wherever they are. */
const utc = (at: string): string => `${at.slice(0, 10)} ${at.slice(11, 16)} UTC`;

/** What Escalate shows for a watch that ended not ours, for the owners of what broke: what failed, when, on which run, what the reading said, and the link. Facts only. */
export function escalation(watch: WatchSummary): string {
  const read = lastRead(watch);
  return [
    `${notOursParts(watch).join(", ")} failed in ${watch.name}.`,
    ...(read === undefined ? [] : [`When: ${utc(read.at)}`]),
    `Run: ${watch.name}`,
    ...(read === undefined ? [] : [`Error: ${read.says}`]),
    ...(read?.link === undefined ? [] : [`Link: ${read.link}`]),
  ].join("\n");
}

/** The failing run's link: the newest reading's that read. */
export const failingLink = (watch: WatchSummary): string | undefined => lastRead(watch)?.link;

/** A reading's state: what its line said, or broken where it gave none. */
export const readingState = (reading: WatchReading): WatchState => (reading.kind === "broken" ? "broken" : reading.state);

/** What a reading says on a row: its sentence, or why it gave none. */
export const readingSays = (reading: WatchReading): string => (reading.kind === "broken" ? reading.why : reading.says);

/** The weighing in words, as the row and the panel draw it under a reading. */
export const verdictSaid = (verdict: WatchVerdict): string => {
  const who = verdict.verdict === "ours" ? "Ours" : "Not ours";
  return verdict.sentence === undefined || verdict.sentence === "" ? who : `${who}: ${verdict.sentence}`;
};

/** The kept readings, oldest first: `readings` where Fleet sends them, else `last` alone. */
export const readingsOf = (watch: WatchSummary): WatchReading[] => watch.readings ?? (watch.last === undefined ? [] : [watch.last]);

/** The verdicts that weighed `reading`: given at or after it, and before the reading after it. */
export function verdictsOn(watch: WatchSummary, reading: WatchReading): WatchVerdict[] {
  const readings = readingsOf(watch);
  const next = readings.find((one) => one.at > reading.at)?.at;
  return (watch.verdicts ?? []).filter((one) => one.at >= reading.at && (next === undefined || one.at < next));
}
