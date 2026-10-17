// How long a checkout has been held, for Cleanup's tiles.
//
// **Its own file because it is arithmetic, and arithmetic is unit-tested.**

import { instant } from "./duration";

/**
 * How long a checkout has been sitting, in the coarsest true unit.
 *
 * **Its own formatter and not `lasting`.** That one writes a run — `4m 09s`,
 * `2h 13m` — because a job in flight is read to the second. This is an age, and
 * `97h 12m` is a number nobody converts in their head at the moment they are
 * deciding whether four days of untouched work is worth opening the directory
 * for. Two quantities, two formatters, and neither pretends to be the other.
 *
 * **It rounds down and never up.** `last_moved_at` is already a floor — armada
 * last moved the job then, and the files were written at or before it — so
 * rounding up would turn a floor into a claim.
 *
 * `null` where the stamp will not parse or is in the future, which is the same
 * convention `instant` sets: a checkout whose date is unreadable says nothing
 * rather than showing an age measured from zero.
 */
export function sitting(at: string, now: number): string | null {
  const moved = instant(at);
  if (moved === null || moved > now) return null;
  const minutes = Math.floor((now - moved) / 60_000);
  if (minutes < 1) return "under a minute";
  if (minutes < 60) return minutes === 1 ? "1 minute" : `${minutes} minutes`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours === 1 ? "1 hour" : `${hours} hours`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "1 day" : `${days} days`;
}
