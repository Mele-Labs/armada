// The two things a Record row draws rather than says: the mark leading it, and
// a path cell that keeps the filename.
//
// **Beside `record.ts` rather than inside it.** That file is the Record's
// English and holds no JSX; these are chrome.

import type { ReactNode } from "react";
import { Bot, Box, FileCheck, FileDiff, FlaskConical, ListTodo, Scale, Shield } from "lucide-react";

import { PathChip, Prose, type JobLedgerMark } from "@armada/components";

import { familyOf, type LedgerFamily, type LedgerRow } from "./draft/ledger";

/** 12px at strokeWidth 2 — `docs/contracts/iconography.md`'s only badge size. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

/**
 * The mark a row leads with under All, or nothing where the registry assigns
 * its family no glyph.
 *
 * **Every family has one** (the owner asked for the six that were owed, 29 Sep
 * 2026). `file-check` and `file-diff` keep their own reservations; `box`,
 * `scale`, `bot`, `list-todo` and `flask-conical` were minted for the Job, a
 * Judge, a Drone, a task and a case run. A Check takes the bare `shield`, under
 * a rule-4 exception the owner gave for this column alone —
 * `docs/contracts/iconography.md`, *The Record's kind marks*.
 * `[conventions.record_kind_mark]` in `packages/icons/icons/` holds the
 * reasoning for each.
 */
export function markFor(kind: string): JobLedgerMark | undefined {
  const family = familyOf(kind);
  if (family === null) return undefined;
  const Glyph = MARKS[family];
  // `says` is the kind as the record spells it, which is what the column this
  // replaced drew and what the tooltip now carries.
  return { glyph: <Glyph size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />, says: kind };
}

const MARKS: Record<LedgerFamily, typeof FileCheck> = {
  job: Box,
  evidence: FileCheck,
  files: FileDiff,
  checks: Shield,
  judges: Scale,
  drones: Bot,
  tasks: ListTodo,
  tests: FlaskConical,
};

/**
 * The kinds whose `what` is a path, or a list of them.
 *
 * Read from what `draft/ledger.ts` puts there: a written file's path, a kept
 * deliverable's path, and a finished task's scope joined with `, `.
 */
const PATHS_IN: readonly string[] = ["file_written", "deliverable_kept", "task_files"];

/**
 * What the row is about, as the table draws it.
 *
 * **A path reads as its filename, with the whole of it on hover** (the owner, 28
 * September 2026). A column of `packages/screens/src/…` is a column saying
 * nothing, which is the defect `PathChip` was built for — and passing it no
 * `directory` is how the chip draws the filename alone.
 */
export function whatCellOf(row: LedgerRow): ReactNode {
  if (!PATHS_IN.includes(row.kind)) return row.what;
  const paths = row.what.split(", ").filter((one) => one !== "");
  if (paths.length === 0) return row.what;
  return (
    <span className="armada-ledger__files">
      {paths.map((path) => (
        <PathChip key={path} basename={basenameOf(path)} title={path} />
      ))}
    </span>
  );
}

/**
 * The open row's name, for the sheet's title. **A row of paths is named by
 * their filenames**, as its What cell is: a title does not wrap, and two whole
 * paths ran under the sheet's Close at 1280. A File row's paths are read whole
 * in its diff's section headers; a kept deliverable's, under the title.
 */
/** The Outcome cell. A failed task's reason is a Drone's markdown; other outcomes are Fleet's words. */
export function outcomeCellOf(row: LedgerRow): ReactNode {
  return row.kind === "task_failed" ? <Prose text={row.outcome} /> : row.outcome;
}

export function titleOf(row: LedgerRow): string {
  if (!PATHS_IN.includes(row.kind)) return row.what;
  const paths = row.what.split(", ").filter((one) => one !== "");
  return paths.length === 0 ? row.what : paths.map(basenameOf).join(", ");
}

// A path at the repository root has no separator, and `lastIndexOf` returning
// -1 makes the slice the whole string, which is the right answer for one.
function basenameOf(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
