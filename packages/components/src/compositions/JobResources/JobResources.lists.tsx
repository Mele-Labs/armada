// Pulse's three lists: the processes a Job holds, the checkouts they are in,
// and the logs being written. Beside `JobResources.tsx` rather than inside it,
// because that file is the verdict and the act and this is the reading.

import { CircleDot, Power } from "lucide-react";

import type { Artifact, JobExamined } from "@armada/protocol";
import { Button } from "../../primitives/Button/Button";
import { HoldButton } from "../../primitives/HoldButton/HoldButton";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What this Job is taking, at one instant.
 *
 * **The instant rides with the figures rather than beside them.** A process
 * can exit between the sample and the render, so a caller cannot hand over the
 * numbers without the time they were true.
 */
export type PulseReading = {
  /**
   * Fleet's reading of its recorded Drone: `running`, `gone`, `replaced`, …
   *
   * **`string`, not the wire's union.** A Fleet ahead of this build can send a
   * reading it has no sentence for, and `unknownHold` says so rather than
   * leaving the row blank.
   */
  held: string;
  /** When every figure here was read, except a worktree's size, which has its own `age`. */
  readAt: string;
  /** The recorded process and everything descended from it, that one first. */
  processes: PulseProcessRow[];
  /** One row per member where the Job has members, one otherwise. */
  worktrees: PulseWorktreeRow[];
  /** One row per log file. Empty is a Job that has written none. */
  logs: PulseLogRow[];
};

/** One process this Job holds, and which checkout it belongs to. */
export type PulseProcessRow = {
  pid: number;
  /**
   * The executable's own name — `node`, `cargo`, `git`. **Never its argument
   * vector**, which carries absolute paths and whatever a Check was invoked with.
   */
  command: string;
  /**
   * The checkout it belongs to, by branch. **`null` is a process nothing could
   * place** — with several checkouts it must not read as the first one.
   */
  owner: string | null;
  /** Share of one core. **Not capped at 100**: a process across four is real. */
  cpuPercent: number;
  memoryBytes: number;
  /** `ps`'s own spelling of how long it has been up. Rendered, never parsed. */
  runningFor: string;
  /** The process Fleet wrote down. At most one row carries it. */
  recorded: boolean;
};

/**
 * One checkout this Job holds.
 *
 * **A list, not a field.** A Job running several Drones holds one per member
 * and a Job running one holds one — the same drawing at a different count,
 * rather than a screen that changes shape the day a Job gains a member.
 */
export type PulseWorktreeRow = {
  /** What names the row. Two checkouts of one Job differ by their branch. */
  branch: string;
  path: string;
  /** What it is doing, in the caller's words — a member's state, or the Job's. */
  state: string;
  /** What it takes on disk. **Absent is a walk that ran past its bound, never zero.** */
  bytes?: number;
  /**
   * How old `bytes` is, as a phrase — `25s`. Formatted by the caller, from the
   * size's own instant: Fleet keeps a size between reads, so it is not the
   * reading's age.
   */
  age?: string;
  /** Whether the state is a fault. Draws it in `--error`. */
  wrong?: boolean;
  /** Whether a Drone is working in it now. Draws the dot in the running hue. */
  working?: boolean;
  /** What `Open` hands the host. Absent draws no control. */
  open?: Artifact;
};

/** One log file this Job is writing, or has written. */
export type PulseLogRow = {
  /** Which log it is: `job`, `transcript`, `brief`. Named on screen by `KIND`. */
  kind: string;
  /** The member it belongs to. `null` is the Job's own. */
  owner: string | null;
  /** What in the member it is about — a Judge brief's step and criterion. */
  about?: string;
  /** What `Open` hands the host. Absent draws no control. */
  open?: Artifact;
  /**
   * The file, relative to `records_root`, where a press on the row reads it in
   * a panel. Absent is a row with nothing to read in one.
   */
  path?: string;
  /** What it weighs. **Absent draws nothing**, never `0` and never a dash. */
  bytes?: number;
  /** Whether something is writing to it right now. */
  writing: boolean;
};

/**
 * The processes, or the sentence that says there are none — four of those,
 * because *no drone was expected*, *fleet believes one is running and it is
 * gone*, *the pid came round as something else* and *the probe would not run*
 * are four different things to do next.
 *
 * **Five columns**: pid, executable, whose it is, CPU, memory. The owner is a
 * column and not a suffix, because *which member is eating the machine* is
 * what a person opens this with. CPU and memory both, unlabelled on screen —
 * `%` and `MiB` tell them apart (owner, 29 Sep) — and named in a tooltip,
 * the label a person asks for by hovering (owner, 29 Sep).
 *
 * **A sixth where the host can kill one**: `onKill` draws a held kill at the
 * row's end, and holding it asks the host to confirm. Absent draws no track.
 */
export function Processes({
  reading,
  examined,
  onKill,
}: {
  reading: PulseReading;
  examined: JobExamined | null;
  onKill?: (process: PulseProcessRow) => void;
}) {
  if (reading.processes.length === 0) {
    const fault = nothingRunningIsAFault(reading.held, examined);
    // None held and nothing wrong with that: an empty slot stays empty.
    if (reading.held === "none" && !fault) return null;
    return (
      <p
        className="armada-holds__nothing"
        data-loud={fault || undefined}
      >
        {NO_PROCESS[reading.held] ?? unknownHold(reading.held)}
      </p>
    );
  }
  return (
    <ul className="armada-holds__rows" data-list="processes" data-kills={onKill === undefined ? undefined : ""}>
      {reading.processes.map((one) => (
        <li key={one.pid} className="armada-holds__row">
          <span className="armada-holds__pid">{one.pid}</span>
          <span className="armada-holds__name armada-holds__mono">{one.command}</span>
          {/* A process nothing could place says so. **Never the first worktree**,
              which with several checkouts would name the wrong member. */}
          <span className="armada-holds__whose" title={one.owner ?? undefined}>
            {one.owner ?? NOT_PLACED}
          </span>
          <Tooltip label={CPU_USAGE} asChild>
            <span className="armada-holds__weight">{cpu(one.cpuPercent)}</span>
          </Tooltip>
          <Tooltip label={MEMORY_USAGE} asChild>
            <span className="armada-holds__weight">{sized(one.memoryBytes)}</span>
          </Tooltip>
          {onKill === undefined ? null : (
            <KillHold
              label={HOLD_TO_KILL}
              asks={`Kill ${one.command}, pid ${one.pid}`}
              onKill={() => onKill(one)}
            />
          )}
        </li>
      ))}
    </ul>
  );
}

/**
 * Whether an absence is a fault or an ordinary state.
 *
 * **The examination decides where there has been one**, because whether a job
 * ought to hold a process is a question about its status and this component has
 * none. Without one, `gone` and `replaced` are loud on their own and `none` is
 * quiet — a job at its approval gate holds nothing and is right to.
 *
 * **Exported**, because `JobHoldsSummary` asks the same question and a second
 * copy would let the summary and the board disagree about one Job.
 */
export function nothingRunningIsAFault(held: string, examined: JobExamined | null): boolean {
  const look = examined?.looks.find((one) => one.asked === "process");
  if (look !== undefined) return look.found === "not_working";
  return held === "gone" || held === "replaced";
}

const NO_PROCESS: Record<string, string> = {
  none: "Fleet holds no process for this job. Nothing has been dispatched, or the drone that was here has gone.",
  running:
    "Fleet's process is alive and the process table would not read, so what it is running is unknown.",
  gone: "Fleet believes a process is running here and nothing holds that pid. Nothing is running.",
  replaced:
    "The pid fleet recorded is held by a different process. The drone that was here has gone.",
  unreadable: "The process check would not run, so nothing here can say what this job holds.",
};

/** A process no checkout claims. A real answer, and never a guess at one. */
const NOT_PLACED = "not placed";

/**
 * A reading this build has no sentence for. **The wire's own spelling, said as
 * unreadable** — never a blank, and never rounded into one of the five.
 */
function unknownHold(held: string): string {
  return `Fleet reports this job as ${held}, which this build of Bridge has no reading for.`;
}

/** What the two process figures are, said by their tooltips. */
export const CPU_USAGE = "CPU usage";
export const MEMORY_USAGE = "Memory usage";

/** The kill's name while it is held, on a row and in its tooltip. */
const HOLD_TO_KILL = "Hold to kill";

/**
 * A kill drawn as its glyph alone, confirmed by holding and then by the host's
 * dialog — the owner asked for both (29 Sep), so neither replaces the other.
 * **Under reduced motion there is no hold and the press asks**, which is the
 * same dialog one step sooner, so `onKill` answers both.
 *
 * `power`, kill's own glyph in the registry: a process stopped on purpose.
 */
export function KillHold({ label, asks, onKill }: { label: string; asks: string; onKill: () => void }) {
  return (
    <Tooltip label={label}>
      <HoldButton
        size="sm"
        icon={<Power size={16} strokeWidth={2} aria-hidden="true" />}
        askLabel={asks}
        description={`${asks}. Hold until the fill completes, then confirm.`}
        onCommit={onKill}
        onAsk={onKill}
      >
        {label}
      </HoldButton>
    </Tooltip>
  );
}

/**
 * CPU and memory across every process, for the Processes card's head —
 * `8.2%` and `384.0 MiB`, drawn with ` · ` between them. The rows are under
 * it, so not a count (owner, 29 Sep). Two figures and not one string, because
 * each carries its own tooltip. `null` where there are no processes: the
 * card's sentence says why.
 */
export function processesTotal(processes: PulseProcessRow[]): { cpu: string; memory: string } | null {
  if (processes.length === 0) return null;
  const cpuSum = processes.reduce((sum, one) => sum + one.cpuPercent, 0);
  const memorySum = processes.reduce((sum, one) => sum + one.memoryBytes, 0);
  return { cpu: cpu(cpuSum), memory: sized(memorySum) };
}

/** Share of one core, as `ps` spells it — `8.2%`. */
function cpu(percent: number): string {
  return `${percent.toFixed(1)}%`;
}

/**
 * The size on disk, for the Worktrees card's head — `1.2 GiB`. No count: the
 * rows are under it (owner, 29 Sep).
 *
 * **Nothing where any checkout went unmeasured.** A total over the rows that
 * were walked would read as the whole of it.
 */
export function worktreesTotal(worktrees: PulseWorktreeRow[]): string {
  if (worktrees.length === 0 || worktrees.some((one) => one.bytes === undefined)) return "";
  return sized(worktrees.reduce((sum, one) => sum + (one.bytes ?? 0), 0));
}

/**
 * The checkouts this Job holds, one row each, each with the control that opens it.
 *
 * **Absent and unmeasured are two answers.** No worktree is a job at its
 * approval gate or one already reclaimed; a size that did not arrive is a walk
 * that ran past its bound, which is what a very large checkout does.
 */
export function Worktrees({
  worktrees,
  onOpen,
}: {
  worktrees: PulseWorktreeRow[];
  onOpen?: (what: Artifact) => void;
}) {
  if (worktrees.length === 0) return null;
  return (
    <ul className="armada-holds__rows" data-list="worktrees">
      {worktrees.map((one) => (
        <li key={one.branch} className="armada-holds__row" data-tall>
          {/* The state again, on hover: the dot is the one mark on the row a
              person hovers to ask what its colour means. **Hidden, and no
              stop** — the row says the state in text beside it, so a name or a
              description here would be the state read twice. */}
          <Tooltip label={one.state} asChild decorative>
            <span
              className="armada-holds__row-dot"
              data-working={one.working || undefined}
              data-wrong={one.wrong || undefined}
              aria-hidden
            />
          </Tooltip>
          <span className="armada-holds__stack">
            <span className="armada-holds__name armada-holds__mono" title={one.path}>
              {one.branch}
            </span>
            <span className="armada-holds__state" data-wrong={one.wrong || undefined}>
              {one.state}
            </span>
          </span>
          <span className="armada-holds__sized">
            <Tooltip label={SIZE_ON_DISK} asChild>
              <span className="armada-holds__weight">{one.bytes === undefined ? NOT_WALKED : sized(one.bytes)}</span>
            </Tooltip>
            {one.age === undefined ? null : (
              <Tooltip label={LAST_MEASURED} asChild>
                <span className="armada-holds__read-at">{ago(one.age)}</span>
              </Tooltip>
            )}
          </span>
          <Opens open={one.open} onOpen={onOpen} />
        </li>
      ))}
    </ul>
  );
}

/**
 * The Job's logs, one row per file, and a filter over whose they are.
 *
 * **Being written is the fact the list exists for.** A log that stopped
 * growing while its Job reads running is the shape of a hang, so a file still
 * open says so where its `Open` would be — as a mark and not a phrase (owner,
 * 2 Oct: "I hate text over icons"). `circle-dot` with its centre pulsing, the
 * glyph `DroneTurns` draws for a Drone still writing, and named by its tooltip.
 *
 * **A size Fleet could not measure is an empty cell** (owner, 29 Sep: "We dont
 * need to say anything"). The cell stays, so the column still lines up.
 */
export function Logs({
  logs,
  member = ANY,
  onOpen,
  onView,
  viewing,
}: {
  logs: PulseLogRow[];
  /** Whose logs to list. `LogMember` picks it. */
  member?: string;
  onOpen?: (what: Artifact) => void;
  /** Read a row with a `path` in a panel. The whole row takes the press. */
  onView?: (log: PulseLogRow) => void;
  /** The path whose panel is open. */
  viewing?: string;
}) {
  const shown = member === ANY ? logs : logs.filter((one) => one.owner === member);
  if (logs.length === 0) return null;
  return (
    <ul className="armada-holds__rows" data-list="logs">
      {shown.map((one) => (
        <li
          key={`${one.owner ?? ""}/${one.kind}/${one.about ?? ""}`}
          className="armada-holds__row"
          data-viewable={(onView !== undefined && one.path !== undefined) || undefined}
          aria-current={one.path !== undefined && one.path === viewing ? "true" : undefined}
          onClick={onView === undefined || one.path === undefined ? undefined : () => onView(one)}
        >
          {onView === undefined || one.path === undefined ? (
            <span className="armada-holds__kind">{KIND[one.kind] ?? one.kind}</span>
          ) : (
            // The keyboard's path to the row's press, `JobDrones`' own. The
            // press stops here, or the row answers it a second time.
            <button
              type="button"
              className="armada-holds__kind armada-holds__view"
              aria-label={`${KIND[one.kind] ?? one.kind}, ${whoseLog(one)}`}
              aria-expanded={one.path === viewing}
              onClick={(event) => {
                event.stopPropagation();
                onView(one);
              }}
            >
              {KIND[one.kind] ?? one.kind}
            </button>
          )}
          <span className="armada-holds__name armada-holds__mono">{whoseLog(one)}</span>
          {one.bytes === undefined ? (
            <span />
          ) : (
            <Tooltip label={SIZE_ON_DISK} asChild>
              <span className="armada-holds__weight">{sized(one.bytes)}</span>
            </Tooltip>
          )}
          {one.writing ? (
            <BeingWritten />
          ) : (
            <Opens open={one.open} onOpen={onOpen} />
          )}
        </li>
      ))}
    </ul>
  );
}

/** Every sub job, which is also what a Job that is not a train shows. */
export const ANY = "All sub jobs";

/**
 * The filter over whose logs are listed. `All sub jobs` and every sub job that
 * owns a row, so a Job with one draws the control with one choice.
 */
export function LogMember({
  logs,
  member,
  onMember,
}: {
  logs: PulseLogRow[];
  member: string;
  onMember: (member: string) => void;
}) {
  const owners = [...new Set(logs.flatMap((one) => (one.owner === null ? [] : [one.owner])))];
  return (
    <span className="armada-holds__filter">
      <Select aria-label="Which sub job" value={member} onChange={(event) => onMember(event.target.value)}>
        <option value={ANY}>{ANY}</option>
        {owners.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
    </span>
  );
}

/** What a log file is, as a person names it. The wire's word where this build has none. */
const KIND: Record<string, string> = {
  job: "Job log",
  transcript: "Drone transcript",
  brief: "Judge brief",
};

/** Whose it is and what about — `implement · no_drift`, or the Job's own. */
function whoseLog(one: PulseLogRow): string {
  const said = [one.owner, one.about].filter((part) => part !== null && part !== undefined);
  return said.length === 0 ? THE_JOBS_OWN : said.join(" · ");
}

/** The row's `Open`. Nothing where the row names nothing the host can open. */
function Opens({ open, onOpen }: { open?: Artifact; onOpen?: (what: Artifact) => void }) {
  if (open === undefined || onOpen === undefined) return null;
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={(event) => {
        // A row that opens a panel takes the press too; this one leaves the app.
        event.stopPropagation();
        onOpen(open);
      }}
    >
      Open
    </Button>
  );
}

/** What the mark on a log a writer holds open is, said by its tooltip and its name. */
const BEING_WRITTEN = "Being written";

/**
 * A log a writer holds open: `circle-dot` in the running hue, its centre
 * pulsing. **Exported** for the panel that reads the file, so a row and its
 * panel say *being written* with one mark and stop saying it together.
 */
export function BeingWritten() {
  return (
    <Tooltip label={BEING_WRITTEN} asChild>
      <span className="armada-holds__writing" role="img" aria-label={BEING_WRITTEN}>
        <CircleDot size={12} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** A log that belongs to the Job rather than to one of its members. */
const THE_JOBS_OWN = "this job";

/** A walk that ran past its bound. The same fact on a directory. */
const NOT_WALKED = "not measured";

/** What a worktree row's figure is, and what the card head's total is. */
const SIZE_ON_DISK = "Size on disk";
/** What the age under a worktree's size is. */
const LAST_MEASURED = "Last measured";

/** An age as the board spells it — `4s ago`. */
export function ago(age: string): string {
  return `${age} ago`;
}
export const TOTAL_SIZE_ON_DISK = "Total size on disk";

/**
 * Bytes as a person reads them. **Binary units and their own names**, because
 * `du` and `df` answer in them and a figure that disagreed with the shell a
 * person is about to open would be worse than no figure.
 */
export function sized(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let at = 0;
  let value = bytes;
  while (value >= 1024 && at < units.length - 1) {
    value = value / 1024;
    at += 1;
  }
  return `${at === 0 ? value : value.toFixed(1)} ${units[at]}`;
}