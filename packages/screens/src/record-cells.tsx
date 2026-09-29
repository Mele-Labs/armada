// The two things a Record row draws rather than says: the mark leading it, and
// a path cell that keeps the filename.
//
// **Beside `record.ts` rather than inside it.** That file is the Record's
// English and holds no JSX; these are chrome.

import type { ReactNode } from "react";
import { FileCheck, FileDiff } from "lucide-react";

import { PathChip, type JobLedgerMark } from "@armada/components";

import { familyOf, type LedgerRow } from "./draft/ledger";

/** 12px at strokeWidth 2 — `docs/contracts/iconography.md`'s only badge size. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

/**
 * The mark a row leads with under All, or nothing where the registry assigns
 * its family no glyph.
 *
 * **Two families have one and six do not.** `file-check` is reserved to a
 * submission that landed and `file-diff` to reading what one file changed.
 * Nothing in `packages/icons/icons.toml` means *a Check ran*, *a Judge
 * answered*, *a Drone arrived*, *a task moved*, *a case was run* or *the Job
 * itself moved* — the shield and circle families are outcomes rather than
 * sources, and the contract's first rule for anything unlisted is no icon.
 * `[record-kind-marks]` is where the six are owed.
 */
export function markFor(kind: string): JobLedgerMark | undefined {
  const family = familyOf(kind);
  // `says` is the kind as the record spells it, which is what the column this
  // replaced drew and what the tooltip now carries.
  if (family === "evidence") {
    return { glyph: <FileCheck size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />, says: kind };
  }
  if (family === "files") {
    return { glyph: <FileDiff size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />, says: kind };
  }
  return undefined;
}

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
 * paths ran under the sheet's Close at 1280. The paths are read whole under it.
 */
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
