// Plan — how the work is split, read before anybody approves it.
//
// **The split is the reading, not the task list.** Job 3's plan ran past a
// Judge that had refused it because nothing on screen said which tasks ran
// together, which agent each got, or what would be run once they stopped. So
// the card is the group, the boundary is under it, and what a Drone will be
// told is one press away.
//
// **Two views of one plan, Graph and List** (owner, 25 Sep 2026). The graph
// came off Workflow — `plan-canvas.ts` places it — and the list is this board.
//
// This file places one reading. The review itself — the Graph and List, the
// panels and every act on the plan — is `plan-review.tsx`, which Overview's
// gate mounts too. What the board says is `plan-board.ts` over
// `tab-plan-read.ts`, the lead is `plan-lead.tsx`, an ask is
// `tab-plan-ask.tsx`, and what it looks like is `PlanBoard`.

import { JudgeRefusal, SkeletonText } from "@armada/components";
import { useEffect } from "react";

import { TAB_LABEL } from "./detail-tabs";
import { Eyebrow } from "./regions";
import { PlanLead } from "./plan-lead";
import { usePlanReview, type PlanReviewProps } from "./plan-review";
import { criteriaOf, revisionsOf } from "./tab-plan-read";
import { WavePlan } from "./wave-plan";
import type { WaveRegionProps } from "./tab-wave";
import type { HeldAct } from "./Acts";
import type { PlanRevisionView } from "./draft/revision";

/**
 * The review's own props — `PlanReview`'s, which Overview's gate takes too —
 * and what only this destination draws around it.
 */
export type PlanTabProps = PlanReviewProps & {
  /**
   * The wave this Job dispatched, as the region draws it. **The same reading
   * Overview takes**, so the toggle between the graph and the list does not
   * reset on the way between the two destinations.
   */
  wave: WaveRegionProps;
  /** Held, never pressed. What Drop from the wave sends, on that Job. */
  onActHeld: (act: HeldAct, jobId: string) => void;
  /** Hold the patch's read open while this destination is — Overview's own call. */
  onReadDiff?: (jobId: string | null) => void;
  /**
   * Whether this Job's own read has yet to answer. The plan stands in with
   * its shape until then — "records no plan" is an answer, and nothing has
   * been asked yet.
   */
  reading?: boolean;
};

/** What an ask came to, in the tense the answer earns. */
function answerSaid(revision: PlanRevisionView): string {
  switch (revision.answer) {
    case "asked":
      return "Asked. The Drone has not answered yet.";
    case "taken":
      return "The Drone took it, and the plan below is the one it wrote after.";
    case "refused":
      return "Refused. The plan below is the one the Drone recorded, unchanged.";
  }
}

/** What stops running if the ask stands. Nothing where nothing fell out. */
function droppedSaid(revision: PlanRevisionView): string | undefined {
  if (revision.dropped.length === 0) return undefined;
  return `It drops ${revision.dropped.map((one) => one.spec).join(", ")}.`;
}

/**
 * What the ask reached, so a refusal is read against the plan rather than
 * against the whole Job. **One task out of eight is the finding** — a person
 * looking at a refusal needs to know the other seven stand.
 */
function reachSaid(revision: PlanRevisionView, tasks: number): string | undefined {
  if (revision.task === undefined) return undefined;
  const rest = tasks - 1;
  if (rest <= 0) return `${revision.task} is the only task the ask touched.`;
  return `${revision.task} is the one task the ask touched. The other ${rest} stand as the Drone wrote them.`;
}

/**
 * Every change asked of the plan's Drone, with the answer under each.
 *
 * **The refusal is `JudgeRefusal` and not a card of its own** (`#1530`): a
 * Drone refusing a revision is the same record as a Judge refusing a step, and
 * a second shape for one record is a record that can be renamed in one place.
 */
function Revisions({
  revisions,
  tasks,
}: {
  revisions: readonly PlanRevisionView[];
  tasks: number;
}) {
  if (revisions.length === 0) return null;
  return (
    <section
      className="armada-detail-tab__region"
      aria-label="What you asked the plan's Drone to change"
    >
      <Eyebrow>What you asked the plan&apos;s Drone to change</Eyebrow>
      <ul className="armada-plan-tab__revisions">
        {revisions.map((revision) => {
          const reach = reachSaid(revision, tasks);
          const dropped = droppedSaid(revision);
          return (
            <li key={`${revision.at}-${revision.task ?? revision.group ?? revision.kind}`}>
              <p className="armada-plan-tab__asked">
                {revision.says}
                {dropped === undefined ? null : (
                  <span className="armada-plan-tab__dropped"> {dropped}</span>
                )}
              </p>
              <p className="armada-plan-tab__answered" data-answer={revision.answer}>
                {answerSaid(revision)}
              </p>
              {revision.refusal === undefined ? null : (
                <JudgeRefusal
                  {...(revision.refusal.criterion === undefined
                    ? {}
                    : { heading: `Refused — ${revision.refusal.criterion}` })}
                  finding={{
                    ...(revision.refusal.expected === undefined
                      ? {}
                      : { expected: revision.refusal.expected }),
                    ...(revision.refusal.produced === undefined
                      ? {}
                      : { produced: revision.refusal.produced }),
                    ...(revision.refusal.consequence === undefined
                      ? {}
                      : { consequence: revision.refusal.consequence }),
                  }}
                />
              )}
              {/* Outside the refusal block deliberately: `JudgeRefusal.reading`
                  hangs a tooltip about panel size off its own label, and this
                  sentence is about the plan rather than about a panel. */}
              {reach === undefined ? null : (
                <p className="armada-plan-tab__reach">{reach}</p>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

export function PlanTab({
  job,
  whole,
  wave,
  draft,
  floor,
  onActHeld,
  onReadDiff,
  reading = false,
  ...review
}: PlanTabProps) {
  // The patch, read while the destination is open — Overview's own effect,
  // since leaving Overview closes that read.
  useEffect(() => {
    if (onReadDiff === undefined) return;
    onReadDiff(job.id);
    return () => onReadDiff(null);
  }, [job.id]);

  // **The review is `PlanReview`'s**, the same call Overview's gate makes, so
  // the acts on a plan are one set wherever it is read. This destination
  // places its three pieces: the gate in the lead, the plan under the wave,
  // the panels outside the box that scrolls.
  const { step, groups, gate, region, layers } = usePlanReview({
    job,
    whole,
    onActHeld,
    ...(draft === undefined ? {} : { draft }),
    floor,
    ...review,
  });
  const revisions = revisionsOf(whole, draft, step);

  return (
    /* The sheet is the panel's sibling and not its child: the panel is the box
       that scrolls, and a layer positioned inside it slides out of the window
       with the reading — the defect `.armada-screen__detail` already carries
       the whole argument for. */
    <>
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.plan}>
      {/* Why the plan was formed, first — the owner's call of 28 Sep 2026.
          Everything under it is how the work was split to meet it. */}
      <PlanLead criteria={criteriaOf(whole, draft)} gate={gate} reading={reading} />
      {/* What the split became, above the plan that drew it: the wave is what
          a person came to this destination to read on a Job that dispatched
          one, and the task board underneath is how it was written. #1544. */}
      <WavePlan
        {...wave}
        {...(step === undefined ? {} : { planStep: step })}
        floor={floor}
        onDropFromWave={(jobId) => onActHeld("kill_job", jobId)}
      />
      {/* No plan recorded, or none yet, draws nothing: an empty slot stays empty. */}
      {region === undefined && reading ? <SkeletonText /> : region}
      {/* Above the criteria and the gate, because a refusal is the answer to
          the last thing a person did and the gate is the next thing they will
          do. */}
      <Revisions revisions={revisions} tasks={groups.flatMap((one) => one.tasks).length} />
    </div>
    {layers}
    </>
  );
}
