// # The board Pulse draws
//
// `PulseView` is the draft shape (`draft/pulse.ts`) and this turns it into the
// rows the panel takes. **The seam is deliberate**: the draft is derived from
// today's wire, and the panel knows nothing about either.

import type { Figure, PulseLogRow, PulseReading, PulseWorktreeRow } from "@armada/components";
import type { JobDetail as JobWhole, JobExamined, Recorded, StepDetail } from "@armada/protocol";
import type { PulseLog, PulseProcess, PulseView } from "./draft/pulse";
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
 * **A Judge's brief is named by the Job and weighed by the reading.**
 * `get_job` names each one a step's criteria were asked with, and the host
 * opens it by that path; `logRowsOf` joins the two. `whole` and `places` are
 * optional because the job sheet passes neither: a brief draws no `Open` and a
 * transcript its Drone's id. A size's age is off its own `measured_at`.
 */
export function pulseReadingOf(
  view: PulseView,
  examined: JobExamined | null,
  whole?: JobWhole | null,
  now?: number,
  places: DronePlaces = new Map(),
): PulseReading {
  const one = view.worktrees.length === 1;
  return {
    held: view.held,
    readAt: view.read_at,
    processes: view.processes.map((process) => ({
      pid: process.pid,
      command: process.command,
      // Whose it is: its Drone, where Fleet names one (23.9), else its checkout.
      owner: (process.drone === null ? undefined : places.get(process.drone)) ?? process.owner,
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
      ...(one ? { ...standing(view.held, examined, view.processes), open: "worktree" as const } : { state: ON_DISK }),
    })),
    logs: logRowsOf(view.logs, whole ?? null, places),
  };
}

/**
 * Fleet's files, then the briefs, each once.
 *
 * **A brief is on both reads, and `get_job` names it.** Its row takes the step
 * and criterion and the `Open` from the Job, because main opens a brief only by
 * a path the Job named; the reading adds what it weighs and whether it is held
 * open. Two paths that are equal are one file, since both are relative to
 * `records_root`. A brief only the reading lists — the board drawn without the
 * Job, or a brief kept before its verdict landed — is named by its file and
 * offers no `Open`, for the same reason. **Every brief row carries its path**,
 * so a press reads it in the panel off `get_brief` (protocol 21.11).
 *
 * **A transcript opens by its own path**, which main checks against the
 * reading it holds, and is named by where its Drone worked (`dronePlacesOf`).
 */
function logRowsOf(logs: readonly PulseLog[], whole: JobWhole | null, places: DronePlaces): PulseLogRow[] {
  const read = new Map(logs.filter((one) => one.kind === "brief").map((one) => [one.path, one]));
  const named = briefsOf(whole).map(({ path, row }) => {
    const log = read.get(path);
    read.delete(path);
    return log === undefined ? row : { ...row, ...weighed(log), writing: log.writing };
  });
  const files = logs.filter((one) => one.kind !== "brief").map((one) => fileRowOf(one, places));
  return [...files, ...named, ...[...read.values()].map((one) => fileRowOf(one, places))];
}

/** One file as the reading lists it. */
function fileRowOf(log: PulseLog, places: DronePlaces): PulseLogRow {
  const row = { kind: log.kind, owner: log.owner, ...weighed(log), writing: log.writing };
  // The two a panel can read live: the Job's log on its socket, a transcript
  // on the observe socket's rows for its Drone. A brief is read once, whole.
  if (log.kind === "job" && log.owner === null) return { ...row, path: log.path, open: "log" };
  if (log.kind === "transcript") {
    const drone = stem(log.path);
    const open = { kept: log.path, what: "transcript" } as const;
    return { ...row, about: places.get(drone) ?? drone, path: log.path, open };
  }
  return { ...row, about: stem(log.path), path: log.path };
}

/** What the file weighs, where Fleet measured it. Absent is never zero. */
function weighed(log: PulseLog): { bytes?: number } {
  return log.bytes === undefined ? {} : { bytes: log.bytes };
}

/**
 * A file's name without its directory or its extension: a transcript's Drone
 * id, a brief's step, run and criterion.
 */
function stem(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/**
 * Where each Drone worked, by its id — what a transcript row is named by:
 * `implement`, `implement · T5`, or `implement · run 2`.
 */
export type DronePlaces = ReadonlyMap<string, string>;

/**
 * Each Drone's step, off the Job's history, and its task where a plan names one.
 *
 * **The history is already in hand**: it opens with the Job, and each
 * `drone_spawned` row names the step a Drone arrived on. Nothing is fetched
 * for a label. **A task wins where there is one**; Fleet runs one Drone per
 * step and names no task (`list_job_drones`), so two Drones on one step are
 * told apart by the order they ran in, never by their ids.
 */
export function dronePlacesOf(
  history: readonly Recorded[] | undefined,
  tasks: readonly { id: string; drone_id?: string }[] = [],
  listed: readonly { drone: string; step: string; task?: string | undefined }[] = [],
): DronePlaces {
  const taskOf = new Map(
    tasks.flatMap((task) => (task.drone_id === undefined ? [] : [[task.drone_id, task.id] as const])),
  );
  const spawned = [...(history ?? [])]
    .sort((a, b) => a.seq - b.seq)
    .flatMap((one) =>
      one.moved.kind === "drone" && one.moved.presence === "drone_spawned"
        ? [{ step: one.moved.step_id, drone: one.moved.drone_id }]
        : [],
    );
  const onStep = new Map<string, number>();
  for (const one of spawned) onStep.set(one.step, (onStep.get(one.step) ?? 0) + 1);
  const ran = new Map<string, number>();
  const places = new Map<string, string>();
  for (const { step, drone } of spawned) {
    if (places.has(drone)) continue;
    const run = (ran.get(step) ?? 0) + 1;
    ran.set(step, run);
    const task = taskOf.get(drone);
    const several = (onStep.get(step) ?? 0) > 1;
    places.set(drone, task !== undefined ? `${step} · ${task}` : several ? `${step} · run ${run}` : step);
  }
  // A Drone beside the Job's kept one is on no history row (23.9): the Drones
  // listed name its step and task.
  for (const one of listed) {
    if (!places.has(one.drone) && one.task !== undefined) places.set(one.drone, `${one.step} · ${one.task}`);
  }
  return places;
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
function standing(
  held: string,
  examined: JobExamined | null,
  processes: readonly PulseProcess[],
): Pick<PulseWorktreeRow, "state" | "wrong" | "working"> {
  const look = examined?.looks.find((one) => one.asked === "worktree");
  if (look?.found === "not_working") return { state: "gone", wrong: true };
  if (look?.found === "cannot_tell") return { state: "could not be read" };
  // Every live Drone works the Job's one copy (23.9), each a recorded row.
  const drones = Math.max(processes.filter((one) => one.recorded).length, 1);
  if (held === "running") return { state: drones === 1 ? "1 drone working" : `${drones} drones working`, working: true };
  return { state: "no drone working" };
}

/**
 * One row per brief the Judge was asked with, across every step and run, and
 * one per gaming check's brief, which main opens on the same rule.
 *
 * **Unweighed here, and not `being written`.** What it weighs and whether it
 * is held open are the reading's, which `logRowsOf` lays over these.
 */
function briefsOf(whole: JobWhole | null): { path: string; row: PulseLogRow }[] {
  const seen = new Set<string>();
  const rows: { path: string; row: PulseLogRow }[] = [];
  for (const step of ordered(whole)) {
    for (const judged of step.judged) {
      const path = judged.brief_path;
      if (path === undefined || seen.has(path)) continue;
      seen.add(path);
      rows.push({
        path,
        row: {
          kind: "brief",
          owner: null,
          about: `${step.step_id} · ${judged.criterion_id}`,
          path,
          writing: false,
          open: { kept: path, what: "brief" },
        },
      });
    }
    for (const flag of step.flagged) {
      const path = flag.brief_path;
      if (path === undefined || seen.has(path)) continue;
      seen.add(path);
      const about = `${step.step_id} · ${GAMING_CHECK}`;
      const open = { kept: path, what: "brief" } as const;
      rows.push({ path, row: { kind: "brief", owner: null, about, path, writing: false, open } });
    }
  }
  return rows;
}

/** What a gaming check's brief is about, beside the step it was asked on. */
const GAMING_CHECK = "gaming check";

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
  if (view !== null) working.push(dronesFigure(view.held, view.processes));
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
 * **Every live Drone's process is a recorded row** (23.9), so the count is
 * those rows; `held` is Fleet's reading of the Job's kept Drone, and `gone`
 * and `replaced` are Fleet believing it is running when it is not — loud at
 * any status, and nought, so the detail is the whole of what tells them apart.
 */
function dronesFigure(held: string, processes: readonly PulseProcess[]): Figure {
  if (held === "gone") return { label: DRONES, value: "0", detail: NOTHING_AT_THAT_PID, wrong: true };
  if (held === "replaced") return { label: DRONES, value: "0", detail: SOMEBODY_ELSE, wrong: true };
  // Words, and marked as such: `could not be read` in the figure's own mono is
  // a sentence dressed as a number.
  if (held === "unreadable") return { label: DRONES, value: "could not be read", words: true };
  const recorded = processes.filter((one) => one.recorded).length;
  return { label: DRONES, value: held === "running" ? `${Math.max(recorded, 1)}` : `${recorded}` };
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
