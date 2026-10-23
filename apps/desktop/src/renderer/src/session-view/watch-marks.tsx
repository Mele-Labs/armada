// A watch's state as a mark: the glyph `[conventions.watch_state_borrowing]` in `packages/icons/` lends, named by its tooltip.
// The words are here until `enum-verbs.toml` carries a `watch_state` vocabulary.

import { Ban, Check, CircleDashed, CircleDot, Clock, Unplug, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { WatchReading, WatchState, WatchSummary, WatchVerdict } from "@armada/protocol";

import "./watching-block.css";

export const WATCH_MARK: Record<WatchState, { Glyph: LucideIcon; said: string }> = {
  waiting: { Glyph: CircleDashed, said: "Waiting" },
  running: { Glyph: CircleDot, said: "Running" },
  passed: { Glyph: Check, said: "Passed" },
  failed: { Glyph: X, said: "Failed" },
  gave_up: { Glyph: Clock, said: "Gave up" },
  broken: { Glyph: Unplug, said: "Broken" },
  stopped: { Glyph: Ban, said: "Stopped" },
};

export function WatchMark({ state, className = "sv-watch-mark" }: { state: WatchState; className?: string }) {
  const { Glyph, said } = WATCH_MARK[state];
  return (
    <Tooltip label={said}>
      <span className={className} data-state={state} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

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
