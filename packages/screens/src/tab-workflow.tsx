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

import {
  DroneBrief,
  DroneMessageBox,
  HoldButton,
  Tabs,
  Tooltip,
  WorkflowCanvas,
  WorkflowDrone,
  WorkflowInspector,
  WorkflowStacked,
} from "@armada/components";
import { useLayoutEffect, useRef, useState } from "react";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActingAct } from "./pending";
import { TAB_LABEL } from "./detail-tabs";
import { droneViewsOf, type DroneView } from "./draft/drone";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { elapsedSince } from "./duration";
import { DRONE_SAYS, droneTurnsOf, stepOf } from "./tab-drones-read";
import { ACT_LABEL, HOLD_LABEL, HOLD_SAID } from "./copy";
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
  onRedirect,
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
  // The Drone read beside the step panel, and what is typed to it. Cleared
  // with the step, so a Drone from another step never sits beside this one.
  const [drone, setDrone] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const openStep = (id: string | null) => {
    setOpen(id);
    setDrone(null);
  };
  // How much of the canvas's right side the open panel covers, so the canvas
  // can slide the step being read clear of it. Measured, because the panel's
  // width is a token and its gutter the layer's, and neither reaches here.
  const frame = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const [covered, setCovered] = useState(0);
  const beside = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    // The leftmost of the panels open over it: the step's, or the Drone's beside it.
    const over = beside.current?.getBoundingClientRect() ?? panel.current?.getBoundingClientRect();
    const under = frame.current?.getBoundingClientRect();
    setCovered(over === undefined || under === undefined ? 0 : Math.max(0, under.right - over.left));
  }, [open, drone, view, narrow]);

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

  // Every Drone that worked the step that is open, running or not, and the one
  // read beside it (owner, 29 Sep 2026). Running first, then in task order.
  // **From the plan's tasks**, as the Drones tab reads them where the draft
  // holds none: Fleet serves which Drone is on a task, and not its transcript.
  const now = Date.now();
  const drones = droneViewsOf(groups);
  const openStepId = whole.steps.find((step) => stepNodeId(step.step_id) === open)?.step_id;
  const here = drones
    .filter((one) => one.step === openStepId)
    .sort((a, b) => Number(b.state === "running") - Number(a.state === "running"));
  const labelOf = (one: DroneView): string => `Drone on ${one.task}`;
  const ranFor = (one: DroneView): string | undefined =>
    one.ended_at !== undefined
      ? elapsedSince(one.since, one.ended_at)
      : one.state === "running"
        ? elapsedSince(one.since, now)
        : undefined;
  // The Drone machine's four states on the step machine's marks: a finished
  // Drone advanced its task, and a failed or killed one stopped.
  const activityOf = (one: DroneView) =>
    one.state === "running"
      ? ("running" as const)
      : one.state === "done"
        ? ("advanced" as const)
        : ("stopped" as const);
  const saysOf = (one: DroneView): string =>
    [one.task, ranFor(one), ...spentOf(one)].filter((part) => part !== undefined).join(" · ");
  const droneRead = here.find((one) => one.id === drone);

  // Nothing until a press, and then **Helm's dock**: the full height of the
  // window under the title bar, held off its edges, over whatever is beneath
  // (owner, 29 Sep 2026: *all of our panels open to the full height of the
  // app. This one should be no different*). `screens.css` says where.
  const layer =
    reading === undefined ? null : (
      <div className="armada-workflow-tab__inspector-layer">
        <div className="armada-workflow-tab__inspector" ref={panel}>
          <WorkflowInspector
            {...reading}
            onClose={() => openStep(null)}
            // The redirect box went (owner, 29 Sep 2026, `losq`): a person
            // never knows which Drone to message. The running Drones take its
            // place, and a press opens one beside this panel (`hzj4`).
            running={{
              rows: here.map((one) => ({
                id: one.id,
                label: labelOf(one),
                activity: activityOf(one),
                said: DRONE_SAYS[one.state].toLowerCase(),
                says: saysOf(one),
                ...(one.id === drone ? { open: true } : {}),
              })),
              onOpen: (id) => {
                setDrone(id === drone ? null : id);
                setMessage("");
              },
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
    );

  // The Drone, beside the step panel: the Drones tab's own reading, in the
  // same frame, one panel to the left — the file diff beside Plan's task
  // panel is the arrangement (owner's experiment, 29 Sep 2026).
  const droneLayer =
    reading === undefined || droneRead === undefined ? null : (
      <div className="armada-workflow-tab__drone-layer">
        <div className="armada-workflow-tab__drone" ref={beside}>
          <WorkflowDrone
            title={labelOf(droneRead)}
            subtitle={[
              stepOf(whole, droneRead.step).label,
              DRONE_SAYS[droneRead.state].toLowerCase(),
              saysOf(droneRead),
            ].join(" · ")}
            turns={
              droneRead.transcript === undefined
                ? []
                : droneTurnsOf(droneRead.transcript, (lines) => <DroneBrief lines={lines} flat />, droneRead.thoughts)
            }
            live={droneRead.state === "running"}
            emptyNote={
              droneRead.transcript === undefined
                ? "Fleet does not serve one Drone's transcript yet."
                : "This Drone has written nothing yet."
            }
            {...(droneRead.state !== "running"
              ? {}
              : {
                  controls: (
                    <HoldButton
                      askLabel={ACT_LABEL.kill_drone}
                      description={HOLD_SAID.kill_drone}
                      disabled={steering.act === undefined || (acting && actingAct !== "kill_drone")}
                      pending={acting && actingAct === "kill_drone"}
                      onAsk={() => onAct("kill_drone", job.id)}
                      onCommit={() => onActHeld("kill_drone", job.id)}
                    >
                      {HOLD_LABEL.kill_drone}
                    </HoldButton>
                  ),
                  footer: (
                    <DroneMessageBox
                      value={message}
                      onChange={setMessage}
                      onSend={() => {
                        onRedirect(job.id, message);
                        setMessage("");
                      }}
                      disabled={steering.act === undefined}
                      disabledReason="No Drone is on this Job, so there is nothing to redirect."
                      {...(steering.sent === undefined ? {} : { waiting: steering.sent })}
                    />
                  ),
                })}
            onClose={() => setDrone(null)}
          />
        </div>
      </div>
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
              <div className="armada-workflow-tab__canvas" ref={frame}>
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
                  keepsClear={open === null ? null : { id: open, right: covered }}
                />
              </div>
            </div>
          ) : (
            <WorkflowStacked label={label} rows={run.rows} />
          )}
        </div>

        {layer}
        {droneLayer}
      </div>
    </div>
  );
}
