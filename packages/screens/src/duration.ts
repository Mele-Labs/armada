// How long, between two instants the wire carried.
//
// **Bridge computes no durations Fleet did not date.** Both ends come from
// served fields — `created_at` for a whole Job, `entered_at` to `updated_at`
// for one step — so what renders is the record's span rather than a figure
// this process invented.
//
// `fleet.elapsed` is the connection's own reading and rounds to one unit,
// which is right for "last read 4m ago" and wrong for a step that took 4m 09s.

/** An RFC 3339 instant, in epoch milliseconds. `null` where it will not parse. */
export function instant(at: string): number | null {
  const ms = Date.parse(at);
  return Number.isNaN(ms) ? null : ms;
}

/**
 * The span between two instants, as "4m 09s". Hours drop the seconds, because
 * a Job three hours in is not read to the second.
 *
 * `null` where either end will not parse — a Job whose dates are unreadable
 * says nothing rather than showing a span measured from zero.
 */
export function span(from: string, to: string | number): string | null {
  const start = instant(from);
  const end = typeof to === "number" ? to : instant(to);
  if (start === null || end === null) return null;
  return lasting(end - start);
}

/**
 * A whole-Job elapsed, from `JobSummary.started_at` — `undefined` where the
 * Job has never run, `#1008`'s own rule. **The one place that absence is
 * read**, so every surface that draws a run time — the Board row, the Job
 * detail's `Run time` fact, the run header, and a verdict's "took" — shares
 * one answer for a Job waiting at the approval gate or in the queue for a
 * slot: nothing, rather than a figure measured from `created_at` that counts
 * the wait.
 *
 * **Where it stops is the caller's, and `Row.tsx`'s `elapsedOf` is what
 * decides it** for the two surfaces that draw a whole Job's: now while it
 * runs, `ended_at` once it is over.
 */
export function elapsedSince(startedAt: string | undefined, to: string | number): string | undefined {
  return startedAt === undefined ? undefined : (span(startedAt, to) ?? undefined);
}

/**
 * A duration in milliseconds, in the same words a span is written in.
 *
 * **Split out of [`span`] rather than written beside it.** Two formatters for
 * one shape is how "4m 09s" and "4m 9s" come to sit on one screen, and what a
 * caller has differs: a span is two instants and a drone's run is a number
 * Fleet already subtracted.
 */
export function lasting(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${pad(seconds % 60)}s`;
  return `${Math.floor(minutes / 60)}h ${pad(minutes % 60)}m`;
}

/**
 * A short duration, in milliseconds where a second would round it away.
 *
 * **`lasting` rounds to the nearest second, and most of a step is faster than
 * that.** 343 of the 351 calls on the recorded `implement` step answered inside
 * one second, so a body written with `lasting` alone reported a Drone's whole
 * working attempt as a column of `0s`. Anything a second or longer reads in the
 * same words as every other span on the screen, which is `lasting` itself.
 */
export function briefly(ms: number): string {
  const held = Math.max(0, Math.round(ms));
  return held < 1000 ? `${held}ms` : lasting(held);
}

function pad(value: number): string {
  return value < 10 ? `0${value}` : String(value);
}
