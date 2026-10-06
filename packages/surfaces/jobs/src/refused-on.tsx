// Why a step stopped, where a Judge refused it: each refused criterion with
// the Judge's finding. The Workflow step panel draws it over the acts Fleet
// offers (owner, Job 3, 2 Oct 2026 — the panel of a step stopped on a refusal
// he had agreed with said nothing about why).

import { CRITERION_VERDICT_JUDGE, CriterionVerdicts } from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";
import type { ReactNode } from "react";

import { panelsOf } from "./gates";

/**
 * The criteria refused on `step`'s current attempt, drawn as the Judge's
 * record draws them — `CriterionVerdicts`, with expected, produced and
 * consequence. **Only the refusals**: a met criterion is not why it stopped.
 * `undefined` where nothing was refused.
 */
export function refusedOn(step: StepDetail, criteria: JobWhole["acceptance_criteria"]): ReactNode {
  const refused = panelsOf(step, criteria).filter((panel) => panel.verdict === "not_met");
  if (refused.length === 0) return undefined;
  const not = CRITERION_VERDICT_JUDGE.not_met;
  return (
    <CriterionVerdicts
      rows={refused.map((panel) => {
        const finding = panel.refused[0];
        return {
          criterionId: panel.criterionId,
          named: "not_met",
          ...(panel.ordinal === undefined ? {} : { ordinal: panel.ordinal }),
          ...(panel.criterion === undefined ? {} : { text: panel.criterion.text }),
          ...(not?.verb == null ? {} : { verdict: not.verb }),
          ...(not?.icon == null ? {} : { icon: not.icon }),
          ...(finding?.expected === undefined ? {} : { expected: finding.expected }),
          ...(finding?.produced === undefined ? {} : { produced: finding.produced }),
          ...(finding?.consequence === undefined ? {} : { consequence: finding.consequence }),
          ...(finding?.brief_path === undefined ? {} : { briefPath: finding.brief_path }),
        };
      })}
    />
  );
}
