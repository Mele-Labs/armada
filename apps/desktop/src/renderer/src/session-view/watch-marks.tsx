// A watch's state as a mark: the glyph `[conventions.watch_state_borrowing]` in `packages/icons/` lends, named by its tooltip.
// The words are here until `enum-verbs.toml` carries a `watch_state` vocabulary.

import { Ban, Check, CircleDashed, CircleDot, Clock, CornerUpRight, Unplug, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { WaitingItem, WatchReading, WatchState, WatchSummary, WatchVerdict } from "@armada/protocol";

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

/** What was gathered for a watch that ended not ours: each failed part with its own link, error lines and log end, every link once, every log, and the exact errors the agent's weighing carried. */
export type NotOursEvidence = {
  parts: { name: string; link?: string; error?: string; tail?: string }[];
  links: string[];
  logs: string[];
  errors: string[];
};

/** The newest reading that kept a log. */
const lastLog = (watch: WatchSummary): string | undefined => [...readingsOf(watch)].reverse().find((one) => one.log !== undefined)?.log;

export function notOursEvidence(watch: WatchSummary): NotOursEvidence {
  const failed = new Set(notOursParts(watch));
  const read = [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "read" }> => one.kind === "read" && one.parts !== undefined);
  const parts = (read?.parts ?? []).filter((one) => failed.has(one.name)).map(({ name, link, error, tail }) => ({ name, ...(link === undefined ? {} : { link }), ...(error === undefined ? {} : { error }), ...(tail === undefined ? {} : { tail }) }));
  // The weighing that set these parts aside: the newest not ours naming any of them.
  const weighed = [...(watch.verdicts ?? [])].reverse().find((one) => one.verdict === "not_ours" && (one.parts ?? []).some((part) => failed.has(part)))?.evidence;
  const once = (all: (string | undefined)[]): string[] => [...new Set(all.filter((one): one is string => one !== undefined && one.trim() !== ""))];
  return {
    parts,
    links: once([lastRead(watch)?.link, ...parts.map((one) => one.link), ...(weighed?.links ?? [])]),
    logs: once([lastLog(watch), ...(weighed?.logs ?? [])]),
    errors: once(weighed?.errors ?? []),
  };
}

/**
 * What Escalate shows for a watch that ended not ours, for the owners of what broke: what failed, when, on which run, what the reading said, each
 * part's exact error lines, the agent's, every link, every log, and the end of each part's log. Facts only, one block a fact.
 */
export function escalation(watch: WatchSummary): string {
  const read = lastRead(watch);
  const evidence = notOursEvidence(watch);
  const block = (head: string, lines: readonly string[]) => (lines.length === 0 ? [] : [[head, ...lines].join("\n")]);
  return [
    [
      `${notOursParts(watch).join(", ")} failed in ${watch.name}.`,
      ...(read === undefined ? [] : [`When: ${utc(read.at)}`]),
      `Run: ${watch.name}`,
      ...(read === undefined ? [] : [`Error: ${read.says}`]),
    ].join("\n"),
    ...evidence.parts.flatMap((one) => block(`${one.name} failed on:`, one.error === undefined ? [] : [one.error])),
    ...block("Errors the agent found:", evidence.errors),
    ...block("Links:", evidence.links),
    ...block("Logs:", evidence.logs),
    ...evidence.parts.flatMap((one) => block(`The end of ${one.name}'s log:`, one.tail === undefined ? [] : [one.tail])),
  ].join("\n\n");
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

/** The watches the Watching block lists: every one on Show ended, else the live ones and any that ended not ours while Needs you still asks about it. */
export function watchesShown(watches: readonly WatchSummary[], waitingFor: readonly WaitingItem[], ended: boolean): WatchSummary[] {
  if (ended) return [...watches];
  const asked = new Set(waitingFor.filter((one) => one.source === "watch_not_ours").map((one) => one.act?.target));
  return watches.filter((watch) => watch.live || asked.has(watch.id));
}

/** The kept readings, oldest first: `readings` where Fleet sends them, else `last` alone. */
export const readingsOf = (watch: WatchSummary): WatchReading[] => watch.readings ?? (watch.last === undefined ? [] : [watch.last]);

/** The verdicts that weighed `reading`: given at or after it, and before the reading after it. */
export function verdictsOn(watch: WatchSummary, reading: WatchReading): WatchVerdict[] {
  const readings = readingsOf(watch);
  const next = readings.find((one) => one.at > reading.at)?.at;
  return (watch.verdicts ?? []).filter((one) => one.at >= reading.at && (next === undefined || one.at < next));
}
