// Drones — every Drone this Job has used, and any one of them read whole.
//
// **The owner's, 29 Sep 2026**: *"a drones tab on the job page … running
// Drones, drones that are completed or have been killed, and I can peek into
// the entire transcript for that drone."* Every Drone Fleet lists for the Job
// (`list_job_drones`), running, finished, failed or killed, each read out of
// the Job's turns by its own id. One Drone per task is the redesign (his
// decision of 22 Sep 2026), and the arc's draft lists those instead.

import { useEffect, useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import {
  DRONE_ACTIVITY,
  DropdownMenu,
  DroneBrief,
  DroneMessageBox,
  HoldButton,
  JobDrones,
  SkeletonText,
  StepActivityMark,
} from "@armada/components";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import { ACT_LABEL, HOLD_LABEL, HOLD_SAID } from "./copy";
import { TAB_LABEL } from "./detail-tabs";
import { absoluteOf } from "./duration";
import type { DroneView } from "./draft/drone";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { steeringOf } from "./steering";
import {
  DRONE_SAYS,
  droneLabelOf,
  dronesFiltersOf,
  dronesUnder,
  droneTurnsOf,
  ORDER_SAYS,
  ranForOf,
  stepOf,
  TRANSCRIPT_EMPTY,
  type DronesFilter,
  type DronesOrder,
} from "./tab-drones-read";
import type { ActingAct } from "./pending";
import { droneOfTask } from "./tab-plan-read";
import { spentOf } from "./workflow-inspector";
import type { TrailProps } from "./trail";

export type DronesTabProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /**
   * Whether this Job's own read has yet to answer. With no Drone in hand the
   * tab stands in with rows, because "No Drone has run on this Job yet" before
   * the read is a wrong answer.
   */
  reading?: boolean;
  /** Every Drone the Job has had — `droneViewsOf`, or the draft's. */
  drones: readonly DroneView[];
  /**
   * Why the Job's turns are not in hand, where they are not — what an opened
   * Drone with no transcript says instead.
   */
  turnsNote?: string | undefined;
  /** The plan's groups, where the draft holds them. Absent reads the wire's. */
  groups?: readonly GroupView[];
  /** Now, injected, so a running Drone's run time moves with the header's. */
  now: number;
  floor: boolean;
  stale: boolean;
  /** An act on this Job is out, and which — the kill marks its own press. */
  acting: boolean;
  actingAct?: ActingAct | undefined;
  onRedirect: (jobId: string, instruction: string, droneId?: string) => void;
  /** The kill's ask, where a hold is not offered. */
  onAct: (act: ConfirmableAct, jobId: string, droneId?: string) => void;
  /** The kill, held. */
  onActHeld: (act: HeldAct, jobId: string, droneId?: string) => void;
  /** Workflow, with this step's panel open. The strip is `JobDetail.tsx`'s. */
  onOpenStep: (stepId: string) => void;
  /** Plan, with this task's sheet open. */
  onOpenTask: (taskId: string) => void;
  /**
   * The Drone to land on with its sheet open — a Drone opened from Plan's task
   * panel. Read once, `PlanTab`'s `opensTask` in reverse.
   */
  opensDrone?: string;
  /**
   * The way back, where a press in another destination's panel landed here,
   * and where this one's open panel is reported — `trail.ts`.
   */
  trail?: TrailProps;
};

export function DronesTab({
  job,
  whole,
  reading = false,
  drones,
  turnsNote,
  groups: givenGroups,
  now,
  floor,
  stale,
  acting,
  actingAct,
  onRedirect,
  onAct,
  onActHeld,
  onOpenStep,
  onOpenTask,
  opensDrone,
  trail,
}: DronesTabProps) {
  const [filter, setFilter] = useState<DronesFilter>("all");
  const [order, setOrder] = useState<DronesOrder>("running");
  const [openRow, setOpenRow] = useState<string | null>(opensDrone ?? null);
  const [instruction, setInstruction] = useState("");

  const groups = useMemo(
    () => givenGroups ?? (whole === null ? [] : taskGroupsOf(whole)),
    [givenGroups, whole],
  );
  const tasks = useMemo(() => new Map(groups.flatMap((group) => group.tasks).map((task) => [task.id, task])), [groups]);
  const shown = useMemo(() => dronesUnder(drones, filter, order), [drones, filter, order]);

  const labelOf = (drone: DroneView): string => {
    // A Drone on a step works no task — `droneLabelOf` names it.
    if (drone.task === undefined) return droneLabelOf(drone, whole);
    const task = tasks.get(drone.task);
    return task === undefined
      ? `Drone on ${drone.task}`
      : (droneOfTask(whole, { ...task, drone_id: drone.id })?.label ?? `Drone on ${drone.task}`);
  };
  const whereOf = (drone: DroneView): string =>
    [stepOf(whole, drone.step).label, drone.task].filter((part) => part !== undefined).join(" · ");
  const ranFor = (drone: DroneView): string | undefined => ranForOf(drone, now);
  // The sheet's where, as ways to it: the step opens its panel in Workflow and
  // the task its sheet in Plan — the Record's eyebrow act, so the app has one
  // look for a jump. A step or task this Job does not hold stays words. A
  // chevron is the trail's own separator, as on the Record, so no `·` follows
  // one.
  const whereLinksOf = (drone: DroneView) => {
    const step = stepOf(whole, drone.step);
    const task = drone.task;
    return (
      <>
        {step.labelIsAnIdentifier === true ? (
          `${step.label} · `
        ) : (
          <>
            <button type="button" className="armada-screen__eyebrow-act" onClick={() => onOpenStep(drone.step)}>
              {step.label}
              <ChevronRight size={12} strokeWidth={2} aria-hidden />
            </button>{" "}
          </>
        )}
        {task === undefined ? null : tasks.has(task) ? (
          <>
            <button type="button" className="armada-screen__eyebrow-act" onClick={() => onOpenTask(task)}>
              {task}
              <ChevronRight size={12} strokeWidth={2} aria-hidden />
            </button>{" "}
          </>
        ) : (
          `${task} · `
        )}
      </>
    );
  };

  const open = drones.find((drone) => drone.id === openRow);
  useEffect(() => trail?.onHere(open === undefined ? null : { id: open.id, label: labelOf(open) }), [open?.id]);
  const steering = steeringOf(job, whole);

  if (reading && drones.length === 0) {
    return (
      <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.drones}>
        <SkeletonText />
      </div>
    );
  }

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.drones}>
      <JobDrones
        rows={shown.map((drone) => ({
          id: drone.id,
          drone: labelOf(drone),
          where: whereOf(drone),
          state: drone.state,
          stateSays: DRONE_SAYS[drone.state],
          spent: spentOf(drone).join(" · "),
          ...(drone.since === undefined
            ? {}
            : { ranFor: ranFor(drone) ?? null, sinceExact: absoluteOf(drone.since) ?? drone.since }),
        }))}
        filters={dronesFiltersOf(drones)}
        filter={filter}
        onFilter={(id) => {
          setFilter(id as DronesFilter);
          setOpenRow(null);
        }}
        controls={
          <DropdownMenu
            align="start"
            triggerLabel={ORDER_SAYS[order]}
            entries={(["running", "task"] as const).map((one) => ({
              kind: "item" as const,
              id: one,
              label: ORDER_SAYS[one],
              selected: one === order,
            }))}
            onSelect={(id) => setOrder(id as DronesOrder)}
          />
        }
        openRow={openRow}
        onOpenRow={(id) => {
          // Jumped to, Close goes back — `trail.ts`.
          if (id === null && trail?.close !== undefined) return trail.close();
          setOpenRow(id);
          setInstruction("");
        }}
        emptyNote={drones.length === 0 ? "No Drone has run on this Job yet." : "No Drone under this filter."}
        floor={floor}
        back={trail?.back}
        {...(open === undefined
          ? {}
          : {
              reading: {
                title: labelOf(open),
                subtitle: (
                  <>
                    {whereLinksOf(open)}
                    {/* The state is the row's mark, never a word (owner, 2 Oct 2026). */}
                    <StepActivityMark
                      activity={DRONE_ACTIVITY[open.state]}
                      label={DRONE_SAYS[open.state]}
                      says={DRONE_SAYS[open.state]}
                    />{" "}
                    {[...spentOf(open), ranFor(open)].filter((one) => one !== undefined).join(" · ")}
                  </>
                ),
                turns: open.transcript === undefined ? [] : droneTurnsOf(open.transcript, (lines) => <DroneBrief lines={lines} flat />),
                live: open.state === "running",
                emptyNote: open.transcript === undefined ? (turnsNote ?? TRANSCRIPT_EMPTY) : TRANSCRIPT_EMPTY,
                // This Drone's own, by its id (#1666, 23.10): the others go on.
                // The header's kill ends the Job instead (owner, 29 Sep 2026).
                ...(open.state !== "running"
                  ? {}
                  : {
                      controls: (
                        <HoldButton
                          askLabel={ACT_LABEL.kill_drone}
                          description={HOLD_SAID.kill_drone}
                          disabled={stale || steering.act === undefined || (acting && actingAct !== "kill_drone")}
                          pending={acting && actingAct === "kill_drone"}
                          onAsk={() => onAct("kill_drone", job.id, open.id)}
                          onCommit={() => onActHeld("kill_drone", job.id, open.id)}
                        >
                          {HOLD_LABEL.kill_drone}
                        </HoldButton>
                      ),
                      footer: (
                        <DroneMessageBox
                          value={instruction}
                          onChange={setInstruction}
                          onSend={() => {
                            onRedirect(job.id, instruction, open.id);
                            setInstruction("");
                          }}
                          disabled={stale || steering.act === undefined}
                          disabledReason="No Drone is on this Job, so there is nothing to redirect."
                          {...(steering.sent === undefined ? {} : { waiting: steering.sent })}
                        />
                      ),
                    }),
              },
            })}
      />
    </div>
  );
}
