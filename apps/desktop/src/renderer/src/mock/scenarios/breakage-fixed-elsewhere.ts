import { retryingCheckFailure, running } from "@armada/jobs/fixtures/build/index";

import { asRow, holding } from "../holding";
import { brokenOnMain, FIX_TITLE, withBreakages } from "../job-detail-fixtures";
import type { Scenario } from "../moment";

/**
 * Three Jobs on one claim, #1673: one fixing a test broken on main, and two
 * parked on the fix — the first because its Check failed on that test.
 * **One claim on every detail**, because Fleet serves one entry to either
 * side: the fix names what it claims, and a parked Job names who is fixing.
 */
function fixedElsewhere(): Scenario {
  const fixing = asRow(running(), 90, "fixing", FIX_TITLE);
  const parked = asRow(retryingCheckFailure(), 91, "parked", "Trim the brief to the files the step touched");
  const alsoParked = asRow(running(), 92, "parked-too", "Memoise the manifest list");
  const claim = {
    ...brokenOnMain(fixing.job.id, parked.job.id),
    waiting: [parked, alsoParked].map((one) => ({ job_id: one.job.id, title: one.job.title })),
  };
  const all = [parked, fixing, alsoParked].map((one) => withBreakages(() => [claim], one));
  return holding("breakage/fixed-elsewhere", "A Check failed on a test another Job is already fixing", all, {
    opens: parked.job.id,
  });
}

export const s130FixedElsewhere: Scenario = fixedElsewhere();
