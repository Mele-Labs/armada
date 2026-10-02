import { useEffect, useRef, useState } from "react";
import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Prose } from "../../primitives/Prose/Prose";
import { patternFor, useHaptics } from "../../haptics";
import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { DronePeek, type DronePeekProps } from "../DronePeek/DronePeek";
import { HoldButton, type HoldButtonProps } from "../../primitives/HoldButton/HoldButton";
import { PlanOverlap, PlanProposeForm, type PlanPropose } from "../PlanBoard/PlanBoard";
import { PlanDropForm, type PlanTaskDrop } from "../PlanBoard/PlanDropForm";

export type { PlanTaskDrop };
import { Select } from "../../primitives/Select/Select";
import { Sheet, type SheetBack } from "../../primitives/Sheet/Sheet";
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
   * `T7 edited a file this task had already finished`, the caller's own
   * sentence, drawn under Files as the plan list draws a group's overlap.
   * Absent draws nothing.
   */
  overlap?: string;
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
   * How hard the planner thought it was, and the model the task runs on —
   * the tier's, unless a person picked one in Edit this task (owner, 30 Sep
   * 2026, reversing the model half of `#1530`'s 22 Sep call).
   */
  tier?: string;
  model?: string;
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
   * What its own agent is doing now, as a sentence — turns while it works,
   * its cost once it stopped. **Absent draws nothing**, which is every task
   * without a Drone of its own (`#1536`).
   */
  doing?: string;
  /**
   * The last file its own Drone wrote, and the size of that edit where the
   * call carried one. Absent draws nothing.
   */
  lastEdit?: { path: string; says?: string };
  /**
   * Hold to stop this task's Drone, beside the task's other acts. Absent
   * draws nothing, which is a task with no Drone of its own running.
   */
  stop?: PlanTaskStop;
  /**
   * What a person can do about a failed task, beside messaging its Drone.
   * Present on a failed task and on nothing else.
   */
  acts?: PlanTaskActs;
  /**
   * Changing the task itself — its title, brief, files, done-when and model
   * (owner, 30 Sep 2026). **Absent draws nothing**, which is a task that is
   * working, done or dropped.
   */
  edit?: PlanTaskEdit;
  /**
   * Dropping the task from the plan, with a reason (owner, 30 Sep 2026).
   * **Absent draws nothing**, which is a task that is done or already
   * dropped: the first has nothing left to drop, the second already is one.
   */
  drop?: PlanTaskDrop;
  /**
   * Proposing a change to this task to the Drone that wrote the plan — the
   * group's own Propose a change, on one task.
   *
   * **Absent draws nothing**, which is every plan past its gate: once a plan
   * is approved it is a record, and a record takes no requests.
   */
  propose?: PlanTaskPropose;
  /**
   * Telling this task's Drone something while it works.
   *
   * **Review and reply are one loop**, so the box is in the surface the task is
   * read in, under the Drone it reaches. Absent where nothing is live to reach.
   */
  redirect?: PlanTaskRedirect;
  /**
   * The task's own Drone, peeked at over the message box. **Absent is a task
   * no Drone has run**, which draws the box alone.
   */
  drone?: PlanTaskDrone;
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
  /**
   * The panel's width, where a person has dragged it — `Sheet`'s own pair
   * (owner, 30 Sep 2026: "I should be able to resize it with the resize
   * handle we have"). Absent draws `--w-dock`; no `onResize` draws no handle.
   * The file diff beside it follows whatever width this is.
   */
  width?: number;
  onResize?: (width: number) => void;
  onClose?: () => void;
  /** The way back, where a press elsewhere opened this panel. `Sheet`'s slot. */
  back?: SheetBack | undefined;
};

/** Hold to stop the task's own Drone. */
export type PlanTaskStop = Pick<
  HoldButtonProps,
  "children" | "askLabel" | "description" | "onCommit" | "onAsk" | "disabled" | "pending"
>;

/** The message box under the Drone. */
export type PlanTaskRedirect = Omit<DroneMessageBoxProps, "placeholder">;

/** The task's Drone, as the peek draws it. The box is `redirect`. */
export type PlanTaskDrone = Omit<DronePeekProps, "message">;

/**
 * A failed task's own acts — the owner's decision of 29 Sep 2026, *a failed
 * task offers four acts*: these two, Edit this task, and the message box.
 * **Each is on screen ahead of its Fleet route** (#250, #1656), so a press
 * answers `Not implemented` naming the issue until the route ships.
 */
export type PlanTaskActs = {
  onPilot: () => void;
  onRestart: () => void;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};

/** What Edit this task changed. **Only the fields that differ** from the task as drawn. */
export type PlanTaskEdited = {
  title?: string;
  note?: string;
  scope?: string[];
  expects?: string;
  model?: string;
};

/**
 * Editing a task, in a form under the acts, filled from the task as it
 * stands. **Ahead of its route** (#1657), as a failed task's acts are.
 */
export type PlanTaskEdit = {
  /** Every model the app knows, any of which the task may run on. */
  models: readonly string[];
  /**
   * Sends what changed. Resolves `true` where it was taken, which closes the
   * form, and `false` where it was not — the form stays with what was typed,
   * and the refusal is the app's to draw.
   */
  onEdit: (edit: PlanTaskEdited) => Promise<boolean>;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};


/**
 * Proposing a change, in the caller's own words. **Prose and not a form**: it
 * is a sentence the Drone reads and may refuse. Edit this task is the form.
 */
export type PlanTaskPropose = PlanPropose & { onPropose: (instruction: string) => void };

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
  overlap,
  touched,
  expects,
  shown,
  tier,
  model,
  beside = [],
  tests = [],
  failedReason,
  doing,
  lastEdit,
  stop,
  acts,
  edit,
  drop,
  propose,
  redirect,
  drone,
  patched,
  file,
  onFile,
  floor = false,
  width,
  onResize,
  onClose,
  back,
}: PlanTaskSheetProps) {
  return (
    <>
    {/* **The floating sheet Record and Drones open** (owner, 30 Sep 2026):
        over the work area, dimming it, so a press behind it lands nowhere. */}
    <Sheet
      open={open}
      floating
      {...(onResize === undefined ? {} : { width, onResize })}
      floor={floor}
      title={title}
      subtitle={
        <Badge status={STATE_STATUS[state]} icon={TASK_GLYPH[state]}>
          {STATE_SAID[state]}
        </Badge>
      }
      leading={<span className="armada-task-sheet__id">{id}</span>}
      back={back}
      closeLabel="Close"
      closeBinding="Esc"
      under={file !== undefined}
      onClose={onClose}
    >
      <div className="armada-task-sheet__body">
        {reason === undefined ? null : (
          <TaskField label="Dropped because">
            <Prose text={reason} />
          </TaskField>
        )}
        {failedReason === undefined ? null : (
          <TaskField label="Why it stopped">
            <Prose text={failedReason} />
          </TaskField>
        )}
        {doing === undefined ? null : (
          <TaskField label="Now">
            <Prose text={doing} />
          </TaskField>
        )}
        {acts === undefined && edit === undefined && drop === undefined && propose === undefined && stop === undefined ? null : (
          /* Keyed apart from the peek, which is keyed by the task too: two
             siblings on one key leave a stale copy behind. */
          <Acts
            key={`${id}-acts`}
            acts={acts}
            edit={edit}
            drop={drop}
            propose={propose}
            stop={stop}
            task={{ title, note, scope, expects, model }}
          />
        )}
        {(note ?? "") === "" ? null : (
          <TaskField label="Brief">
            <Prose text={note ?? ""} />
          </TaskField>
        )}
        {/* **What a person reads and what they send are one place**, under
            the head: the Drone's tail with the box at its foot. A task no
            Drone has run has no tail, so the box stands alone. */}
        {drone !== undefined ? (
          <DronePeek
            key={id}
            {...drone}
            {...(redirect === undefined ? {} : { message: redirect })}
          />
        ) : redirect === undefined ? null : (
          <section className="armada-task-sheet__field" aria-label="Drone">
            <h3 className="armada-task-sheet__label">Drone</h3>
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
        {lastEdit === undefined ? null : (
          <TaskField label="Last edit">
            {/* A row of the Files list, so it reads as one and opens the
                same diff where the Job's patch changed it. */}
            <ul className="armada-task-sheet__files">
              <li>
                <FilePath path={lastEdit.path} patched={patched} open={file?.path} onFile={onFile} />
                {lastEdit.says === undefined ? null : <span>{lastEdit.says}</span>}
              </li>
            </ul>
          </TaskField>
        )}
        {tier === undefined || model === undefined ? null : (
          <TaskField label="Model">
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
            {overlap === undefined ? null : <PlanOverlap says={overlap} />}
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
      </div>
    </Sheet>
    {file === undefined ? null : (
      /* **Beside the task, under the one dim.** A second floating sheet to
         the task's left, so the task and the file read together; its own
         scrim dims nothing. At the floor the task is flush to both edges and
         this lies over it: closing it lands back on the task. */
      <Sheet
        open
        floating
        beside
        {...(onResize === undefined ? {} : { besideWidth: width })}
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
 * A task's acts, and whichever one is open under them — the edit form, the
 * drop's reason, or the proposal. **One open at a time.** A failed task's two
 * come first; while a plan waits on a person, Propose a change sits between
 * Edit this task and Drop this task. Drop this task is always last.
 */
function Acts({
  acts,
  edit,
  drop,
  propose,
  stop,
  task,
}: {
  acts?: PlanTaskActs;
  edit?: PlanTaskEdit;
  drop?: PlanTaskDrop;
  propose?: PlanTaskPropose;
  stop?: PlanTaskStop;
  task: TaskAsDrawn;
}) {
  const [open, setOpen] = useState<"edit" | "drop" | "propose" | null>(null);
  const shut = () => setOpen(null);
  return (
    <>
      <div className="armada-task-sheet__acts">
        {acts === undefined ? null : (
          <>
            <Button size="sm" ground="sunken" disabled={acts.disabled} onClick={acts.onPilot}>
              Pilot
            </Button>
            <Button size="sm" ground="sunken" disabled={acts.disabled} onClick={acts.onRestart}>
              Restart this task
            </Button>
          </>
        )}
        {edit === undefined || open === "edit" ? null : (
          <Button size="sm" ground="sunken" disabled={edit.disabled} onClick={() => setOpen("edit")}>
            Edit this task
          </Button>
        )}
        {propose === undefined || open === "propose" ? null : (
          <Button
            size="sm"
            ground="sunken"
            disabled={propose.disabled === true || propose.pending === true}
            onClick={() => setOpen("propose")}
          >
            {propose.label}
          </Button>
        )}
        {drop === undefined || open === "drop" ? null : (
          <Button size="sm" ground="sunken" disabled={drop.disabled} onClick={() => setOpen("drop")}>
            Drop this task
          </Button>
        )}
        {stop === undefined ? null : <HoldButton size="sm" ground="sunken" {...stop} />}
      </div>
      {edit === undefined || open !== "edit" ? null : <EditForm edit={edit} task={task} onClose={shut} />}
      {propose === undefined || open !== "propose" ? null : (
        <PlanProposeForm
          label={propose.label}
          send={propose.send}
          {...(propose.pending === undefined ? {} : { pending: propose.pending })}
          {...(propose.disabled === undefined ? {} : { disabled: propose.disabled })}
          onCancel={shut}
          onSend={(instruction) => {
            shut();
            propose.onPropose(instruction);
          }}
        />
      )}
      {drop === undefined || open !== "drop" ? null : <PlanDropForm drop={drop} label="Drop this task" send="Drop" sending="Dropping…" onClose={shut} />}
    </>
  );
}

/** The task as the panel draws it, which the edit form starts from. */
type TaskAsDrawn = {
  title: string;
  note: string | undefined;
  scope: readonly string[];
  expects: string | undefined;
  model: string | undefined;
};

/** One path per line, blank lines and stray spaces dropped. */
function pathsOf(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

/**
 * Edit this task: the five fields, filled from the task. **Cancel restores**
 * by unmounting, so the next Edit starts from the task again. Save sends only
 * what differs, and is off until something does.
 */
function EditForm({ edit, task, onClose }: { edit: PlanTaskEdit; task: TaskAsDrawn; onClose: () => void }) {
  const [title, setTitle] = useState(task.title);
  const [note, setNote] = useState(task.note ?? "");
  const [files, setFiles] = useState(task.scope.join("\n"));
  const [expects, setExpects] = useState(task.expects ?? "");
  const [model, setModel] = useState(task.model ?? "");
  const [saving, setSaving] = useState(false);
  const tap = useHaptics();

  const scope = pathsOf(files);
  const changed: PlanTaskEdited = {
    ...(title.trim() === task.title ? {} : { title: title.trim() }),
    ...(note.trim() === (task.note ?? "") ? {} : { note: note.trim() }),
    ...(scope.join("\n") === task.scope.join("\n") ? {} : { scope }),
    ...(expects.trim() === (task.expects ?? "") ? {} : { expects: expects.trim() }),
    ...(model === (task.model ?? "") ? {} : { model }),
  };
  const untitled = title.trim() === "";
  const same = Object.keys(changed).length === 0;
  // The task's own model leads the list even where the app does not know it,
  // so the field never reads as a model the task is not on.
  const models = task.model === undefined || edit.models.includes(task.model) ? edit.models : [task.model, ...edit.models];

  async function save(): Promise<void> {
    setSaving(true);
    try {
      const taken = await edit.onEdit(changed);
      tap(patternFor(taken ? "accepted" : "refused"));
      if (taken) onClose();
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="armada-task-sheet__edit" aria-label="Edit this task">
      <Input
        label="Title"
        value={title}
        invalid={untitled}
        disabled={saving}
        onChange={(event) => setTitle(event.target.value)}
      />
      <Textarea label="Brief" rows={3} value={note} disabled={saving} onChange={(event) => setNote(event.target.value)} />
      <Textarea label="Files" rows={3} value={files} disabled={saving} onChange={(event) => setFiles(event.target.value)} />
      <Textarea
        label="Done when"
        rows={2}
        value={expects}
        disabled={saving}
        onChange={(event) => setExpects(event.target.value)}
      />
      <Select label="Model" value={model} disabled={saving} onChange={(event) => setModel(event.target.value)}>
        {task.model === undefined ? <option value="" /> : null}
        {models.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
      <div className="armada-task-sheet__drop-acts">
        <Button variant="secondary" size="sm" ground="sunken" disabled={saving} onClick={onClose}>
          Cancel
        </Button>
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          pending={saving}
          disabled={untitled || same || edit.disabled}
          onClick={() => void save()}
        >
          {saving ? "Saving…" : "Save"}
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
  const empty = of === undefined || of === "";
  return (
    <div className="armada-task-sheet__row">
      <dt>{said}</dt>
      <dd className={empty ? "armada-task-sheet__unsaid" : undefined}>
        {empty ? absent : <Prose text={of} />}
      </dd>
    </div>
  );
}
