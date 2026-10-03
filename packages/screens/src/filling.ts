// What a proposal has settled, folded onto the Job it is reading for.
//
// **Out of `proposal.ts` because main calls it** (`apps/desktop/src/main/
// arrivals.ts`), and main's build cannot read a module importing
// `@armada/components`. This one imports the wire's types and nothing else.

import type { Criterion, JobDetail, JobSummary, ProposalMoved, ProposalSettled } from "@armada/protocol";

/**
 * The Job a proposal has settled some of, and its detail.
 *
 * **One function, because a field settles in one place.** What the proposer has
 * decided is the Job becoming more complete — the owner's decision of 30 Sep
 * 2026 that a dispatched request *is* a Job is what makes that true — so a
 * settled field is folded onto the row and the detail rather than drawn from a
 * channel of its own beside them. Every reader on the Board and on the Job's
 * page then draws it with no arm for a proposal at all.
 *
 * **`movedOnto` is how a message reaches it**, on the Job `proposal.moved`
 * names: Fleet creates that Job at dispatch since 21.6 (#1714, #1716).
 */
export function filled(
  job: JobSummary,
  detail: JobDetail,
  settled: ProposalSettled | undefined,
): { job: JobSummary; detail: JobDetail } {
  if (settled === undefined) return { job, detail };
  const title = settled.title;
  // **The request is not thrown away when the title replaces it.** Until a
  // title lands the row's title *is* the request as it was typed, so the
  // moment the title changes under somebody, the words they wrote move into
  // the brief, which is where Fleet puts them when the call answers anyway. Nothing a person typed stops being
  // on screen because the proposer got further.
  const facts = title === undefined ? detail.facts : (detail.facts ?? job.title);
  const moved: JobSummary = {
    ...job,
    ...(settled.workflow_id === undefined ? {} : { workflow_id: settled.workflow_id }),
    ...(title === undefined ? {} : { title }),
    ...(settled.settings?.urgency === undefined ? {} : { urgency: settled.settings.urgency }),
    ...(settled.settings?.model === undefined ? {} : { model: settled.settings.model }),
  };
  return {
    job: moved,
    detail: {
      ...detail,
      job: moved,
      ...(facts === undefined ? {} : { facts }),
      acceptance_criteria: criteriaOf(settled.done_when) ?? detail.acceptance_criteria,
    },
  };
}

/**
 * `proposal.moved`, folded onto the Job its `job_id` names (21.6): the row,
 * and the detail where that Job is the one open. `null` where it names no row
 * held here or has settled nothing.
 *
 * **Every message carries everything settled so far**, so a fold lost to a
 * re-read comes back with the next one, and folding the same fields twice
 * changes nothing. Main's `arrivals.ts` and the mock both call this.
 */
export function movedOnto(
  jobs: readonly JobSummary[],
  open: JobDetail | undefined,
  moved: Pick<ProposalMoved, "job_id" | "proposing">,
): { jobs: JobSummary[]; detail?: JobDetail } | null {
  const settled = moved.proposing?.settled;
  const held = jobs.find((one) => one.id === moved.job_id);
  if (settled === undefined || held === undefined) return null;
  const mine = open?.job.id === held.id ? open : undefined;
  // The row alone needs no detail; a bare one stands in where the page is shut.
  const standing: JobDetail = {
    job: held,
    created_at: held.created_at,
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
  };
  const after = filled(held, mine ?? standing, settled);
  return {
    jobs: jobs.map((one) => (one.id === held.id ? after.job : one)),
    ...(mine === undefined ? {} : { detail: after.detail }),
  };
}

/**
 * The lines the proposer has written, as criteria. `undefined` where it has
 * written none, so a fold never replaces criteria with an empty list.
 *
 * **The Judge is the source, and Fleet writes the same value** — these are
 * prose, and prose is what the Judge reads. The id is the position, because the
 * record has not minted one yet and a Judge citation names a criterion by where
 * it sits.
 */
function criteriaOf(doneWhen: readonly string[] | undefined): Criterion[] | undefined {
  if (doneWhen === undefined || doneWhen.length === 0) return undefined;
  return doneWhen.map((text, at) => ({ criterion_id: String(at + 1), text, source: "judge" }));
}
