// Drones — every Drone this Job has used, and any one of them read whole.
//
// **The owner's, 29 Sep 2026**: *"a drones tab on the job page … running
// Drones, drones that are completed or have been killed, and I can peek into
// the entire transcript for that drone."* One Drone per task is the redesign
// (`.claude/decisions/2026-09-22-its-own-agent-per-task.md`), and the list is
// read from the draft until Fleet serves it.

import { useMemo, useState } from "react";
import { DropdownMenu, DroneBrief, DroneMessageBox, JobDrones } from "@armada/components";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { absoluteOf, clock } from "./duration";
import { droneViewsOf, type DroneView } from "./draft/drone";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { steeringOf } from "./steering";
import {
  DRONE_SAYS,
  dronesFiltersOf,
  dronesUnder,
  droneTurnsOf,
  ORDER_SAYS,
  stepOf,
  type DronesFilter,
  type DronesOrder,
} from "./tab-drones-read";
import { droneOfTask } from "./tab-plan-read";
import { spentOf } from "./workflow-inspector";

export type DronesTabProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** Every Drone, where the draft holds them. Absent reads the plan's tasks. */
  drones?: readonly DroneView[];
  /** The plan's groups, where the draft holds them. Absent reads the wire's. */
  groups?: readonly GroupView[];
  floor: boolean;
  stale: boolean;
  onRedirect: (jobId: string, instruction: string) => void;
};

export function DronesTab({ job, whole, drones: given, groups: givenGroups, floor, stale, onRedirect }: DronesTabProps) {
  const [filter, setFilter] = useState<DronesFilter>("all");
  const [order, setOrder] = useState<DronesOrder>("running");
  const [openRow, setOpenRow] = useState<string | null>(null);
  const [instruction, setInstruction] = useState("");

  const groups = useMemo(
    () => givenGroups ?? (whole === null ? [] : taskGroupsOf(whole)),
    [givenGroups, whole],
  );
  const drones = useMemo(() => given ?? droneViewsOf(groups), [given, groups]);
  const tasks = useMemo(() => new Map(groups.flatMap((group) => group.tasks).map((task) => [task.id, task])), [groups]);
  const shown = useMemo(() => dronesUnder(drones, filter, order), [drones, filter, order]);

  const labelOf = (drone: DroneView): string => {
    const task = tasks.get(drone.task);
    return task === undefined
      ? `Drone on ${drone.task}`
      : (droneOfTask(whole, { ...task, drone_id: drone.id })?.label ?? `Drone on ${drone.task}`);
  };
  const whereOf = (drone: DroneView): string => `${stepOf(whole, drone.step).label} · ${drone.task}`;

  const open = drones.find((drone) => drone.id === openRow);
  const steering = steeringOf(job, whole);

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
            : { since: clock(drone.since), sinceExact: absoluteOf(drone.since) ?? drone.since }),
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
          setOpenRow(id);
          setInstruction("");
        }}
        emptyNote={drones.length === 0 ? "No Drone has run on this Job yet." : "No Drone under this filter."}
        floor={floor}
        {...(open === undefined
          ? {}
          : {
              reading: {
                title: labelOf(open),
                subtitle: [
                  whereOf(open),
                  DRONE_SAYS[open.state].toLowerCase(),
                  ...spentOf(open),
                  ...(open.since === undefined
                    ? []
                    : open.ended_at === undefined
                      ? [`since ${clock(open.since)}`]
                      : [`${clock(open.since)} to ${clock(open.ended_at)}`]),
                ].join(" · "),
                turns: open.transcript === undefined ? [] : droneTurnsOf(open.transcript, whole, (lines) => <DroneBrief lines={lines} />),
                live: open.state === "running",
                emptyNote:
                  open.transcript === undefined
                    ? "Fleet does not serve one Drone's transcript yet."
                    : "This Drone has written nothing yet.",
                // Mocked against this Drone, as the Plan task sheet is: Fleet
                // redirects the Job, not one Drone (#1536).
                ...(open.state !== "running"
                  ? {}
                  : {
                      footer: (
                        <DroneMessageBox
                          value={instruction}
                          onChange={setInstruction}
                          onSend={() => {
                            onRedirect(job.id, instruction);
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
