// # The board Pulse draws
//
// `PulseView` is the draft shape (`draft/pulse.ts`) and this turns it into the
// rows the panel takes. **The seam is deliberate**: the draft is derived from
// today's wire, and the panel knows nothing about either.

import type { Figure, PulseLogRow, PulseReading, PulseWorktreeRow } from "@armada/components";
import type { JobDetail as JobWhole, JobExamined, StepDetail } from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
import { span } from "./duration";
import { ordered, spent } from "./facts";
import { cap } from "./RaiseCap";
import { checksOf, isRunning } from "./gates";

/**
 * The reading, as the board draws it.
 *
 * **The look is applied to one worktree and never to several.** Fleet's
 * `worktree` look asks about the Job's own checkout; drawn against a list it
 * would be a finding about members nothing looked at. So is `Open`: the host
 * opens a worktree by the Job's id, which names one checkout.
 *
 * **The Judge's briefs come off the Job, not the reading.** `get_job` names
 * each one a step's criteria were asked with, and the host opens it by that
 * path. `whole` is optional because the job sheet draws this board from a
 * caller that does not pass it; that board lists the Job's own log alone.
 * A size's age is off its own `measured_at`: Fleet keeps a size between reads.
 */
export function pulseReadingOf(
  view: PulseView,
  examined: JobExamined | null,
  whole?: JobWhole | null,
  now?: number,
): PulseReading {
  const one = view.worktrees.length === 1;
  return {
    held: view.held,
    readAt: view.read_at,
    processes: view.processes.map((process) => ({
      pid: process.pid,
      command: process.command,
      owner: process.owner,
      cpuPercent: process.cpu_percent,
      memoryBytes: process.memory_bytes,
      runningFor: process.running_for,
      recorded: process.recorded,
    })),
    worktrees: view.worktrees.map((worktree) => ({
      branch: worktree.branch,
      path: worktree.path,
      ...(worktree.bytes === undefined ? {} : { bytes: worktree.bytes }),
      ...sizeAge(worktree.measured_at, now),
      ...(one ? { ...standing(view.held, examined), open: "worktree" as const } : { state: ON_DISK }),
    })),
    logs: [
      ...view.logs.map((log) => ({
        kind: log.kind,
        owner: log.owner,
        ...(log.bytes === undefined ? {} : { bytes: log.bytes }),
        writing: log.writing,
        ...(log.kind === "job" && log.owner === null ? { open: "log" as const } : {}),
      })),
      ...briefsOf(whole ?? null),
    ],
  };
}

function sizeAge(measuredAt: string | undefined, now: number | undefined): { age?: string } {
  const age = measuredAt === undefined || now === undefined ? null : span(measuredAt, now);
  return age === null ? {} : { age };
}

/** A checkout nobody found anything wrong with, and no Drone is in. */
const ON_DISK = "on disk";

/**
 * What the Job's one checkout is doing: what the look found wrong with it,
 * or whether its Drone is in it.
 *
 * **One Drone per Job today**, so `held` answers for the checkout.
 */
function standing(held: string, examined: JobExamined | null): Pick<PulseWorktreeRow, "state" | "wrong" | "working"> {
  const look = examined?.looks.find((one) => one.asked === "worktree");
  if (look?.found === "not_working") return { state: "gone", wrong: true };
  if (look?.found === "cannot_tell") return { state: "could not be read" };
  if (held === "running") return { state: "1 drone working", working: true };
  return { state: "no drone working" };
}

/**
 * One row per brief the Judge was asked with, across every step and run.
 *
 * **Unweighed, and never `being written`.** Fleet names the path once the
 * brief is kept, so a listed brief is finished; what it weighs is not served.
 */
function briefsOf(whole: JobWhole | null): PulseLogRow[] {
  const seen = new Set<string>();
  const rows: PulseLogRow[] = [];
  for (const step of ordered(whole)) {
    for (const judged of step.judged) {
      const path = judged.brief_path;
      if (path === undefined || seen.has(path)) continue;
      seen.add(path);
      rows.push({
        kind: "brief",
        owner: null,
        about: `${step.step_id} · ${judged.criterion_id}`,
        writing: false,
        open: { kept: path, what: "brief" },
      });
    }
  }
  return rows;
}

/**
 * What this Job is running and what it is taking, over the board.
 *
 * **Drones come off the reading and the rest do not.** A Drone running is a
 * process on this machine, which only a look at the machine answers; the rest
 * are on `GET /jobs/:job_id`, there whether or not anybody has looked.
 *
 * **The verb is in the label and the value is a figure**, the design board's
 * own strip. `Judges` over `none out` is a phrase the eye lands on; `Judges
 * running` over `0` is a reading, and the bare `0` is no gap because the label
 * already says what is counted.
 */
export function pulseFiguresOf(view: PulseView | null, whole: JobWhole | null, caps?: CapPresses): Figure[] {
  const steps = ordered(whole);
  const working: Figure[] = [];
  if (view !== null) working.push(dronesFigure(view.held));
  working.push({ label: "Checks running", value: `${checksRunning(steps)}` });
  working.push({ label: "Judges running", value: `${judgesRunning(steps)}` });
  const taking: Figure[] = [];
  const spend = spendFigure(whole);
  if (spend !== undefined) {
    taking.push(caps === undefined ? spend : { ...spend, onPress: caps.cost, pressLabel: CHANGE_COST_CAP });
  }
  const turns = turnsFigure(whole);
  if (turns !== undefined) {
    taking.push(caps === undefined ? turns : { ...turns, onPress: caps.turns, pressLabel: CHANGE_TURN_CAP });
  }
  if (view !== null) taking.push({ label: "Processes", value: `${view.processes.length}` });
  // The rule lands on the first cost there is, whichever survives: a Fleet that
  // does not price has no spend, and it still has to fall between the groups.
  const [first, ...rest] = taking;
  return first === undefined ? working : [...working, { ...first, apart: true }, ...rest];
}

/**
 * Where Spend and Turns go when pressed: the job's Settings, on the cap each
 * one reads. **The caller's, and optional**, because only a destination with
 * a Settings tab beside it has somewhere to send them — the overview sheet
 * draws the same band and leaves them figures.
 */
export type CapPresses = { cost: () => void; turns: () => void };

/** The tooltips on the two pressable figures. What the press does, not what the figure is. */
const CHANGE_COST_CAP = "Change the cost cap in Settings";
const CHANGE_TURN_CAP = "Change the turn cap in Settings";

/**
 * How many Drones are up, and whether that is a fault.
 *
 * **One Drone per Job today.** `held` is Fleet's reading of the one process it
 * recorded, so the count is nought or one; `gone` and `replaced` are Fleet
 * believing something is running that is not, which is loud at any status —
 * and both count nought, so the detail is the whole of what tells them apart.
 */
function dronesFigure(held: string): Figure {
  if (held === "gone") return { label: DRONES, value: "0", detail: NOTHING_AT_THAT_PID, wrong: true };
  if (held === "replaced") return { label: DRONES, value: "0", detail: SOMEBODY_ELSE, wrong: true };
  // Words, and marked as such: `could not be read` in the figure's own mono is
  // a sentence dressed as a number.
  if (held === "unreadable") return { label: DRONES, value: "could not be read", words: true };
  return { label: DRONES, value: held === "running" ? "1" : "0" };
}

/** The band's word for the Drone count, spelled once for the four arms above. */
const DRONES = "Drones running";

/** Fleet recorded a pid and nothing holds it. */
const NOTHING_AT_THAT_PID = "fleet recorded one";

/** The pid came round and something else has it. */
const SOMEBODY_ELSE = "that pid is another process";

/** Checks the gate has started and not finished, across every step. */
export function checksRunning(steps: readonly StepDetail[]): number {
  return steps.reduce((count, step) => count + checksOf(step).filter(isRunning).length, 0);
}

/** Judge calls out right now, across every step. `StepDetail.judging`. */
export function judgesRunning(steps: readonly StepDetail[]): number {
  return steps.filter((step) => step.judging !== undefined).length;
}

/**
 * What the Job has spent against what it may spend.
 *
 * **The cap is under the figure and no longer beside it.** It is still there
 * for the reason it always was — the number a person decides a raise against
 * has to be with the number they are deciding about — but on the band a cap on
 * the same line doubled the figure's length and stopped it reading as one.
 *
 * **The spend is hedged and the cap is not.** A cost is derived from list
 * prices and wears the tilde `spent` gives it; a ceiling is a setting somebody
 * chose, and `~$3.00` would make the limit read as an estimate too.
 */
export function spendFigure(whole: JobWhole | null): Figure | undefined {
  const spend = whole?.spend;
  if (spend === undefined) return undefined;
  return {
    label: "Spend",
    value: spent(spend.cost_micros, spend.unpriced),
    detail: `of ${cap(spend.cost_cap_micros)}`,
  };
}

/**
 * Turns taken, with the turn cap under them. `turnsTaken` is the panel's one
 * line and this is the band's two — the same two numbers, split where the band
 * wants the figure alone.
 */
export function turnsFigure(whole: JobWhole | null): Figure | undefined {
  const spend = whole?.spend;
  return spend === undefined
    ? undefined
    : { label: "Turns", value: `${spend.turns}`, detail: `of ${spend.turn_cap}` };
}
