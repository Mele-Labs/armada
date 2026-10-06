// One step's proof, folded to the row the record draws for it: what it came
// to, and a short reading of the counts. The owner, 3 Oct 2026, on the record's
// one long column: *"what if there was a way to summarize each step instead of
// listing every check."* The rows still open from it.
//
// **A count is right here**: it stands where the items are not listed.

import { STEP_STATE } from "@armada/components";
import type { CheckRun as CheckRunRow, CheckRunsSummary } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { JUDGE_ROW } from "./checks";

/** The row a step with nothing but an existence check carries. `provesItOf`'s own id. */
const NOTHING_CHECKED = "nothing-checked";

/** A registry verb as a badge's first word. */
const asLabel = (verb: string): string => verb.charAt(0).toUpperCase() + verb.slice(1);

/**
 * The hue a settled step takes. `step_state` carries no badge of its own, and
 * only these two are an outcome: the rest are a step still on its way.
 */
const SETTLED: Readonly<Record<string, string | undefined>> = {
  advanced: "completed-success",
  stopped: "completed-failed",
};

/**
 * What one step came to, and its rows' counts. **The badge is the step's own
 * state, where it settled** — never a hue worked out of its rows, because a
 * step whose Judge refused one criterion can still have advanced, and a badge
 * saying refused there would say the step was. The reading says the rest.
 */
export function proofSummaryOf(
  state: StepDetail["state"],
  rows: readonly CheckRunRow[],
): CheckRunsSummary {
  const checks = rows.filter((row) => row.id !== JUDGE_ROW && row.id !== NOTHING_CHECKED);
  const judge = rows.find((row) => row.id === JUDGE_ROW);
  const nothing = rows.find((row) => row.id === NOTHING_CHECKED);
  const failed = checks.filter((row) => row.named === "failed").length;
  const passed = checks.filter((row) => row.named === "passed").length;
  const noun = (count: number) => (count === 1 ? "Check" : "Checks");
  const said: string[] = [];
  if (checks.length > 0) {
    said.push(
      failed > 0
        ? `${failed} of ${checks.length} ${noun(checks.length)} failed`
        : passed === checks.length
          ? `${passed} ${noun(passed)} passed`
          : `${passed} of ${checks.length} ${noun(checks.length)} passed`,
    );
  }
  if (judge !== undefined && typeof judge.says === "string") {
    said.push(`Judge: ${judge.says}${judge.named === "overruled" ? ", overruled by you" : ""}`);
  }
  if (nothing !== undefined && typeof nothing.says === "string") said.push(nothing.says);
  const outcome = outcomeOf(state);
  return { reading: said.join(" · "), ...(outcome === undefined ? {} : { outcome }) };
}

function outcomeOf(state: StepDetail["state"]): CheckRunsSummary["outcome"] {
  const status = SETTLED[state];
  const rendering = STEP_STATE[state];
  if (status === undefined || rendering?.verb == null || rendering.icon == null) return undefined;
  return { status, icon: rendering.icon, label: asLabel(rendering.verb) };
}
