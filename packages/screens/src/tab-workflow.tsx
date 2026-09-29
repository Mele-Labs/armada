// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **The steps, and one Plan node** (owner, 25 Sep 2026). The whole plan used
// to hang off the step that wrote it — a node per group, a node per task, and
// a second edge from the step that worked each group. It is one node now,
// carrying the group count and the task count, and pressing it opens the Plan
// tab where the plan is drawn whole. The second edge went with the group
// nodes, knowingly: `plan-canvas.ts` carries what was traded away.
//
// **And nothing of the plan but that node** (owner, 28 Sep 2026). The implement
// board is gone, and with it the only route into a task from here — a task is
// opened from Plan. `docs/journeys/monitor-active-work.md` carries what that
// board still owes and where it has no destination.
//
// **Canvas by default, stacked available, at every width** (#1530). Narrow
// opens on where you are: a whole plan fitted into 768px is cards nobody can
// read, so the canvas narrows onto the step a person is on rather than
// shrinking the run. The toggle is remembered per viewer, and where it is kept
// is the caller's: this package holds no storage.

import { Tabs, WorkflowCanvas, WorkflowInspector, WorkflowStacked } from "@armada/components";
import { useState } from "react";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActingAct } from "./pending";
import { TAB_LABEL } from "./detail-tabs";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { ACT_LABEL, HOLD_LABEL, HOLD_SAID } from "./copy";
import { steeringOf } from "./steering";
import {
  PLAN_NODE_ID,
  stepNodeId,
  stepThatWorksTheGroups,
  workflowRunOf,
} from "./workflow-canvas";
import { workflowReadingOf } from "./workflow-inspector";
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
   * The plan's groups, for what the Plan node counts and for what the
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
   * Where the Plan node goes: the Plan tab. **The screen's, not this tab's** —
   * `JobDetail.tsx` owns which destination is open, and a tab that moved it
   * itself would be a second place the strip can be driven from.
   */
  onOpenPlan: () => void;
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
  onRedirect,
  onAct,
  onActHeld,
  onOpenPlan,
  opensStep,
}: WorkflowTabProps) {
  // The node a person has open. **Not the running step held in state** — that
  // moves under them as the Job advances, and a panel that changed subject
  // while somebody was reading it is the surface this screen exists to escape.
  const [open, setOpen] = useState<string | null>(
    opensStep === undefined ? null : stepNodeId(opensStep),
  );
  // Whether the canvas re-centres on the running step as the Job advances.
  // **Off until it is asked for**: it wins over the fit, and a run opened
  // centred on one card is a run with its other steps off screen.
  const [following, setFollowing] = useState(false);
  const [instruction, setInstruction] = useState("");

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
  // The steps, and the one Plan node. A press on a step opens it in the panel;
  // a press on the Plan node leaves for the Plan tab, so nothing here is
  // selected by it.
  const run = workflowRunOf({
    whole,
    groups,
    selected: open,
    onOpen: (id) => {
      if (id === PLAN_NODE_ID) {
        onOpenPlan();
        return;
      }
      setOpen(id);
    },
  });
  // **Nothing is open until a press opens it** (owner, 25 Sep 2026) — here, or
  // on the step's name in the Record's reading, which lands with it open. The panel
  // used to land on the step the Job is on, so the column beside the canvas was
  // never blank — there is no column now. The canvas has the tab's whole width
  // and this is a layer over it, so a reading nobody asked for would be a panel
  // covering the run it exists to explain.
  const reading = workflowReadingOf({ whole, groups, selected: open, groupsUnder });
  const steering = steeringOf(job, whole);
  const label = `${job.title}, as its workflow's run`;

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      {/* No `data-plan` any more: the run is a spine with at most one node
          under it, which is the height `--h-workflow-canvas` is set for. */}
      <div className="armada-workflow-tab" data-view={view} data-narrow={narrow || undefined}>
        <div className="armada-workflow-tab__surface">
          {/* Above the run rather than over it: drawn inside the canvas the
              toggle sat on top of the last step's card at every width.

              No `?` here any more. Guide 11 explained how a group gets its
              second edge; the plan's graph moved to the Plan tab on 25
              September 2026, so the guide was retired and its number with it. */}
          <div className="armada-workflow-tab__modes">{toggle}</div>
          {view === "canvas" ? (
            <div className="armada-workflow-tab__canvas">
              <WorkflowCanvas
                nodes={run.nodes}
                edges={run.edges}
                label={label}
                running={run.running}
                following={following}
                onFollowing={setFollowing}
                opensOn={run.opensOn}
              />
            </div>
          ) : (
            <WorkflowStacked label={label} rows={run.rows} />
          )}
        </div>

        {/* Nothing until a press, and then a layer over the canvas rather than
            a column beside it — the dock's own arrangement, `screens.css`. The
            canvas keeps the tab's width either way. */}
        {reading === undefined ? null : (
          <div className="armada-workflow-tab__inspector-layer">
            <div className="armada-workflow-tab__inspector">
              <WorkflowInspector
                {...reading}
                onClose={() => setOpen(null)}
                redirect={{
                  value: instruction,
                  onChange: setInstruction,
                  onSend: () => {
                    onRedirect(job.id, instruction);
                    setInstruction("");
                  },
                  drones: reading.drones,
                  disabled: steering.act === undefined,
                  disabledReason: NO_DRONE,
                  ...(steering.sent === undefined ? {} : { waiting: steering.sent }),
                }}
                {...(steering.act === undefined
                  ? {}
                  : {
                      stop: {
                        children: HOLD_LABEL.kill_drone,
                        askLabel: ACT_LABEL.kill_drone,
                        description: HOLD_SAID.kill_drone,
                        disabled: acting && actingAct !== "kill_drone",
                        pending: acting && actingAct === "kill_drone",
                        onAsk: () => onAct("kill_drone", job.id),
                        onCommit: () => onActHeld("kill_drone", job.id),
                      },
                    })}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Why the box is closed. **One Drone per Job today**, so a step with nothing on
 * it has nothing to reach — and stopping a group is stopping that Drone until
 * Fleet runs one per task.
 */
const NO_DRONE = "No Drone is on this Job, so there is nothing to redirect.";
