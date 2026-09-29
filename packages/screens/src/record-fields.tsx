// The values a Record reading carries past its title, each under its own label,
// and the small helpers every reading shares.
//
// **Its own file so the readings can share it.** `record-read.tsx` draws the
// head and picks the reading; `RowFields` here is the reading of every row
// without one of its own, and `record-task.tsx` and `record-check.tsx` take
// these helpers without reaching back up to the file that imports them.

import type { JobDetail as JobWhole } from "@armada/protocol";

import { Eyebrow, FieldLabel } from "./regions";
import type { LedgerRow } from "./draft/ledger";

/**
 * Every value a row carries past its title, each under its own label (the owner,
 * 29 Sep 2026: *there is just some text with no labels*). **One system for the
 * whole body**: a single value takes `FieldLabel` over it, as a Check's
 * `Expected` does, and a block — a list, a log, a diff, a Drone — takes the
 * `Eyebrow` over it. The two sit at one spacing, so the body is one column. A label the concepts table knows carries its
 * sentence on hover, which is where an explanation goes — never the body.
 *
 * Nothing at all where the row carries nothing more, so a Job created or a
 * Drone started is its status line and no placeholder.
 */
export function RowFields({ row, detail }: { row: LedgerRow; detail: JobWhole }) {
  if (row.kind === "plan_recorded") {
    const planned = detail.work_plan?.tasks ?? [];
    if (planned.length === 0) return null;
    return (
      <section className="armada-ledger__read-section">
        <Eyebrow>Tasks</Eyebrow>
        <ol className="armada-ledger__read-said armada-ledger__read-list">
          {planned.map((task) => (
            <li key={task.id}>
              {task.id} — {task.title}
            </li>
          ))}
        </ol>
      </section>
    );
  }
  if (row.kind === "judged") {
    const [, found] = splitOnce(row.outcome, " — ");
    return (
      <>
        <Field label="Criterion" value={splitOnce(row.what, " · ")[1] ?? row.what} />
        {found === undefined ? null : <Field label="What it found" value={found} />}
      </>
    );
  }
  const label = FIELD_OF[row.kind] ?? "Outcome";
  return row.outcome === "" ? null : <Field label={label} value={row.outcome} />;
}

/** The label over a row's outcome, by kind. Anything unlisted reads `Outcome`. */
const FIELD_OF: Readonly<Record<string, string>> = {
  task_done: "What it showed",
  task_working: "What it showed",
  task_failed: "Why",
  task_dropped: "Why",
  flagged: "Cited",
  evidence_submitted: "Shown by",
  handed_in: "Evidence type",
};

export function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="armada-ledger__read-field">
      <FieldLabel>{label}</FieldLabel>
      <p className="armada-ledger__read-said">{sentenceCase(value)}</p>
    </div>
  );
}

// The part before the first `separator` and the rest, or the whole and nothing.
function splitOnce(said: string, separator: string): [string, string | undefined] {
  const at = said.indexOf(separator);
  return at < 0 ? [said, undefined] : [said.slice(0, at), said.slice(at + separator.length)];
}

export function sentenceCase(said: string): string {
  return said.charAt(0).toUpperCase() + said.slice(1);
}

// `outputs.fetch` is keyed by the file's own name and never the whole path —
// `fleet` builds that name out of the run's key, so a fixture or a record
// keyed on the path would never be found.
export function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
