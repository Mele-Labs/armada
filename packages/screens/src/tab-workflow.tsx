// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **The steps, and nothing of the plan on the canvas** (owner, 25, 28 and 29
// Sep 2026). The whole plan hung off the step that wrote it until 25 Sep, then
// one Plan node did, and on 29 Sep that node went too (`nm0h`): the Workflow
// board draws none, and the step that makes or works the plan says so in its
// own panel, with a link to the Plan tab. The second edge went with the group
// nodes, knowingly: `plan-canvas.ts` carries what was traded away. A task is
// opened from Plan; `docs/journeys/monitor-active-work.md` carries what the
// retired implement board still owes.
//
// **Canvas by default, stacked available, at every width** (#1530). Narrow
// opens on where you are: a whole plan fitted into 768px is cards nobody can
// read, so the canvas narrows onto the step a person is on rather than
// shrinking the run. The toggle is remembered per viewer, and where it is kept
// is the caller's: this package holds no storage.

import { Tabs, Tooltip, WorkflowCanvas, WorkflowInspector, WorkflowStacked } from "@armada/components";
import { useEffect, useState } from "react";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActingAct } from "./pending";
import { TAB_LABEL } from "./detail-tabs";
import { droneViewsOf, type DroneView } from "./draft/drone";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { elapsedSince } from "./duration";
import { DRONE_SAYS } from "./tab-drones-read";
import type { TrailProps } from "./trail";
import { STEP_STOP } from "./copy";
import { steeringOf } from "./steering";
import { ordered } from "./facts";
import { stepNodeId, stepThatWorksTheGroups, workflowRunOf } from "./workflow-canvas";
import { spentOf, workflowReadingOf } from "./workflow-inspector";
import { WORKFLOW_VIEWS, WORKFLOW_VIEW_LABEL, type WorkflowView } from "./workflow-view";

export type WorkflowTabProps = {
  job: JobSummary;
  /** The Job whole. `null` while the read is in flight, or where it failed. */
  whole: JobWhole | null;
  /** Why there is no run to draw, where there is none. */
  absent?: string;
  /**
   * Whether the window is narrow enough that the inspector takes the whole
   * width of the tab when it opens, rather than a panel's width over one side
   * of the canvas. `useNarrow`'s answer.
   */
  narrow: boolean;
  view: WorkflowView;
  onView: (view: WorkflowView) => void;
  /**
   * The plan's groups, for what the working step counts and for what the
   * inspector says about the step that works them (`#1532`'s draft, on
   * `JobDetailProps.draft`). **Absent derives one group per task from what
   * Fleet serves**, which is thinner and never broken — `draft/group.ts`
   * carries the reasoning.
   */
  groups?: readonly GroupView[];
  /** A press is out and Fleet has not answered. */
  acting: boolean;
  actingAct?: ActingAct;
  onRedirect: (jobId: string, instruction: string) => void;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onActHeld: (act: HeldAct, jobId: string) => void;
  /**
   * Where the plan link in a step's panel goes: the Plan tab. **The screen's,
   * not this tab's** — `JobDetail.tsx` owns which destination is open, and a tab that moved it
   * itself would be a second place the strip can be driven from.
   */
  onOpenPlan: () => void;
  /**
   * Opens a Drone the step panel lists, in the Drones tab with its sheet open
   * (owner, 29 Sep 2026: a jump with a way back, not a second panel beside
   * this one). **The screen's**, as `onOpenPlan` is. Absent draws the rows as
   * facts rather than presses.
   */
  onOpenDrone?: (droneId: string) => void;
  /**
   * The way back after a jump into this tab, and where this tab's open step
   * is reported so a jump out of it can return — `trail.ts`.
   */
  trail?: TrailProps;
  /**
   * The step to land on with its panel open — a step pressed in the Record's
   * reading. Read once, when the tab opens; after that the panel is the
   * person's.
   */
  opensStep?: string;
};

export function WorkflowTab({
  job,
  whole,
  absent,
  narrow,
  view,
  onView,
  groups: given,
  acting,
  actingAct,
  onAct,
  onActHeld,
  onOpenPlan,
  onOpenDrone,
  trail,
  opensStep,
}: WorkflowTabProps) {
  // The node a person has open. **Not the running step held in state** — that
  // moves under them as the Job advances, and a panel that changed subject
  // while somebody was reading it is the surface this screen exists to escape.
  const [open, setOpen] = useState<string | null>(
    opensStep === undefined ? null : stepNodeId(opensStep),
  );
  // **And again whenever it changes**, not only at mount: Back after a jump
  // out of this tab lands on the step it left, and the screen hands that step
  // in here (`trail.ts`). Plan answers its own `opensTask` the same way.
  useEffect(() => {
    if (opensStep !== undefined) setOpen(stepNodeId(opensStep));
  }, [opensStep]);
  // Whether the canvas re-centres on the running step as the Job advances.
  // **Off until it is asked for**: it wins over the fit, and a run opened
  // centred on one card is a run with its other steps off screen.
  const [following, setFollowing] = useState(false);
  const openStep = setOpen;
  // Which step is open, told to the trail, so a jump out of it can come back
  // here with the same step open. Its id is the step's, which is what
  // `opensStep` lands on.
  const openedStep = whole?.steps.find((step) => stepNodeId(step.step_id) === open);
  useEffect(() => {
    trail?.onHere(openedStep === undefined ? null : { id: openedStep.step_id, label: openedStep.label });
    // `trail` is rebuilt by the screen every render; what matters is the step.
  }, [openedStep?.step_id]);

  // **Nothing scrolls the panel into view any more, because it is never out of
  // it.** Until 25 Sep the panel was a column that folded under the whole graph
  // at a narrow window, so a press on a task moved a reply box nobody could
  // see and an effect wrote scroll position to fix it. The panel is a layer
  // over the canvas now and sticks to the top of the destination
  // (`screens.css`), so review and reply stay one loop with no scrolling at all.

  const toggle = (
    <Tabs
      items={WORKFLOW_VIEWS.map((one) => ({ id: one, label: WORKFLOW_VIEW_LABEL[one] }))}
      value={view}
      onChange={(id) => onView(id as WorkflowView)}
    />
  );

  if (whole === null || whole.steps.length === 0) {
    return (
      <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
        <p className="armada-inside__absent" role="note">
          {absent ?? "This Job's frozen workflow has no steps."}
        </p>
      </div>
    );
  }

  const groups = given ?? taskGroupsOf(whole);
  const groupsUnder = stepThatWorksTheGroups(whole);
  // The steps. A press on one opens it in the panel.
  const run = workflowRunOf({ whole, groups, selected: open, onOpen: openStep });
  // **Nothing is open until a press opens it** (owner, 25 Sep 2026) — here, or
  // on the step's name in the Record's reading, which lands with it open. The panel
  // used to land on the step the Job is on, so the column beside the canvas was
  // never blank — there is no column now. The canvas has the tab's whole width
  // and this is a layer over it, so a reading nobody asked for would be a panel
  // covering the run it exists to explain.
  const reading = workflowReadingOf({ whole, groups, selected: open, groupsUnder, onOpenPlan });
  const steering = steeringOf(job, whole);
  const label = `${job.title}, as its workflow's run`;
  // The board's corner: which workflow this is, and which step the Job is on.
  // **No step count** — the steps are drawn right under it.
  const on = ordered(whole).find((step) => step.step_id === whole.job.current_step_id);

  // Every Drone that worked the step that is open, running or not (owner, 29
  // Sep 2026), running first, then in task order. A press opens one in the
  // Drones tab, with a way back here. **From the plan's tasks**, as the Drones
  // tab reads them where the draft holds none.
  const now = Date.now();
  const openStepId = whole.steps.find((step) => stepNodeId(step.step_id) === open)?.step_id;
  const here = droneViewsOf(groups)
    .filter((one) => one.step === openStepId)
    .sort((a, b) => Number(b.state === "running") - Number(a.state === "running"));
  const ranFor = (one: DroneView): string | undefined =>
    one.ended_at !== undefined
      ? elapsedSince(one.since, one.ended_at)
      : one.state === "running"
        ? elapsedSince(one.since, now)
        : undefined;
  // The Drone machine's four states on the step machine's marks: a finished
  // Drone advanced its task, and a failed or killed one stopped.
  const activityOf = (one: DroneView) =>
    one.state === "running" ? ("running" as const) : one.state === "done" ? ("advanced" as const) : ("stopped" as const);

  // Nothing until a press, and then **the app's own panel**: the floating
  // Sheet Record, Drones and Plan draw theirs in, over the work area and
  // dimming it (owner, 29 and 30 Sep 2026), with the way back in its head after
  // a jump here. So a second step is read by closing this one first.
  const layer =
    reading === undefined ? null : (
      <WorkflowInspector
        {...reading}
        sheet={{ back: trail?.back }}
        onClose={() => openStep(null)}
        // The redirect box went (owner, 29 Sep 2026, `losq`): a person never
        // knows which Drone to message. The step's Drones take its place.
        running={{
          rows: here.map((one) => ({
            id: one.id,
            label: `Drone on ${one.task}`,
            activity: activityOf(one),
            said: DRONE_SAYS[one.state].toLowerCase(),
            says: [one.task, ranFor(one), ...spentOf(one)].filter((part) => part !== undefined).join(" · "),
          })),
          ...(onOpenDrone === undefined ? {} : { onOpen: onOpenDrone }),
        }}
        {...(steering.act === undefined
          ? {}
          : {
              stop: {
                children: STEP_STOP.label,
                askLabel: STEP_STOP.ask,
                description: STEP_STOP.said,
                disabled: acting && actingAct !== "kill_drone",
                pending: acting && actingAct === "kill_drone",
                onAsk: () => onAct("kill_drone", job.id),
                onCommit: () => onActHeld("kill_drone", job.id),
              },
            })}
      />
    );

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      {/* The run is a spine and nothing under it. The canvas takes the height
          the destination has left, as the board draws it, and never less than
          `--h-workflow-canvas`. */}
      <div className="armada-workflow-tab" data-view={view} data-narrow={narrow || undefined}>
        <div className="armada-workflow-tab__surface">
          {/* Above the run rather than over it: drawn inside the canvas the
              toggle sat on top of the last step's card at every width.

              No `?` here any more. Guide 11 explained how a group gets its
              second edge; the plan's graph moved to the Plan tab on 25
              September 2026, so the guide was retired and its number with it. */}
          <div className="armada-workflow-tab__modes">{toggle}</div>
          {view === "canvas" ? (
            <div className="armada-workflow-tab__stage">
              <div className="armada-workflow-tab__canvas">
                <div className="armada-workflow-tab__where">
                  <Tooltip label="The workflow this Job froze">
                    <span className="armada-workflow-tab__workflow mono">{job.workflow_id}</span>
                  </Tooltip>
                  {on === undefined ? null : (
                    <>
                      <span className="armada-workflow-tab__rule" aria-hidden="true" />
                      <span className="armada-workflow-tab__on">step {on.ordinal}</span>
                    </>
                  )}
                </div>
                <WorkflowCanvas
                  nodes={run.nodes}
                  edges={run.edges}
                  label={label}
                  running={run.running}
                  following={following}
                  onFollowing={setFollowing}
                  opensOn={run.opensOn}
                  hangsFromTop
                  runsDown
                />
              </div>
            </div>
          ) : (
            <WorkflowStacked label={label} rows={run.rows} />
          )}
        </div>

        {layer}
      </div>
    </div>
  );
}
