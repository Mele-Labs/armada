import { Button } from "../../primitives/Button/Button";
import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { HoldButton, type HoldButtonProps } from "../../primitives/HoldButton/HoldButton";
import { PathChip } from "../PathChip/PathChip";
import { Prose } from "../../primitives/Prose/Prose";
import { Select } from "../../primitives/Select/Select";
import { Sheet, type SheetBack } from "../../primitives/Sheet/Sheet";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * One step or group of a Job's workflow, read whole — what it is doing, the
 * tasks under it, the Checks at its boundary, the tests at that same boundary,
 * and the two things a person may do about it. `#1539`.
 *
 * **Tests are drawn apart from the Checks** (`#1530`, 22 Sep). A Check is the
 * repository's command; a case is what the work owes. Folding them into one
 * list would make a case with no spec read as a Check that passed.
 *
 * **Review and reply are one loop**, so the redirect is in this panel and not
 * behind a dialog somewhere else — and it says which Drone it reaches, because
 * a group running two tasks has two.
 *
 * **The regions run in the Workflow board's order**: what it works, the reply,
 * the tests, the Checks, and the stop at the foot.
 */

/**
 * The plan, as a step that makes or works it reads it: **a small card of its
 * groups, each with how many tasks it holds, and a press opens Plan** (owner,
 * 29 Sep 2026, `dco5`, after `25i2` and `nm0h` took the task list away). The
 * tasks themselves are the Plan destination's.
 */
export type WorkflowInspectorPlan = {
  groups: readonly { id: string; name: string; tasks: string }[];
  /** Opens the Plan destination. Absent draws the card as no control. */
  onOpen?: () => void;
};

/** Why no test runs here, and the issue that would change it. */
export type WorkflowInspectorTestsAbsent = {
  says: string;
  /** The issue that builds what is missing, as a link a person can open. */
  issue?: { href: string; label: string };
};

/** One task under the step or group. */
export type WorkflowInspectorTask = {
  id: string;
  title: string;
  /** What it is doing, in the draft's own word — `working`, `done`, `dropped`. */
  said: string;
  /** Short facts: `3 files`, `12 turns`, `$0.31`. Values, never sentences. */
  facts?: readonly string[];
  /** Why it is worth a second look — a task a later task edited after it finished. */
  flag?: string;
};

/** One Check at the boundary. */
export type WorkflowInspectorCheck = {
  name: string;
  /** What it came to. Absent where it has not run. */
  outcome?: string;
  named?: FactChipNamed;
  /**
   * Where the gate has it right now, while it runs the step's Checks: running,
   * with `outcome` its elapsed time, or waiting its turn. Absent once written.
   */
  live?: "running" | "waiting";
};

/** One case that runs at this boundary. Drawn apart from the Checks above. */
export type WorkflowInspectorTest = {
  id: string;
  title: string;
  /** What the run came to, or why there is nothing to run. */
  outcome?: string;
  named?: FactChipNamed;
};

/** One Drone that worked this step, as a row that opens it. */
export type WorkflowInspectorRunning = {
  id: string;
  /** `Drone on T5`. */
  label: string;
  /** Where it has got to, on the step machine's marks: running, advanced, stopped. */
  activity: StepActivity;
  /** The same, in words, for somebody who cannot see the mark. */
  said: string;
  /** Its task, how long and what it has spent: `T5 · 12m · 14 turns`. */
  says: string;
};

/** A Drone a redirect could reach. */
export type WorkflowInspectorDrone = { id: string; label: string };

export type WorkflowInspectorRedirect = Omit<DroneMessageBoxProps, "placeholder"> & {
  /**
   * The Drones this box could reach. **One is a sentence, several are a
   * picker** — a group running two tasks has two Drones, and a box that did
   * not say which one it reached would send a correction to either.
   */
  drones: readonly WorkflowInspectorDrone[];
  /** Which one it is addressed to. */
  reaches?: string;
  onReaches?: (id: string) => void;
};

/** One line of a task's log, newest last. */
export type WorkflowInspectorLine = {
  id: string;
  /** When it was said, already formatted. */
  at?: string;
  said: string;
  /**
   * `said` is the Drone's own words, so it is drawn as the markdown it was
   * written in. **Absent is a line Armada assembled** — a call and its
   * argument, a Check — and that stays literal: a glob in a call's argument,
   * read as markdown, loses its stars to emphasis.
   */
  words?: true;
};

/**
 * What one task is, read whole. **Drawn only for `kind: "task"`** — a step and
 * a group carry tasks, and a task carries what it was told and what it wrote.
 */
export type WorkflowInspectorTaskReading = {
  /** What its Drone was told — the planner's words, never a paraphrase. */
  brief?: string;
  briefAbsent?: string;
  /** The repository-relative paths it claims. */
  scope?: readonly string[];
  scopeAbsent?: string;
  /** The tasks it runs beside, by id. Empty is a task that runs alone. */
  beside?: readonly string[];
  /** The last thing it wrote — a path, and what the edit was. */
  lastEdit?: { path: string; says?: string };
  lastEditAbsent?: string;
  /** Its own lines. Bounded by the caller, which says what it left out. */
  log?: readonly WorkflowInspectorLine[];
  logAbsent?: string;
};

export type WorkflowInspectorProps = WorkflowInspectorTaskReading & {
  /** The step's label, the group's name, or the task's id and title. */
  name: string;
  kind: "step" | "group" | "task";
  /** Where it sits, over the name in caps — `Step 2`. */
  eyebrow?: string;
  /** Its state, as the board's pill under the name. */
  state?: { activity: StepActivity; said: string };
  /** What it is doing now, as a sentence. Absent where the pill says it. */
  doing?: string;
  /** The plan this step makes or works. Drawn in place of its tasks. */
  plan?: WorkflowInspectorPlan;
  /** Why no test runs at this boundary, where none does. */
  testsAbsent?: WorkflowInspectorTestsAbsent;
  tasks?: readonly WorkflowInspectorTask[];
  /** Why there are no tasks, where there are none. */
  tasksAbsent?: string;
  checks?: readonly WorkflowInspectorCheck[];
  checksAbsent?: string;
  tests?: readonly WorkflowInspectorTest[];
  /**
   * The boundary this reading sits behind, where it failed: the Check that
   * broke it, how many times it has been run again, and what the next Drone is
   * told. Absent everywhere else.
   */
  failure?: { says: string; retrySays?: string; toldNext?: string };
  redirect?: WorkflowInspectorRedirect;
  /**
   * Every Drone that worked this step, running or not, each one a press away
   * (owner, 29 Sep 2026: `losq`, *I will never know which drone to message*;
   * `hzj4` once the drones view landed; and *all drones that ran during that
   * step … even if its not running anymore*). Drawn where the redirect sat; a
   * press opens that Drone, in the Drones tab with a way back here.
   */
  running?: {
    rows: readonly WorkflowInspectorRunning[];
    /** Opens one. Absent draws the rows as facts rather than presses. */
    onOpen?: (id: string) => void;
  };
  /** Hold to stop what is running here. Absent where nothing is running. */
  stop?: Pick<HoldButtonProps, "children" | "askLabel" | "description" | "onCommit" | "onAsk" | "disabled" | "pending">;
  /**
   * Take the panel off what it is reading. **Drawn where the caller draws this
   * panel on a layer** — Helm's dock is closed by a Close in its own head, and
   * a panel over a canvas is closed the same way. Absent where the panel is
   * part of the flow and there is nothing to close it back to.
   */
  onClose?: () => void;
  /**
   * Drawn in the app's own panel rather than a frame of its own (owner, 29
   * Sep 2026: *all of our panels open to the full height of the app. This one
   * should be no different*). The `Sheet` draws the head: the name, its state
   * under it, where it sits, the way back and Close. **It dims what is under
   * it**, as Record's, Drones' and Plan's do (owner, 30 Sep 2026).
   */
  sheet?: { floor?: boolean; back?: SheetBack | undefined };
};

function Region({ name, children }: { name: string; children: React.ReactNode }) {
  return (
    <section className="armada-wf-inspector__region" aria-label={name}>
      <h4 className="armada-wf-inspector__eyebrow">{name}</h4>
      {children}
    </section>
  );
}

/** The plan's groups on one small card. A button where it opens Plan. */
function PlanCard({ plan }: { plan: WorkflowInspectorPlan }) {
  const rows = (
    <ul className="armada-wf-inspector__plan-groups">
      {plan.groups.map((group) => (
        <li key={group.id}>
          <span>{group.name}</span>
          <span className="armada-wf-inspector__plan-tasks">{group.tasks}</span>
        </li>
      ))}
    </ul>
  );
  return plan.onOpen === undefined ? (
    <div className="armada-wf-inspector__plan">{rows}</div>
  ) : (
    <button type="button" className="armada-wf-inspector__plan" aria-label="Open the plan" onClick={plan.onOpen}>
      {rows}
    </button>
  );
}

/** One Drone's row: its mark, whose it is, and what it has done. */
function DroneRow({ row }: { row: WorkflowInspectorRunning }) {
  return (
    <>
      <StepActivityMark activity={row.activity} label={row.said} />
      <span className="armada-wf-inspector__drone-name">{row.label}</span>
      <span className="armada-wf-inspector__drone-says">{row.says}</span>
    </>
  );
}

function Absent({ said }: { said: string }) {
  return (
    <p className="armada-wf-inspector__absent" role="note">
      {said}
    </p>
  );
}

/** The directory half of a path, trailing separator kept — `PathChip`'s rule. */
function splitPath(path: string): { directory?: string; basename: string } {
  const cut = path.lastIndexOf("/");
  if (cut < 0) return { basename: path };
  return { directory: path.slice(0, cut + 1), basename: path.slice(cut + 1) };
}

/** What a task was told, what it claims, what it wrote and what it said. */
function TaskRegions({ reading }: { reading: WorkflowInspectorTaskReading }) {
  const beside = reading.beside ?? [];
  const scope = reading.scope ?? [];
  const log = reading.log ?? [];
  return (
    <>
      <Region name="What its Drone was told">
        {reading.brief === undefined ? (
          <Absent said={reading.briefAbsent ?? "No brief was recorded for this task."} />
        ) : (
          <div className="armada-wf-inspector__brief">
            <Prose text={reading.brief} />
          </div>
        )}
      </Region>

      <Region name="What it may touch">
        {scope.length === 0 ? (
          <Absent said={reading.scopeAbsent ?? "The planner named no files for this task."} />
        ) : (
          <ul className="armada-wf-inspector__scope">
            {scope.map((path) => (
              <li key={path}>
                <PathChip {...splitPath(path)} title={path} />
              </li>
            ))}
          </ul>
        )}
      </Region>

      <Region name="What it runs beside">
        {beside.length === 0 ? (
          <Absent said="Nothing else in its group runs at the same time." />
        ) : (
          <p className="armada-wf-inspector__beside">{beside.join(", ")}</p>
        )}
      </Region>

      <Region name="Its last edit">
        {reading.lastEdit === undefined ? (
          <Absent said={reading.lastEditAbsent ?? "Nothing this task wrote has been read yet."} />
        ) : (
          <PathChip
            {...splitPath(reading.lastEdit.path)}
            title={reading.lastEdit.path}
            {...(reading.lastEdit.says === undefined ? {} : { note: reading.lastEdit.says })}
          />
        )}
      </Region>

      <Region name="Its log">
        {log.length === 0 ? (
          <Absent said={reading.logAbsent ?? "This task has said nothing yet."} />
        ) : (
          <ul className="armada-wf-inspector__log">
            {log.map((line) => (
              <li key={line.id}>
                {line.at === undefined ? null : (
                  <span className="armada-wf-inspector__at mono">{line.at}</span>
                )}
                {line.words === true ? (
                  <div className="armada-wf-inspector__said">
                    <Prose text={line.said} />
                  </div>
                ) : (
                  <span className="armada-wf-inspector__said">{line.said}</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </Region>
    </>
  );
}

export function WorkflowInspector({
  name,
  kind,
  eyebrow,
  state,
  doing,
  plan,
  testsAbsent,
  tasks = [],
  tasksAbsent,
  checks = [],
  checksAbsent,
  tests = [],
  failure,
  redirect,
  running,
  stop,
  onClose,
  sheet,
  ...reading
}: WorkflowInspectorProps) {
  const pill =
    state === undefined ? null : (
      <span className="armada-wf-inspector__state" data-activity={state.activity}>
        <StepActivityMark activity={state.activity} label={state.said} />
        <span>{state.said}</span>
      </span>
    );
  const regions = (
    <>
      {failure === undefined ? null : (
        <Region name="Why this boundary stopped">
          <p className="armada-wf-inspector__failed">{failure.says}</p>
          {failure.retrySays === undefined ? null : (
            <FactChip named="failed">{failure.retrySays}</FactChip>
          )}
          {failure.toldNext === undefined ? null : (
            <pre className="armada-wf-inspector__told">{failure.toldNext}</pre>
          )}
        </Region>
      )}

      {kind === "task" ? <TaskRegions reading={reading} /> : null}

      {kind === "task" ? null : plan !== undefined ? (
        <Region name="The plan">
          <PlanCard plan={plan} />
        </Region>
      ) : kind === "step" ? null : (
        <Region name="Its tasks">
          {tasks.length === 0 ? (
            <Absent said={tasksAbsent ?? "No tasks are recorded here."} />
          ) : (
            <ul className="armada-wf-inspector__tasks">
              {tasks.map((task) => (
                <li className="armada-wf-inspector__task" key={task.id}>
                  <span className="armada-wf-inspector__task-name">{task.title}</span>
                  <span className="armada-wf-inspector__values">
                    <FactChip>{task.said}</FactChip>
                    {(task.facts ?? []).map((fact) => (
                      <FactChip key={fact}>{fact}</FactChip>
                    ))}
                  </span>
                  {task.flag === undefined ? null : (
                    <p className="armada-wf-inspector__flag" role="note">
                      {task.flag}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          )}
        </Region>
      )}

      {running === undefined || running.rows.length === 0 ? null : (
        <Region name="Drones">
          <ul className="armada-wf-inspector__drones">
            {running.rows.map((row) => (
              <li key={row.id}>
                {running.onOpen === undefined ? (
                  <span className="armada-wf-inspector__drone">
                    <DroneRow row={row} />
                  </span>
                ) : (
                  <button
                    type="button"
                    className="armada-wf-inspector__drone"
                    onClick={() => running.onOpen?.(row.id)}
                  >
                    <DroneRow row={row} />
                  </button>
                )}
              </li>
            ))}
          </ul>
        </Region>
      )}

      {redirect === undefined ? null : (
        <Region name="Redirect a Drone">
          {redirect.drones.length === 0 ? null : redirect.drones.length === 1 ? (
            <p className="armada-wf-inspector__reaches" role="note">
              Reaches {redirect.drones[0]?.label}
            </p>
          ) : (
            <Select
              label="Reaches"
              value={redirect.reaches}
              onChange={(event) => redirect.onReaches?.(event.currentTarget.value)}
            >
              {redirect.drones.map((drone) => (
                <option value={drone.id} key={drone.id}>
                  {drone.label}
                </option>
              ))}
            </Select>
          )}
          <DroneMessageBox
            value={redirect.value}
            onChange={redirect.onChange}
            onSend={redirect.onSend}
            disabled={redirect.disabled}
            disabledReason={redirect.disabledReason}
            waiting={redirect.waiting}
          />
        </Region>
      )}

      {kind === "task" ? null : (
        <>
        {/* Apart from the Checks below, and never folded into them. Where no
            case runs here the band says what is missing and which issue builds
            it — the owner's `frpl`: a line that named Fleet and nothing else
            told him neither. */}
        {tests.length === 0 && testsAbsent === undefined ? null : (
          <Region name={kind === "step" ? "Tests at this step" : "Tests at this boundary"}>
            {tests.length === 0 && testsAbsent !== undefined ? (
              <p className="armada-wf-inspector__note" role="note">
                {testsAbsent.says}
                {testsAbsent.issue === undefined ? null : (
                  <>
                    {" "}
                    <a
                      className="armada-wf-inspector__link"
                      href={testsAbsent.issue.href}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {testsAbsent.issue.label}
                    </a>
                  </>
                )}
              </p>
            ) : (
              <ul className="armada-wf-inspector__rows">
                {tests.map((test) => (
                  <li className="armada-wf-inspector__row" key={test.id}>
                    <span className="armada-wf-inspector__row-name">{test.title}</span>
                    <FactChip named={test.named}>{test.outcome ?? "not covered"}</FactChip>
                  </li>
                ))}
              </ul>
            )}
          </Region>
        )}

        <Region name={kind === "step" ? "Checks at this step" : "Checks at this boundary"}>
          {checks.length === 0 ? (
            <Absent said={checksAbsent ?? "No Check runs here."} />
          ) : (
            <ul className="armada-wf-inspector__rows">
              {checks.map((check) => (
                <li className="armada-wf-inspector__row" key={check.name} data-live={check.live}>
                  <span className="armada-wf-inspector__row-name">
                    {check.live === "running" ? (
                      <StepActivityMark activity="running" label="running" pulsing />
                    ) : null}
                    {check.name}
                  </span>
                  <FactChip named={check.named}>{check.outcome ?? "not run"}</FactChip>
                </li>
              ))}
            </ul>
          )}
        </Region>
        </>
      )}

      {stop === undefined ? null : (
        <div className="armada-wf-inspector__acts">
          <HoldButton {...stop} />
        </div>
      )}
    </>
  );

  if (sheet !== undefined) {
    return (
      <Sheet
        open
        floating
        floor={sheet.floor ?? false}
        title={name}
        {...(pill === null ? {} : { subtitle: pill })}
        {...(eyebrow === undefined ? {} : { leading: <span className="armada-wf-inspector__leading">{eyebrow}</span> })}
        back={sheet.back}
        closeLabel="Close"
        closeBinding="Esc"
        onClose={onClose ?? (() => {})}
      >
        <div className="armada-wf-inspector armada-wf-inspector--sheet" aria-label={`${name}, ${kind}`}>
          {regions}
        </div>
      </Sheet>
    );
  }

  return (
    <div className="armada-wf-inspector armada-glass" aria-label={`${name}, ${kind}`} role="region">
      {/* The head holds still and the regions under it scroll — Helm's dock's
          own arrangement (`TheShell.css`, `__dock-head` and `__dock-body`), so
          what this panel is, and the way out of it, stay on screen however far
          down a log somebody has read. */}
      <header className="armada-wf-inspector__head">
        <div className="armada-wf-inspector__titles">
          {eyebrow === undefined ? null : <p className="armada-wf-inspector__eyebrow">{eyebrow}</p>}
          <h3 className="armada-wf-inspector__name">{name}</h3>
          {pill}
          {doing === undefined ? null : <p className="armada-wf-inspector__doing">{doing}</p>}
        </div>
        {onClose === undefined ? null : (
          <Button variant="secondary" size="sm" ground="card" onClick={onClose}>
            Close
          </Button>
        )}
      </header>

      <div className="armada-wf-inspector__body">{regions}</div>
    </div>
  );
}

