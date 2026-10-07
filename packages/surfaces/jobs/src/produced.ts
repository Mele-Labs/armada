// When the diff sheet takes the Job's patch again.

import { useEffect } from "react";

import type { ChangedFile as WireFile, Turn } from "@armada/protocol";

/**
 * What the live footprint last said this Job has written, as one value an
 * effect can depend on.
 *
 * **The signal the diff sheet re-reads on.** The sheet's reading is taken when
 * it opens, so a Drone that wrote afterwards would leave it stale. This is
 * what says the list moved.
 *
 * The last reading wins: Fleet republishes the footprint only where it
 * changed, so a Drone editing the same files for an hour produces one value
 * and no re-read.
 */
export function wroteSoFar(turns: readonly Turn[]): string {
  let files: readonly WireFile[] = [];
  for (const turn of turns) if (turn.saw.event === "produced") files = turn.saw.files;
  return files.map((file) => `${file.change} ${file.path}`).join("\n");
}

/**
 * How often an open diff takes its reading again while a Drone is writing.
 *
 * **Five seconds, and it is the resolution of a live patch rather than a
 * setting.** `FOOTPRINT_INTERVAL` in `crates/fleet/src/footprint.rs` is two,
 * for a reading that costs a directory walk; this one costs the patch, which
 * `docs/journeys/monitor-active-work.md` measured at 25ms over a hundred files
 * and 90ms over four hundred. Slower than a person re-reads a hunk, faster
 * than they can wonder whether it is stuck.
 */
const WHILE_OPEN = 5_000;

/**
 * Take the patch again for the sheet somebody is looking at: on the press that
 * opens it, whenever the file list moves under it, and on a clock while a
 * Drone is still writing.
 *
 * **Only while it is open, and the clock only while a Drone holds the pen.**
 * The patch is the megabyte the split in `crates/ipc/src/work.rs` exists to
 * save. Fleet republishes a footprint only where the *list* changed, so the
 * event alone left a Drone editing the same seven files drawing the hunks as
 * of whenever the list last moved — the reading a person opened the sheet to
 * watch, frozen with nothing saying so. A worktree nobody is writing to cannot
 * go stale, so a Job with no Drone on it pays nothing.
 *
 * `apps/desktop/src/main/review.ts` holds why taking it again does not blank
 * the layer being read.
 */
export function useDiffAgain(
  read: (jobId: string | null) => void,
  jobId: string,
  sheet: string | null,
  turns: readonly Turn[],
  working: boolean,
): void {
  const wrote = wroteSoFar(turns);
  useEffect(() => {
    if (sheet === "diff") read(jobId);
  }, [sheet, wrote, jobId]);
  useEffect(() => {
    if (sheet !== "diff" || !working) return;
    const ticking = setInterval(() => read(jobId), WHILE_OPEN);
    return () => clearInterval(ticking);
  }, [sheet, working, jobId]);
}
