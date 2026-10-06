// Which Drone produced a Record row, read the way the Workflow panel reads it.

import type { JobDetail as JobWhole } from "@armada/protocol";

import { Eyebrow } from "./regions";
import type { GroupView } from "./draft/group";
import { familyOf, type LedgerRow } from "./draft/ledger";
import { droneOfTask, jobDroneOf } from "./tab-plan-read";
import { spentOf } from "./workflow-inspector";

/**
 * Which Drone produced a row, and what it spent. **The Workflow panel's own
 * reading** — `droneOfTask` for the label and `spentOf` for the figures — so
 * the two surfaces cannot name one Drone two ways. Nothing where the row names
 * no task, or the plan holds none by that id.
 */
export function DroneRead({
  row,
  detail,
  tasks,
}: {
  row: LedgerRow;
  detail: JobWhole;
  tasks: readonly GroupView["tasks"][number][];
}) {
  if (familyOf(row.kind) === "drones") return <StepDroneRead row={row} detail={detail} />;
  const taskId = row.coord?.task;
  if (row.actor !== "drone" || taskId === undefined) return null;
  const task = tasks.find((one) => one.id === taskId);
  const drone = task === undefined ? undefined : droneOfTask(detail, task);
  if (task === undefined || drone === undefined) return null;
  const spent = spentOf(task);
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>Drone</Eyebrow>
      <p className="armada-ledger__read-drone">
        {drone.label} · {task.state}
      </p>
      {spent.length === 0 ? null : <p className="armada-ledger__read-said">{spent.join(" · ")}</p>}
    </section>
  );
}

/**
 * The Drone a Drone row is about. **The Job's one Drone**, because today's wire
 * holds one per step attempt and names none on the attempt itself — the same
 * fallback `droneOfTask` takes. Its state is the attempt's; no turns or cost,
 * which the wire keeps per task and never per attempt.
 */
function StepDroneRead({ row, detail }: { row: LedgerRow; detail: JobWhole }) {
  const drone = jobDroneOf(detail);
  if (drone === undefined) return null;
  const attempt = detail.steps
    .find((one) => one.step_id === row.coord?.step)
    ?.attempts.find((one) => one.attempt === row.coord?.step_attempt);
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>Drone</Eyebrow>
      <p className="armada-ledger__read-drone">
        {drone.label}
        {/* A Drone ending says what it came to under Outcome, above. */}
        {attempt === undefined || row.kind === "drone_exited" ? null : <> · {attempt.outcome}</>}
      </p>
    </section>
  );
}
