import { useEffect, useRef, useState } from "react";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { TASK_GLYPH, type TaskMarkState } from "../TaskMark/TaskMark";
import { UnifiedDiff, type UnifiedDiffProps } from "../UnifiedDiff/UnifiedDiff";
import { WorkflowStepCard, type WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";
import { TaskField } from "./TaskFields";

/**
 * One plan task's whole reading, on the layer that can hold it.
 *
 * **A new home rather than an edit**, `JobHoldsSheet`'s shape. The task's note,
 * the paths it touches and both ends of its evidence were drawn under the title
 * in the rail, and the rail is 380px: one real plan put 5,196 characters of
 * them under 364 characters of title, and a 69-character path was clipped
 * mid-word because a path cannot break. The row keeps what a person scans —
 * mark, id, title, and how many files — and this holds the rest.
 *
 * **Not `wide`.** `JobHoldsSheet` argues it for the same reading: a card
 * legible in a 380px column, spread across three times that, puts four figures
 * across a field instead of down one.
 */
export type PlanTaskSheetProps = {
  open: boolean;
  /** `T1`, `T2`, … as the plan numbered it. */
  id: string;
  title: string;
  state: TaskMarkState;
  /** Present on a dropped task and on nothing else. */
  reason?: string;
  /**
   * The planner's brief for the task — what Edit this task will change.
   * Absent or empty draws nothing.
   */
  note?: string;
  /** The repository-relative paths the task names, in the order it named them. */
  scope?: readonly string[];
  /**
   * Which of `scope` the work actually reached, and what it reached that
   * `scope` never named. **Absent is not empty**: absent is a Job whose turns
   * were not read, where empty is a task that touched nothing. `#1432`.
   */
  touched?: { declared: readonly { path: string; touched: boolean }[]; unplanned: readonly string[] };
  /** What the plan said should prove it, written before the work. */
  expects?: string;
  /** What the work said proved it, written by whoever did it. */
  shown?: string;
  /**
   * How hard the planner thought it was, and the model that tier resolved to.
   * **The planner picks the tier and the model follows** (`#1530`, 22 Sep), so
   * the pair is drawn in that order and never the model alone.
   */
  tier?: string;
  model?: string;
  /** How it is run — `its own agent`, `the step's Drone`, `a Job of its own`. */
  runBy?: string;
  /**
   * The tasks it runs beside, as the Plan graph draws each one — **the same
   * card, so it reads the same live state**. Empty draws nothing.
   */
  beside?: readonly WorkflowStepCardProps[];
  /** The cases it owes. A case with no spec reads `not covered`, never green. */
  tests?: readonly PlanTaskTest[];
  /** Why its own agent stopped. Present on a failed task and on nothing else. */
  failedReason?: string;
  /**
   * What a person can do about a failed task, beside messaging its Drone.
   * Present on a failed task and on nothing else.
   */
  acts?: PlanTaskActs;
  /**
   * Asking the Drone that wrote the plan to write this task differently.
   *
   * **Absent draws nothing**, which is every plan past its gate: once a plan
   * is approved it is a record, and a record takes no requests.
   */
  rewrite?: PlanTaskRewrite;
  /**
   * Telling this task's Drone something while it works.
   *
   * **Review and reply are one loop**, so the box is in the surface the task is
   * read in, under the Drone it reaches. Absent where nothing is live to reach.
   */
  redirect?: PlanTaskRedirect;
  /**
   * The paths the Job's patch changed. **A file in it is a press** that opens
   * what the Job did to it; a file outside it stays text. Absent is a Job
   * with no patch yet, and no file is a press.
   */
  patched?: readonly string[];
  /**
   * The file open beside the task, and everything the Job's patch did to it.
   * **The Job's, not the task's** — Fleet serves one patch, so where two tasks
   * wrote one file both show (owner, 29 Sep 2026).
   */
  file?: { path: string; diff: UnifiedDiffProps };
  /** A file pressed, or `null` for the diff closed. */
  onFile?: (path: string | null) => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  /** Beside the content, as Helm's dock. Below `--layout-breakpoint` it is a sheet over it. */
  docked?: boolean;
  onClose?: () => void;
};

/** The message box under the Drone. */
export type PlanTaskRedirect = Omit<DroneMessageBoxProps, "placeholder">;

/**
 * A failed task's own acts — the owner's decision of 29 Sep 2026, *a failed
 * task offers four acts*. The fourth is the message box. **Each is on screen
 * ahead of its Fleet route** (#250, #1656, #1657), so a press answers
 * `Not implemented` naming the issue until the route ships.
 */
export type PlanTaskActs = {
  onPilot: () => void;
  onRestart: () => void;
  onEdit: () => void;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};

/**
 * The rewrite ask, in the caller's own words.
 *
 * **Prose and not a form.** What a person wants a task to be instead is a
 * sentence the Drone reads, so the field is a `textarea` — a plan revision
 * with a picker for each field would be editing the record, which is the one
 * thing this is not.
 */
export type PlanTaskRewrite = {
  /** What the ask is, and that it may come back refused. */
  lead: string;
  /** The field's own label. */
  label: string;
  placeholder: string;
  /** The control's word — `Ask the Drone`. */
  send: string;
  /** The ask is out and nothing has answered. */
  pending?: boolean;
  /** Nothing is live to send it over. */
  disabled?: boolean;
  onAsk: (instruction: string) => void;
};

/** One case this task owes, and what it reads as. */
export type PlanTaskTest = {
  id: string;
  /** The spec's repository path. */
  spec: string;
  reads: "owed" | "not covered" | "dropped";
  /** What dropped it. Present on `dropped` and on nothing else. */
  droppedSays?: string;
};

/**
 * How many paths a task shows before it offers the rest.
 *
 * **Twelve, because the owner asked what twenty would do** (28 Sep 2026).
 * A list that long pushes everything under it — the evidence, the tests, the
 * box that reaches its Drone — off a 392px panel, and the rows a reader wants
 * are the first few and the unplanned ones, which sort to the end.
 */
const FILES_SHOWN = 12;

/** The words for a state, as the rail's own mark spells them. */
const STATE_SAID: Record<TaskMarkState, string> = {
  open: "Open",
  working: "Working",
  done: "Done",
  failed: "Failed",
  dropped: "Dropped",
};

/**
 * The status token stem each state's tag takes. **The hue the rail's mark
 * already draws**: `--step-advanced`, `--step-running` and `--step-failed` are
 * aliases of these three, and a drop is a person's decision, `killed`'s own.
 */
const STATE_STATUS: Record<TaskMarkState, string> = {
  open: "not-started",
  working: "running",
  done: "completed-success",
  failed: "completed-failed",
  dropped: "killed",
};

export function PlanTaskSheet({
  open,
  id,
  title,
  state,
  reason,
  note,
  scope = [],
  touched,
  expects,
  shown,
  tier,
  model,
  runBy,
  beside = [],
  tests = [],
  failedReason,
  acts,
  rewrite,
  redirect,
  patched,
  file,
  onFile,
  floor = false,
  docked = false,
  onClose,
}: PlanTaskSheetProps) {
  return (
    <>
    <Sheet
      open={open}
      contained
      docked={docked}
      floor={floor}
      title={title}
      subtitle={
        <Badge status={STATE_STATUS[state]} icon={TASK_GLYPH[state]}>
          {STATE_SAID[state]}
        </Badge>
      }
      leading={<span className="armada-task-sheet__id">{id}</span>}
      closeLabel="Close"
      closeBinding="Esc"
      under={file !== undefined}
      onClose={onClose}
    >
      <div className="armada-task-sheet__body">
        {reason === undefined ? null : (
          <TaskField label="Dropped because">{reason}</TaskField>
        )}
        {failedReason === undefined ? null : (
          <TaskField label="Why it stopped">{failedReason}</TaskField>
        )}
        {acts === undefined ? null : (
          <div className="armada-task-sheet__acts">
            <Button size="sm" ground="sunken" disabled={acts.disabled} onClick={acts.onPilot}>
              Pilot
            </Button>
            <Button size="sm" ground="sunken" disabled={acts.disabled} onClick={acts.onRestart}>
              Restart this task
            </Button>
            <Button size="sm" ground="sunken" disabled={acts.disabled} onClick={acts.onEdit}>
              Edit this task
            </Button>
          </div>
        )}
        {(note ?? "") === "" ? null : <TaskField label="Brief">{note}</TaskField>}
        {/* **A frame for a peek into the Drone, and empty on purpose** (owner,
            29 Sep 2026): what it shows waits on the Drone work. The message
            box sits under it, so what a person reads and what they send are
            one place. */}
        <section className="armada-task-sheet__field" aria-label="Drone">
          <h3 className="armada-task-sheet__label">Drone</h3>
          <div className="armada-task-sheet__peek" aria-hidden="true" />
          {redirect === undefined ? null : (
            <DroneMessageBox
              value={redirect.value}
              onChange={redirect.onChange}
              onSend={redirect.onSend}
              disabled={redirect.disabled}
              disabledReason={redirect.disabledReason}
              waiting={redirect.waiting}
            />
          )}
        </section>
        {tier === undefined || model === undefined ? null : (
          <TaskField label="Model" note={runBy === undefined ? undefined : `Run by ${runBy}`}>
            {tier} · {model}
          </TaskField>
        )}
        {beside.length === 0 ? null : (
          <TaskField label="Runs beside">
            <ul className="armada-task-sheet__beside">
              {beside.map((card) => (
                <li key={card.name}>
                  <WorkflowStepCard {...card} />
                </li>
              ))}
            </ul>
          </TaskField>
        )}
        {scope.length === 0 && (touched?.unplanned.length ?? 0) === 0 ? null : (
          <TaskField label="Files">
            <Files
              scope={scope}
              touched={touched}
              patched={patched}
              open={file?.path}
              onFile={onFile}
            />
          </TaskField>
        )}
        {(expects ?? "") === "" && (shown ?? "") === "" ? null : (
          /* **The planner's expectation, and labelled as one.** The Job's
             acceptance criteria are a different thing, and `#1274` is why: a
             Drone never chooses what it is held to. */
          <TaskField label="Done when">
            {/* **Two rows and never one.** The plan names an artifact before the
                work starts and the work finds out what actually proved it; the
                pair disagreeing is what a reader is here for. */}
            <dl className="armada-task-sheet__evidence">
              <Evidence said="The plan asked for" of={expects} absent="Nothing was named." />
              <Evidence
                said="The work showed"
                of={shown}
                absent={state === "done" ? "Nothing was recorded." : "Not yet."}
              />
            </dl>
          </TaskField>
        )}
        {/* Every case is the planning Drone's, written into the plan before
            any of this ran — `casesOf` in `tab-plan-read.ts`. Nothing a
            person types reaches this list, which is why it is read and not
            edited here. */}
        <TaskField label="Tests for this task">
          {tests.length === 0 ? (
            "No test covers this task yet."
          ) : (
            <ul className="armada-task-sheet__files">
              {tests.map((test) => (
                <li key={test.id}>
                  <span>{test.spec}</span>
                  {test.reads === "owed" ? null : (
                    <span className="armada-task-sheet__unreached">
                      {test.droppedSays ?? test.reads}
                    </span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </TaskField>
        {rewrite === undefined ? null : <Rewrite {...rewrite} />}
      </div>
    </Sheet>
    {file === undefined ? null : (
      /* **Beside the task where there is room, over it where there is not.**
         Docked, it is a second dock to the left, so the task and the file
         read together. Below the breakpoint two panels do not fit, and it
         is a sheet over the task sheet: closing it lands back on the task. */
      <Sheet
        open
        contained
        docked={docked}
        beside
        floor={floor}
        title={file.path}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={() => onFile?.(null)}
      >
        <div className="armada-task-diff">
          <UnifiedDiff {...file.diff} />
        </div>
      </Sheet>
    )}
    </>
  );
}

/**
 * Asking for this task to be written differently.
 *
 * **Inline rather than behind a dialog.** A dialog and a sheet share
 * `--z-modal`, so one opened from inside the other is a stack the tokens do
 * not order — and the ask needs a sentence anyway, which is what a dialog
 * would have had to hold. The field is the confirmation: an empty one sends
 * nothing.
 */
function Rewrite({ lead, label, placeholder, send, pending = false, disabled = false, onAsk }: PlanTaskRewrite) {
  const [instruction, setInstruction] = useState("");
  const empty = instruction.trim() === "";
  return (
    <section className="armada-task-sheet__field" aria-label={label}>
      <h3 className="armada-task-sheet__label">{label}</h3>
      <p className="armada-task-sheet__prose">{lead}</p>
      <Textarea
        rows={3}
        value={instruction}
        placeholder={placeholder}
        disabled={disabled || pending}
        onChange={(event) => setInstruction(event.target.value)}
      />
      <div className="armada-task-sheet__rewrite-act">
        <Button
          size="sm"
          ground="sunken"
          pending={pending}
          disabled={disabled || empty}
          onClick={() => onAsk(instruction.trim())}
        >
          {send}
        </Button>
      </div>
    </section>
  );
}

/**
 * The paths, capped.
 *
 * **What the plan never named sorts last and is never cut.** The rows a
 * reader stops on are the ones the work reached that nothing asked it to, so
 * the cap eats the ordinary declared paths and leaves those standing.
 */
function Files({
  scope,
  touched,
  patched,
  open,
  onFile,
}: {
  scope: readonly string[];
  touched?: PlanTaskSheetProps["touched"];
  patched?: readonly string[];
  open?: string;
  onFile?: (path: string | null) => void;
}) {
  const [whole, setWhole] = useState(false);
  const declared = touched?.declared ?? scope.map((path) => ({ path, touched: true }));
  const unplanned = touched?.unplanned ?? [];
  const cut = whole ? declared.length : Math.min(declared.length, FILES_SHOWN);
  const rest = declared.length - cut;
  return (
    <>
      <ul className="armada-task-sheet__files">
        {declared.slice(0, cut).map((file) => (
          <li key={file.path}>
            <FilePath path={file.path} patched={patched} open={open} onFile={onFile} />
            {/* **Only the unreached one is marked.** Marking both halves
                would put a badge on every row and say nothing; the row
                worth stopping on is the one the work never reached. */}
            {file.touched ? null : <span className="armada-task-sheet__unreached">not touched</span>}
          </li>
        ))}
        {unplanned.map((path) => (
          <li key={path} className="armada-task-sheet__unplanned">
            <FilePath path={path} patched={patched} open={open} onFile={onFile} />
            <span className="armada-task-sheet__unreached">not planned</span>
          </li>
        ))}
      </ul>
      {rest === 0 ? null : (
        <Button size="sm" ground="sunken" onClick={() => setWhole(true)}>
          Show {rest} more
        </Button>
      )}
    </>
  );
}

/** A path, and a press where the Job's patch changed it. */
function FilePath({
  path,
  patched,
  open,
  onFile,
}: {
  path: string;
  patched?: readonly string[];
  open?: string;
  onFile?: (path: string | null) => void;
}) {
  const selected = open === path;
  const ref = useRef<HTMLButtonElement>(null);
  const was = useRef(selected);
  // The diff closed with nothing else open: focus comes back to the row that
  // opened it rather than falling to the page.
  useEffect(() => {
    if (was.current && open === undefined) ref.current?.focus();
    was.current = selected;
  }, [selected, open]);
  if (onFile === undefined || patched?.includes(path) !== true) return <span>{path}</span>;
  return (
    <button
      ref={ref}
      type="button"
      className="armada-task-sheet__file"
      aria-pressed={selected}
      onClick={() => onFile(selected ? null : path)}
    >
      {path}
    </button>
  );
}

function Evidence({ said, of, absent }: { said: string; of?: string; absent: string }) {
  const empty = (of ?? "") === "";
  return (
    <div className="armada-task-sheet__row">
      <dt>{said}</dt>
      <dd className={empty ? "armada-task-sheet__unsaid" : undefined}>{empty ? absent : of}</dd>
    </div>
  );
}
