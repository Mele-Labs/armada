import { useState } from "react";
import { Button } from "../../primitives/Button/Button";
import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";
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
  /**
   * Where its own agent has got to, as a sentence — turns while it runs, the
   * cost once it stopped. Absent before anything was dispatched at it.
   */
  doing?: string;
  /** Present on a dropped task and on nothing else. */
  reason?: string;
  /** What the other fields cannot hold. Absent where the task has none. */
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
  /** The tasks it runs beside, by id. Empty where it runs alone. */
  beside?: readonly string[];
  /** The cases it owes. A case with no spec reads `not covered`, never green. */
  tests?: readonly PlanTaskTest[];
  /** Why its own agent stopped. Present on a failed task and on nothing else. */
  failedReason?: string;
  /**
   * Asking the Drone that wrote the plan to write this task differently.
   *
   * **Absent draws nothing**, which is every plan past its gate: once a plan
   * is approved it is a record, and a record takes no requests.
   */
  rewrite?: PlanTaskRewrite;
  /**
   * Telling this task's own Drone something while it works.
   *
   * **Review and reply are one loop**, so the box is in the surface the task is
   * read in. Absent where nothing is live to reach — and it says which Drone it
   * reaches, because a task with an agent of its own is not the Job's one Drone.
   */
  redirect?: PlanTaskRedirect;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  /** Beside the content, as Helm's dock. Below `--layout-breakpoint` it is a sheet over it. */
  docked?: boolean;
  onClose?: () => void;
};

/** The redirect, with the Drone it is addressed to named. */
export type PlanTaskRedirect = Omit<DroneMessageBoxProps, "placeholder"> & {
  /** What the box reaches — `Drone on T5`, or the Job's one Drone. */
  reaches: string;
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
 * The count over the file list. **Three figures only where the work has been
 * read**, since a Job whose turns nobody read would otherwise say every
 * declared file went untouched.
 */
function filesSaid(
  declared: number,
  touched?: PlanTaskSheetProps["touched"],
): string {
  if (touched === undefined) return `${declared} in the plan`;
  const reached = touched.declared.filter((file) => file.touched).length;
  const unplanned = touched.unplanned.length;
  const said = [`${declared} in the plan`, `${reached} written to`];
  if (unplanned > 0) said.push(`${unplanned} the plan never named`);
  return said.join(" · ");
}

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

export function PlanTaskSheet({
  open,
  id,
  title,
  state,
  doing,
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
  rewrite,
  redirect,
  floor = false,
  docked = false,
  onClose,
}: PlanTaskSheetProps) {
  return (
    <Sheet
      open={open}
      contained
      docked={docked}
      floor={floor}
      title={title}
      subtitle={
        <span className="armada-task-sheet__said">
          <span className="armada-task-sheet__id">{id}</span>
          <span className="armada-task-sheet__state">{STATE_SAID[state]}</span>
        </span>
      }
      leading={<TaskMark state={state} />}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-task-sheet__body">
        {/* Where its agent has got to, above everything the plan decided:
            what a person opening a task mid-run came for is what it is doing,
            and the plan is what it was told to do. */}
        {doing === undefined ? null : (
          <TaskField label="Where it got to">{doing}</TaskField>
        )}
        {reason === undefined ? null : (
          <TaskField label="Dropped because">{reason}</TaskField>
        )}
        {failedReason === undefined ? null : (
          <TaskField label="Why it stopped">{failedReason}</TaskField>
        )}
        {/* The brief, as far as the plan decides it. Fleet composes the rest
            from the Job, which is why this field says what the plan holds
            rather than claiming to be the whole prompt. */}
        <TaskField label="What the Drone is told">
          {(note ?? "") === "" ? "Its title and the files below, and nothing else." : note}
        </TaskField>
        {tier === undefined || model === undefined ? null : (
          <TaskField label="Model" note={runBy === undefined ? undefined : `Run by ${runBy}`}>
            {tier} · {model}
          </TaskField>
        )}
        <TaskField label="Runs beside">
          {beside.length === 0 ? "Nothing. It runs on its own." : beside.join(", ")}
        </TaskField>
        {scope.length === 0 && (touched?.unplanned.length ?? 0) === 0 ? null : (
          <TaskField label="Files" note={filesSaid(scope.length, touched)} bare>
            <Files scope={scope} touched={touched} />
          </TaskField>
        )}
        {(expects ?? "") === "" && (shown ?? "") === "" ? null : (
          /* **The planner's expectation, and labelled as one.** The Job's
             acceptance criteria are a different thing, and `#1274` is why: a
             Drone never chooses what it is held to. */
          <TaskField label="How we will know it worked" bare>
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
        <TaskField label="Tests for this task" bare={tests.length > 0}>
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
        {/* Named, so the box says which Drone a correction reaches: a task
            with an agent of its own is not the Job's one Drone, and a box
            that said neither would send to either. */}
        {redirect === undefined ? null : (
          <section className="armada-task-sheet__field" aria-label="Redirect">
            <h3 className="armada-task-sheet__label">Redirect</h3>
            <p className="armada-task-sheet__reaches" role="note">
              Reaches {redirect.reaches}
            </p>
            <DroneMessageBox
              value={redirect.value}
              onChange={redirect.onChange}
              onSend={redirect.onSend}
              disabled={redirect.disabled}
              disabledReason={redirect.disabledReason}
              waiting={redirect.waiting}
            />
          </section>
        )}
      </div>
    </Sheet>
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
}: {
  scope: readonly string[];
  touched?: PlanTaskSheetProps["touched"];
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
            <span>{file.path}</span>
            {/* **Only the unreached one is marked.** Marking both halves
                would put a badge on every row and say nothing; the row
                worth stopping on is the one the work never reached. */}
            {file.touched ? null : <span className="armada-task-sheet__unreached">not touched</span>}
          </li>
        ))}
        {unplanned.map((path) => (
          <li key={path} className="armada-task-sheet__unplanned">
            <span>{path}</span>
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

function Evidence({ said, of, absent }: { said: string; of?: string; absent: string }) {
  const empty = (of ?? "") === "";
  return (
    <div className="armada-task-sheet__row">
      <dt>{said}</dt>
      <dd className={empty ? "armada-task-sheet__unsaid" : undefined}>{empty ? absent : of}</dd>
    </div>
  );
}
