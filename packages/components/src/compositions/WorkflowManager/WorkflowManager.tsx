import { Ban, Check, CircleDashed, CopyPlus, GitCompare, GitPullRequestArrow, Layers, Link2, ListMinus, ListPlus, LoaderCircle, Plus, Trash2, Undo2 } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Table, TableBody, TableCell, TableHead, TableHeaderCell, TableRow } from "../../primitives/Table/Table";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Clamped } from "../Clamped/Clamped";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { SentFromComposer } from "../SessionComposer/SessionComposer";
import { STEP_FIELDS, stepChangesOf } from "./diff";
import type { StepChange } from "./diff";
import { RuleChangeCard, WorkspaceEditor } from "./WorkspaceEditor";
import type { StepRows, WorkspaceEditing } from "./WorkspaceEditor";
import { STEP_GLYPHS, keyOf } from "./types";
import type {
  AgentState,
  Drift,
  DriftedJob,
  ManagedStep,
  ManagedWorkflow,
  Suggestion,
  Suggestions,
  WorkflowActAnswer,
  WorkflowRef,
  WorkflowSource,
} from "./types";

/**
 * Workflows as a person manages them: a list grouped by where each comes from, and the picked one drawn
 * as its steps, one card each, with the words the agent is told in a box that is edited where it stands.
 * The decision of 27 Oct, a feature Job walks before it tests.
 *
 * **Armada's rows are read-only** in the column of steps: Make my copy puts one under Yours; Propose to Armada
 * asks for the change to land for every install. A repository's copy and Yours are edited, compared with
 * Armada's, reset to it, shared and deleted. A change of shape is typed to an agent, and nothing saves until
 * its diff is approved. **In the Workspace's lanes every row is edited**: the first change to Armada's makes
 * the person's own copy behind it, and the box that tells an agent what to change stands over the lanes.
 *
 * Its columns are `data-col`, for the System page's keys: up and down pick a row, Enter goes into the
 * steps, and `E` edits the focused step.
 */
export type WorkflowManagerProps = {
  /** The repository's name, for the caption of its group. */
  repository: string;
  /** Fleet's sentence for why the workflows could not be read or one's steps were refused, drawn above the list. */
  notice?: string | undefined;
  workflows: readonly ManagedWorkflow[];
  drift: Drift;
  picked: WorkflowRef | null;
  onPick: (one: WorkflowRef) => void;
  agent: AgentState;
  /** Every act answers with Fleet's sentence where it was refused, and with nothing where it went through. */
  onMakeCopy: (one: WorkflowRef) => Promise<WorkflowActAnswer>;
  /** The Job it started, to open; or Fleet's sentence. `note` is what to say beside it where the brief was not sent. */
  onPropose: (one: WorkflowRef) => Promise<WorkflowActAnswer | { jobId: string; note?: string }>;
  onSaveWords: (one: WorkflowRef, stepId: string, words: string) => Promise<WorkflowActAnswer>;
  onReset: (one: WorkflowRef) => Promise<WorkflowActAnswer>;
  onDelete: (one: WorkflowRef) => Promise<WorkflowActAnswer>;
  /** What stands in for the Share button: the acts on one piece of the person's own. Absent draws none. */
  actsFor?: (one: WorkflowRef) => ReactNode;
  onAsk: (one: WorkflowRef, request: string) => Promise<WorkflowActAnswer>;
  onApprove: (one: WorkflowRef) => Promise<WorkflowActAnswer>;
  onDismiss: (one: WorkflowRef) => Promise<WorkflowActAnswer>;
  onSendBack: (one: WorkflowRef, notes: string) => Promise<WorkflowActAnswer>;
  /** A job that builds what a request Armada can't do yet needs: the Job it started, to open; or Fleet's sentence. */
  onBuildSupport: (one: WorkflowRef) => Promise<WorkflowActAnswer | { jobId: string; note?: string }>;
  /** The Job a press started, as the shared Job card, once the host holds it. Absent or null, a line says it started. */
  jobFor?: (jobId: string) => ReactNode;
  /** The suggested changes waiting, and the step one is being drawn up for (`workflow/step`). */
  suggestions: Suggestions;
  suggesting: string | null;
  onOpenJob: (one: WorkflowRef, jobId: string) => void;
  onNotDrift: (one: WorkflowRef, stepId: string, driftId: string) => Promise<WorkflowActAnswer>;
  onSuggest: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
  onApproveSuggestion: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
  onDismissSuggestion: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
  /** New workflow: opens the sheet for a Job that makes one. Absent draws no button. */
  onNew?: () => void;
  /** The Workspace the workflows are edited in: the picked one draws as lanes with rules between them, in place of its column of steps. Absent draws the column. */
  workspace?: WorkspaceEditing;
};

/** What a row adds to its ref beyond the id and the place: the Manifest it was read under, and the file it was left out at. */
const refOf = (one: ManagedWorkflow): Pick<WorkflowRef, "manifestId" | "file"> => ({
  ...(one.manifestId === undefined ? {} : { manifestId: one.manifestId }),
  ...(one.leftOut === undefined ? {} : { file: one.leftOut.file }),
});

/** What a copy replaces, said at the glyph on its row. */
function replacesOf(one: ManagedWorkflow, all: readonly ManagedWorkflow[]): string | undefined {
  if (one.leftOut !== undefined) return undefined;
  const has = (source: WorkflowSource) => all.some((other) => other.id === one.id && other.source === source && other.leftOut === undefined);
  if (one.source === "repository" && has("armada")) return "This repository's copy replaces Armada's";
  if (one.source === "yours" && has("repository")) return "Yours replaces this repository's";
  if (one.source === "yours" && has("armada")) return "Yours replaces Armada's";
  return undefined;
}

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

const sentencesOf = (words: string) => words.split(/(?<=[.!?])\s+/).filter((one) => one !== "");

export function WorkflowManager(props: WorkflowManagerProps) {
  const { repository, workflows, picked, onPick } = props;
  const [query, setQuery] = useState("");
  const search = useRef<HTMLInputElement>(null);
  // Words typed and not saved, kept across leaving a workflow.
  const [drafts, setDrafts] = useState<Readonly<Record<string, string>>>({});

  const groups = useMemo(() => {
    const wanted = query.trim().toLowerCase();
    const found = workflows.filter((one) => one.id.toLowerCase().includes(wanted));
    // A repository's copies sit in a group of its own, in the order the repositories were listed.
    const fromRepositories = [...new Set(found.filter((one) => one.source === "repository").map((one) => one.manifestId ?? ""))].map((from) => {
      const rows = found.filter((one) => one.source === "repository" && (one.manifestId ?? "") === from);
      return { key: `repository@${from}`, label: `${rows[0]?.repository ?? repository} repository`, rows };
    });
    return [
      { key: "armada", label: "Armada", rows: found.filter((one) => one.source === "armada") },
      ...fromRepositories,
      { key: "yours", label: "Yours", rows: found.filter((one) => one.source === "yours") },
    ].filter((group) => group.rows.length > 0);
  }, [workflows, query, repository]);

  const shown = groups.flatMap((group) => group.rows);
  const pickedHere = picked !== null && shown.some((one) => keyOf(one) === keyOf(picked));
  const current = picked === null ? undefined : workflows.find((one) => keyOf(one) === keyOf(picked));

  // `F` reaches this search from a page with nothing focused; `/` stays System's own search.
  useEffect(() => {
    const reach = (event: globalThis.KeyboardEvent) => {
      if (event.key.toLowerCase() !== "f" || event.metaKey || event.ctrlKey || event.altKey || document.activeElement !== document.body) return;
      event.preventDefault();
      event.stopPropagation();
      search.current?.focus();
    };
    window.addEventListener("keydown", reach, true);
    return () => window.removeEventListener("keydown", reach, true);
  }, []);

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const typing = (event.target as HTMLElement).closest("input, textarea, select") !== null;
    if (event.key.toLowerCase() === "n" && !typing && !event.metaKey && !event.ctrlKey && !event.altKey && props.onNew !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      props.onNew();
      return;
    }
    if (event.key.toLowerCase() !== "f" || typing || event.metaKey || event.ctrlKey || event.altKey) return;
    event.preventDefault();
    event.stopPropagation();
    search.current?.focus();
    search.current?.select();
  };

  return (
    <div className="armada-wm" onKeyDown={onKeyDown}>
      <div className="armada-wm__list" data-col="list">
        {props.onNew === undefined ? null : (
          <Button variant="primary" size="default" className="armada-wm__new" onClick={props.onNew}>
            <Plus size={16} strokeWidth={2} aria-hidden />
            New workflow
          </Button>
        )}
        <Input
          ref={search}
          type="search"
          aria-label="Search workflows"
          placeholder="Find workflow"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape" && query !== "") {
              event.preventDefault();
              event.stopPropagation();
              setQuery("");
            }
          }}
          trailing={<Kbd>F</Kbd>}
        />
        {props.notice === undefined ? null : (
          <p className="armada-wm__said" role="alert" data-refused>
            {props.notice}
          </p>
        )}
        <h3 className="armada-wm__kept" aria-hidden="true">
          Kept in
        </h3>
        <div className="armada-wm__groups">
          {groups.map((group) => (
            <div key={group.key} role="group" aria-label={group.label} className="armada-wm__group">
              <h3 className="armada-wm__caption" aria-hidden="true">
                {group.label}
              </h3>
              <ul className="armada-wm__rows">
                {group.rows.map((one) => {
                  const on = picked !== null && keyOf(one) === keyOf(picked);
                  const replaces = replacesOf(one, workflows);
                  const out = one.leftOut;
                  return (
                    <li key={keyOf(one)} className="armada-wm__row">
                      <button
                        type="button"
                        className="armada-wm__entry"
                        data-nav-item=""
                        aria-current={on ? "true" : undefined}
                        tabIndex={on || (!pickedHere && one === shown[0]) ? 0 : -1}
                        onClick={() => onPick({ id: one.id, source: one.source, ...refOf(one) })}
                      >
                        {one.id}
                      </button>
                      <span className="armada-wm__replaces">
                        {out !== undefined ? (
                          <Tooltip label={out.said}>
                            <span role="img" aria-label="Left out">
                              <Ban size={14} strokeWidth={2} aria-hidden />
                            </span>
                          </Tooltip>
                        ) : replaces === undefined ? null : (
                          <Tooltip label={replaces}>
                            <span role="img" aria-label={replaces}>
                              <Layers size={14} strokeWidth={2} aria-hidden />
                            </span>
                          </Tooltip>
                        )}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>
      </div>
      <div className="armada-wm__detail" data-col="detail">
        {current === undefined ? null : (
          <Detail
            // In the lanes a copy made on the first edit swaps the row for Yours; the pick, the tab and the panel stay.
            key={props.workspace === undefined ? keyOf(current) : current.id}
            {...props}
            workflow={current}
            armada={workflows.find((one) => one.source === "armada" && one.id === current.id && one.leftOut === undefined)}
            drafts={drafts}
            setDrafts={setDrafts}
          />
        )}
      </div>
    </div>
  );
}

type DetailProps = WorkflowManagerProps & {
  workflow: ManagedWorkflow;
  /** Armada's workflow of the same id, which a copy is compared with and reset to. */
  armada: ManagedWorkflow | undefined;
  drafts: Readonly<Record<string, string>>;
  setDrafts: (next: (was: Readonly<Record<string, string>>) => Readonly<Record<string, string>>) => void;
};

function Detail({ workflow, armada, drafts, setDrafts, drift, agent, suggestions, suggesting, actsFor, jobFor, ...acts }: DetailProps) {
  const ref: WorkflowRef = { id: workflow.id, source: workflow.source, ...refOf(workflow) };
  const key = keyOf(workflow);
  const editable = workflow.source !== "armada" && workflow.leftOut === undefined;
  const comparable = editable && armada !== undefined;
  const [comparing, setComparing] = useState(false);
  // What the last act said, as a fact beside the acts: or Fleet's sentence for why it was refused, or the Job a proposal started.
  const [said, setSaid] = useState<{ words: string; refused?: true; job?: string } | null>(null);
  /** Runs an act; `words` is said when it went through. The answer is whether it did. */
  const run = async (act: () => Promise<WorkflowActAnswer>, words?: string): Promise<boolean> => {
    const refused = await act();
    if (refused !== undefined) setSaid({ words: refused, refused: true });
    else setSaid((was) => (words !== undefined ? { words } : was?.refused === true ? null : was));
    return refused === undefined;
  };
  const lanes = acts.workspace !== undefined;
  // **In the lanes the first change on Armada's workflow makes the person's own copy**, once, and the change is written to it.
  const yours: WorkflowRef = { id: workflow.id, source: "yours", ...(workflow.manifestId === undefined ? {} : { manifestId: workflow.manifestId }) };
  const copying = useRef<Promise<WorkflowActAnswer> | undefined>(undefined);
  useEffect(() => {
    copying.current = undefined;
  }, [workflow.source]);
  /** Fleet's sentence where the copy was refused. */
  const ensureCopy = (): Promise<WorkflowActAnswer> => {
    if (workflow.source !== "armada") return Promise.resolve(undefined);
    copying.current ??= acts.onMakeCopy(ref).then((refused) => {
      if (refused !== undefined) {
        copying.current = undefined;
        setSaid({ words: refused, refused: true });
      }
      return refused;
    });
    return copying.current;
  };
  /** Writes a change to the workflow's own definition, or to the copy the change makes. */
  const inCopy = async (act: (to: WorkflowRef) => Promise<WorkflowActAnswer>): Promise<WorkflowActAnswer> => {
    if (workflow.source !== "armada") return act(ref);
    return (await ensureCopy()) ?? act(yours);
  };
  const first = workflow.source === "armada" || comparable || (workflow.leftOut !== undefined && armada !== undefined);
  const header = workflow.leftOut === undefined ? (
    <p className="armada-wm__for">
      <span className="armada-wm__fact-label">For requests</span>
      <span>{workflow.forRequests}</span>
    </p>
  ) : (
    <p className="armada-wm__for">
      <span className="armada-wm__fact-label">Left out</span>
      <span>{workflow.leftOut.said}</span>
    </p>
  );
  /** One step as its card: in the column, or in the inspector when the lanes pick it. */
  const stepCard = (step: ManagedStep, at: number, rows?: StepRows): ReactNode => (
    <Step
      key={step.id}
      n={at + 1}
      first={at === 0 && acts.workspace === undefined}
      step={step}
      keep={`${key}/${step.id}`}
      editable={editable || lanes}
      live={lanes}
      {...(rows === undefined ? {} : { rows })}
      draft={drafts[`${key}/${step.id}`]}
      onDraft={(words) =>
        setDrafts((was) => {
          const next = { ...was };
          if (words === undefined) delete next[`${key}/${step.id}`];
          else next[`${key}/${step.id}`] = words;
          return next;
        })
      }
      onSave={(words) => run(() => inCopy((to) => acts.onSaveWords(to, step.id, words)))}
      drifted={drift[workflow.id]?.[step.id] ?? []}
      suggestion={editable ? suggestions[workflow.id]?.[step.id] : undefined}
      suggestible={editable}
      suggesting={suggesting === `${workflow.id}/${step.id}`}
      onOpenJob={(job) => acts.onOpenJob(ref, job.jobId)}
      onNotDrift={(driftId) => void run(() => acts.onNotDrift(ref, step.id, driftId))}
      onSuggest={() => void run(() => acts.onSuggest(ref, step.id))}
      onApproveSuggestion={() => acts.onApproveSuggestion(ref, step.id)}
      onDismissSuggestion={() => acts.onDismissSuggestion(ref, step.id)}
      versus={comparing && comparable ? { armada: armada!.steps.find((one) => one.id === step.id) } : undefined}
    />
  );
  /** The box that tells an agent what to change, and the diff it comes back with. In the lanes it stands over them, on every workflow. */
  const agentBox =
    editable || lanes ? (
      <Agent
        key={lanes ? workflow.id : key}
        keep={lanes ? workflow.id : key}
        agent={agent}
        onAsk={(request) => inCopy((to) => acts.onAsk(to, request))}
        onApprove={() => acts.onApprove(ref)}
        onDismiss={() => acts.onDismiss(ref)}
        onSendBack={(notes) => acts.onSendBack(ref, notes)}
        onBuildSupport={() => acts.onBuildSupport(ref)}
        onOpenJob={(job) => acts.onOpenJob(ref, job)}
        jobFor={jobFor}
        workspace={acts.workspace}
        steps={workflow.steps}
      />
    ) : null;
  return (
    <>
      <header className="armada-wm__head">
        <div className="armada-wm__title">
          <h2 className="armada-wm__name">{workflow.id}</h2>
          {header}
        </div>
        <div className="armada-wm__acts" role="toolbar" aria-label="Acts">
          {first ? (
          <div className="armada-wm__acts-group" role="group" aria-label="Armada's">
            {workflow.source === "armada" && !lanes ? (
              <Act
                label="Make my copy"
                Glyph={CopyPlus}
                onPress={() => void run(() => acts.onMakeCopy(ref))}
              />
            ) : null}
            {comparable ? <Act label="Compare with Armada's" Glyph={GitCompare} pressed={comparing} onPress={() => setComparing(!comparing)} /> : null}
            {comparable || (workflow.leftOut !== undefined && armada !== undefined) ? (
              <Act
                label="Reset to Armada's"
                Glyph={Undo2}
                onPress={() =>
                  void run(async () => {
                    const refused = await acts.onReset(ref);
                    if (refused === undefined) setDrafts((was) => Object.fromEntries(Object.entries(was).filter(([name]) => !name.startsWith(`${key}/`))));
                    return refused;
                  }, "Matches Armada's")
                }
              />
            ) : null}
            {workflow.source === "armada" ? (
              <Act
                label="Propose to Armada"
                Glyph={GitPullRequestArrow}
                disabled={said !== null && said.refused !== true}
                onPress={() =>
                  void (async () => {
                    const proposed = await acts.onPropose(ref);
                    if (typeof proposed === "string") setSaid({ words: proposed, refused: true });
                    else setSaid({ words: proposed?.note ?? "Sent. A Job started on the Armada repository", ...(proposed === undefined ? {} : { job: proposed.jobId }) });
                  })()
                }
              />
            ) : null}
          </div>
          ) : null}
          {workflow.source !== "armada" && editable ? actsFor?.(ref) : null}
          {workflow.source === "armada" ? null : (
            <div className="armada-wm__acts-group" role="group" aria-label="Delete" data-end="">
              <Act label="Delete" Glyph={Trash2} destructive onPress={() => void run(() => acts.onDelete(ref))} />
            </div>
          )}
        </div>
        {said === null ? null : <Said {...said} onOpenJob={(job) => acts.onOpenJob(ref, job)} />}
      </header>

      {acts.workspace === undefined ? (
        <div className="armada-wm__steps">{workflow.steps.map((step, at) => stepCard(step, at))}</div>
      ) : (
        <WorkspaceEditor
          editing={acts.workspace}
          workflow={workflow}
          editable={editable}
          proposed={agent.phase === "proposed" ? agent.rules : undefined}
          stepCard={stepCard}
          panel={acts.workspace.panel}
          above={agentBox}
          onMakeCopy={() => void ensureCopy()}
          onTell={(request) => inCopy((to) => acts.onAsk(to, request))}
          onSteps={(change) =>
            void inCopy(async (to) => {
              acts.workspace!.onSteps(to, change);
              return undefined;
            })
          }
        />
      )}

      {lanes || !editable ? null : agentBox}
    </>
  );
}

/** One act of the header's toolbar: a glyph with its tooltip, named for a reader who cannot see it. */
function Act({ label, Glyph, onPress, pressed, disabled, destructive }: { label: string; Glyph: LucideIcon; onPress: () => void; pressed?: boolean; disabled?: boolean; destructive?: boolean }) {
  return (
    <Tooltip label={label}>
      <Button variant={destructive === true ? "destructive" : "secondary"} size="sm" iconOnly aria-label={label} aria-pressed={pressed} disabled={disabled} onClick={onPress}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </Button>
    </Tooltip>
  );
}

/** What an act said, beside the acts: done, or Fleet's sentence for why not, with Open Job where it started one. */
function Said({ words, refused, job, onOpenJob }: { words: string; refused?: true; job?: string; onOpenJob: (job: string) => void }) {
  return (
    <p className="armada-wm__said" role={refused === true ? "alert" : "status"} data-refused={refused}>
      {refused === true ? null : <Check size={14} strokeWidth={2} aria-hidden />}
      {words}
      {job === undefined ? null : (
        <Button variant="ghost" size="sm" onClick={() => onOpenJob(job)}>
          Open Job
        </Button>
      )}
    </p>
  );
}

type StepProps = {
  n: number;
  first: boolean;
  step: ManagedStep;
  keep: string;
  editable: boolean;
  /** The words are saved as they are left, with no Save or Discard to press. */
  live: boolean;
  /** Drawn in the side panel: the step's name and what ends it are fields in the card's rows, and the band that repeats its name is left off. */
  rows?: StepRows;
  draft: string | undefined;
  onDraft: (words: string | undefined) => void;
  /** Whether the save went through, so the words typed are let go only then. */
  onSave: (words: string) => Promise<boolean>;
  drifted: readonly DriftedJob[];
  suggestion: Suggestion | undefined;
  /** A suggestion can be asked for and approved here: only a copy that is edited. */
  suggestible: boolean;
  suggesting: boolean;
  onOpenJob: (job: DriftedJob) => void;
  onNotDrift: (driftId: string) => void;
  onSuggest: () => void;
  onApproveSuggestion: () => Promise<WorkflowActAnswer>;
  onDismissSuggestion: () => Promise<WorkflowActAnswer>;
  /** Set while comparing: Armada's step of the same id, or none where this copy added it. */
  versus: { armada: ManagedStep | undefined } | undefined;
};

function Step({
  n,
  first,
  step,
  keep,
  editable,
  live,
  rows,
  draft,
  onDraft,
  onSave,
  drifted,
  suggestion,
  suggestible,
  suggesting,
  onOpenJob,
  onNotDrift,
  onSuggest,
  onApproveSuggestion,
  onDismissSuggestion,
  versus,
}: StepProps) {
  const card = useRef<HTMLDivElement>(null);
  const id = useId();
  const Glyph = step.glyph === undefined ? undefined : STEP_GLYPHS[step.glyph];
  // Fleet's sentence for a suggestion's Approve or Dismiss it would not take, said on the card.
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const settle = async (act: () => Promise<WorkflowActAnswer>) => {
    setRefused(undefined);
    const out = await act();
    setRefused(out);
    if (out === undefined) card.current?.focus();
  };
  const changed = draft !== undefined && draft !== step.words;
  const armada = versus?.armada;
  const added = versus !== undefined && armada === undefined;
  const differs = armada !== undefined && (armada.words !== step.words || armada.ends !== step.ends || (armada.endsAny ?? []).join("\n") !== (step.endsAny ?? []).join("\n") || armada.passedOver !== step.passedOver);
  return (
    <div
      ref={card}
      role="group"
      aria-label={step.label}
      className="armada-wm-step"
      data-nav-item=""
      data-changed={changed || undefined}
      tabIndex={first ? 0 : -1}
      onKeyDown={(event) => {
        if (event.target !== event.currentTarget || !editable || event.key.toLowerCase() !== "e" || event.metaKey || event.ctrlKey || event.altKey) return;
        event.preventDefault();
        card.current?.querySelector<HTMLTextAreaElement>("textarea")?.focus();
      }}
    >
      {rows !== undefined && !added && !differs && !(changed && !live) ? null : (
      <div className="armada-wm-step__band">
        {rows !== undefined ? null : (
          <>
            <span className="armada-wm-step__n">{n}</span>
            <span className="armada-wm-step__label">{step.label}</span>
            {Glyph === undefined ? null : <Glyph size={14} strokeWidth={2} aria-hidden />}
          </>
        )}
        <span className="armada-wm-step__marks">
          {added ? (
            <Tooltip label="Not in Armada's">
              <span role="img" aria-label="Not in Armada's">
                <ListPlus size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : differs ? (
            <Tooltip label="Differs from Armada's">
              <span role="img" aria-label="Differs from Armada's">
                <GitCompare size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : null}
        </span>
        <span className="armada-wm-step__save">
          {changed && !live ? (
            <>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  onDraft(undefined);
                  card.current?.focus();
                }}
              >
                Discard
              </Button>
              <Button
                variant="secondary"
                size="sm"
                ground="sunken"
                onClick={() => {
                  void onSave(draft).then((saved) => {
                    if (!saved) return;
                    onDraft(undefined);
                    card.current?.focus();
                  });
                }}
              >
                Save
              </Button>
            </>
          ) : null}
        </span>
      </div>
      )}

      <dl className="armada-wm-step__facts">
        {rows === undefined ? null : (
          <>
            <dt id={`${id}-name`}>Name</dt>
            <dd>{rows.name(`${id}-name`)}</dd>
          </>
        )}
        {step.endsAny !== undefined && step.endsAny.length > 1 ? (
          <>
            <dt>Ends when</dt>
            <dd>
              <ul className="armada-wm-step__ends" aria-label="Ends when">
                {step.endsAny.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </dd>
          </>
        ) : rows !== undefined ? (
          <>
            <dt id={`${id}-ends`}>Ends when</dt>
            <dd>{rows.ends(`${id}-ends`)}</dd>
          </>
        ) : step.ends === "" ? null : (
          <>
            <dt>Ends when</dt>
            <dd>{step.ends}</dd>
          </>
        )}
        {rows?.passedOver !== undefined ? (
          <>
            <dt id={`${id}-passed`}>Passed over when</dt>
            <dd>{rows.passedOver(`${id}-passed`)}</dd>
          </>
        ) : step.passedOver === undefined ? null : (
          <>
            <dt>Passed over when</dt>
            <dd>{step.passedOver}</dd>
          </>
        )}
        <dt>Words</dt>
        <dd>
          {differs && !changed ? (
            <Compared against={armada.words} words={step.words} />
          ) : editable ? (
            <Textarea
              aria-label="Words"
              grow
              rows={2}
              keep={`workflows:words:${keep}`}
              value={draft ?? step.words}
              onChange={(event) => onDraft(event.target.value)}
              onBlur={() => {
                if (!live || draft === undefined || draft === step.words) return;
                void onSave(draft).then((saved) => saved && onDraft(undefined));
              }}
              onKeyDown={(event) => {
                if (event.key !== "Escape") return;
                event.preventDefault();
                event.stopPropagation();
                card.current?.focus();
              }}
            />
          ) : (
            <p className="armada-wm-step__words">{step.words}</p>
          )}
        </dd>
      </dl>

      {suggestion === undefined ? null : (
        <section
          role="group"
          aria-label="Suggested change"
          className="armada-wm-diff armada-wm-suggest"
          data-nav-item=""
          tabIndex={-1}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey) && suggestion.failed === undefined) {
              event.preventDefault();
              event.stopPropagation();
              void settle(async () => {
                const out = await onApproveSuggestion();
                if (out === undefined) onDraft(undefined);
                return out;
              });
            } else if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              void settle(onDismissSuggestion);
            }
          }}
        >
          <div className="armada-wm-diff__band">Suggested change</div>
          {suggestion.failed === undefined ? (
            <div className="armada-wm-suggest__words">
              <Compared against={step.words} words={suggestion.words} left="Before" right="After" label="Before and after" />
            </div>
          ) : (
            <p className="armada-wm__said" data-refused>
              {suggestion.failed}
            </p>
          )}
          {refused === undefined ? null : (
            <p className="armada-wm__said" role="alert" data-refused>
              {refused}
            </p>
          )}
          <div className="armada-wm-diff__acts">
            <Button variant="secondary" size="sm" onClick={() => void settle(onDismissSuggestion)}>
              Dismiss
            </Button>
            {suggestion.failed !== undefined ? null : (
              <Button
                variant="primary"
                size="sm"
                onClick={() =>
                  void settle(async () => {
                    const out = await onApproveSuggestion();
                    if (out === undefined) onDraft(undefined);
                    return out;
                  })
                }
              >
                Approve
              </Button>
            )}
          </div>
        </section>
      )}

      {drifted.length === 0 ? null : (
        <div className="armada-wm-drift">
          <div className="armada-wm-drift__head">
            <h4 className="armada-wm__caption">Drifted here</h4>
            {!suggestible || suggestion !== undefined ? null : suggesting ? (
              <Tooltip label="The agent is working">
                <span className="armada-wm-agent__working" role="img" aria-label="The agent is working">
                  <LoaderCircle size={14} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : (
              <Button variant="ghost" size="sm" onClick={onSuggest}>
                Suggest a change
              </Button>
            )}
          </div>
          <ul className="armada-wm-drift__rows">
            {drifted.map((one) => (
              <li key={one.id}>
                <div
                  role="group"
                  aria-label={`${one.job}, ${one.what}`}
                  className="armada-wm-drift__row"
                  data-nav-item=""
                  data-own-keys="n"
                  tabIndex={-1}
                  onKeyDown={(event) => {
                    if (event.metaKey || event.ctrlKey || event.altKey) return;
                    const key = event.key.toLowerCase();
                    if (key === "o") {
                      event.preventDefault();
                      onOpenJob(one);
                    } else if (key === "n") {
                      event.preventDefault();
                      onNotDrift(one.id);
                      card.current?.focus();
                    }
                  }}
                >
                  <span className="armada-wm-drift__job">{one.job}</span>
                  <span className="armada-wm-drift__what">{one.what}</span>
                  <span className="armada-wm-drift__when">{one.when}</span>
                  <span className="armada-wm-drift__mark">
                    {suggestion?.failed === undefined && suggestion?.from.includes(one.id) ? (
                      <Tooltip label="In the suggested change">
                        <span role="img" aria-label="In the suggested change">
                          <Link2 size={14} strokeWidth={2} aria-hidden />
                        </span>
                      </Tooltip>
                    ) : null}
                  </span>
                  <Button variant="ghost" size="sm" onClick={() => onOpenJob(one)}>
                    Open Job
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      onNotDrift(one.id);
                      card.current?.focus();
                    }}
                  >
                    Not a drift
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

/** A step's words beside Armada's, a sentence marked where the two part. */
function Compared({
  against,
  words,
  left = "Armada's",
  right = "This copy",
  label = "Compared with Armada's",
}: {
  against: string;
  words: string;
  left?: string;
  right?: string;
  label?: string;
}) {
  const theirs = sentencesOf(against);
  const ours = sentencesOf(words);
  return (
    <div role="group" aria-label={label} className="armada-wm-compare">
      <div className="armada-wm-compare__side">
        <span className="armada-wm__fact-label">{left}</span>
        <p className="armada-wm-step__words">
          {theirs.map((one) => (
            <span key={one} data-mark={ours.includes(one) ? undefined : "removed"}>
              {one}{" "}
            </span>
          ))}
        </p>
      </div>
      <div className="armada-wm-compare__side">
        <span className="armada-wm__fact-label">{right}</span>
        <p className="armada-wm-step__words">
          {ours.map((one) => (
            <span key={one} data-mark={theirs.includes(one) ? undefined : "added"}>
              {one}{" "}
            </span>
          ))}
        </p>
      </div>
    </div>
  );
}

/** The box that asks an agent to change the workflow, and the diff it comes back with. */
function Agent({
  keep,
  agent,
  onAsk,
  onApprove,
  onDismiss,
  onSendBack,
  onBuildSupport,
  onOpenJob,
  jobFor,
  workspace,
  steps,
}: {
  keep: string;
  agent: AgentState;
  onAsk: (request: string) => Promise<WorkflowActAnswer>;
  onApprove: () => Promise<WorkflowActAnswer>;
  onDismiss: () => Promise<WorkflowActAnswer>;
  onSendBack: (notes: string) => Promise<WorkflowActAnswer>;
  onBuildSupport: () => Promise<WorkflowActAnswer | { jobId: string; note?: string }>;
  onOpenJob: (job: string) => void;
  jobFor: ((jobId: string) => ReactNode) | undefined;
  /** Where the rules a proposal adds are named. */
  workspace: WorkspaceEditing | undefined;
  steps: readonly ManagedStep[];
}) {
  const [asked, setAsked] = useState("");
  const [notes, setNotes] = useState<string | null>(null);
  // Fleet's sentence for the last act it would not take, said on the card until the next act.
  const [refused, setRefused] = useState<string | undefined>(undefined);
  // The job a request Armada can't do yet started, said over the box once the request has closed.
  const [started, setStarted] = useState<{ words: string; job?: string } | null>(null);
  const diff = useRef<HTMLElement>(null);
  const missingId = useId();
  /** Runs an act and keeps its refusal; the answer is whether it went through. */
  const settle = async (act: () => Promise<WorkflowActAnswer>): Promise<boolean> => {
    setRefused(undefined);
    const out = await act();
    setRefused(out);
    return out === undefined;
  };
  /** The composer is given the words back where Fleet refused them. */
  const send = async (sent: SentFromComposer): Promise<boolean> => {
    const request = sent.text.trim();
    if (request === "") return false;
    if (!(await settle(() => onAsk(request)))) return false;
    setStarted(null);
    setAsked(request);
    return true;
  };
  const build = async () => {
    setRefused(undefined);
    const out = await onBuildSupport();
    if (typeof out === "string") setRefused(out);
    else setStarted({ words: out?.note ?? "A job started to build it", ...(out === undefined ? {} : { job: out.jobId }) });
  };
  const said =
    refused === undefined ? null : (
      <p className="armada-wm__said" role="alert" data-refused>
        {refused}
      </p>
    );
  useEffect(() => {
    if (agent.phase === "proposed" || agent.phase === "cannot") diff.current?.scrollIntoView({ block: "nearest" });
  }, [agent.phase]);
  useEffect(() => {
    if (agent.phase !== "proposed") setNotes(null);
  }, [agent.phase]);
  useEffect(() => {
    if (agent.phase === "idle") setRefused(undefined);
  }, [agent.phase]);
  const request = (agent.phase === "idle" ? undefined : agent.asked) ?? asked;

  if (agent.phase === "working") {
    return (
      <div className="armada-wm-agent" data-phase="working">
        <Tooltip label="The agent is working">
          <span className="armada-wm-agent__working" role="img" aria-label="The agent is working">
            <LoaderCircle size={14} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-wm-agent__asked">{request}</span>
      </div>
    );
  }

  if (agent.phase === "failed") {
    return (
      <div className="armada-wm-agent" data-phase="failed">
        <p className="armada-wm__said" role="alert" data-refused>
          {agent.reason}
        </p>
        {said}
        <div className="armada-wm-agent__acts">
          <Button variant="secondary" size="sm" onClick={() => void settle(onDismiss)}>
            Dismiss
          </Button>
        </div>
      </div>
    );
  }

  if (agent.phase === "cannot") {
    return (
      <section ref={diff} className="armada-wm-diff" role="region" aria-label="Proposed change">
        <div className="armada-wm-diff__band">Proposed change</div>
        <p className="armada-wm-diff__asked">{request}</p>
        <p className="armada-wm-diff__cannot">Armada can't do this yet</p>
        <div className="armada-wm-diff__missing">
          <h4 className="armada-wm__caption" id={missingId}>
            Missing
          </h4>
          <ul className="armada-wm-diff__missing-rows" aria-labelledby={missingId}>
            {agent.missing.map((one) => (
              <li key={one} className="armada-wm-diff__missing-row">
                <span className="armada-wm-diff__sign">
                  <Tooltip label="Not built yet">
                    <span role="img" aria-label="Not built yet">
                      <CircleDashed size={14} strokeWidth={2} aria-hidden />
                    </span>
                  </Tooltip>
                </span>
                <span>{one}</span>
              </li>
            ))}
          </ul>
        </div>
        {said}
        <div className="armada-wm-diff__acts">
          <Button variant="ghost" size="sm" onClick={() => void settle(onDismiss)}>
            Dismiss
          </Button>
          <Button variant="primary" size="sm" onClick={() => void build()}>
            Start a job to build it
          </Button>
        </div>
      </section>
    );
  }

  if (agent.phase === "proposed") {
    const { changes, unchanged } = stepChangesOf(agent.before, agent.after);
    const sendNotes = (written: string) => void settle(() => onSendBack(written));
    return (
      <section ref={diff} className="armada-wm-diff" role="region" aria-label="Proposed change">
        <div className="armada-wm-diff__band">Proposed change</div>
        <p className="armada-wm-diff__asked">{request}</p>
        <div className="armada-wm-diff__cards">
          {changes.map((one) => (
            <StepChangeCard key={`${one.kind}:${one.kind === "removed" ? one.before.id : one.after.id}`} change={one} />
          ))}
          {workspace === undefined ? null : (agent.rules ?? []).map((one) => <RuleChangeCard key={one.id} rule={one} workspace={workspace.value} steps={steps} />)}
        </div>
        {unchanged.length === 0 || agent.rules !== undefined ? null : (
          <p className="armada-wm-diff__unchanged">
            <span className="armada-wm__fact-label">Unchanged</span>
            <span>{unchanged.join(", ")}</span>
          </p>
        )}
        {notes === null ? null : (
          <div className="armada-wm-diff__notes">
            <Textarea
              label="Notes for the agent"
              rows={2}
              keep={`workflows:notes:${keep}`}
              value={notes}
              onChange={(event) => setNotes(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && (event.metaKey || event.ctrlKey) && notes.trim() !== "") {
                  event.preventDefault();
                  sendNotes(notes.trim());
                }
                if (event.key === "Escape") {
                  event.preventDefault();
                  event.stopPropagation();
                  setNotes(null);
                }
              }}
            />
          </div>
        )}
        {said}
        <div className="armada-wm-diff__acts">
          <Button variant="ghost" size="sm" onClick={() => void settle(onDismiss)}>
            Dismiss
          </Button>
          {agent.fresh === true ? null : notes === null ? (
            <Button variant="secondary" size="sm" onClick={() => setNotes("")}>
              Send back
            </Button>
          ) : (
            <Button variant="secondary" size="sm" disabled={notes.trim() === ""} onClick={() => sendNotes(notes.trim())}>
              Send notes
            </Button>
          )}
          <Button variant="primary" size="sm" onClick={() => void settle(onApprove)}>
            Approve
          </Button>
        </div>
      </section>
    );
  }

  // The job drawn as the shared Job card once the host holds it; until then, the line that says it started.
  const job = started?.job === undefined ? null : (jobFor?.(started.job) ?? null);
  return (
    <div className="armada-wm-agent" data-phase="idle">
      {started === null ? null : job !== null ? <div className="armada-wm-agent__job">{job}</div> : <Said {...started} onOpenJob={onOpenJob} />}
      {said}
      {/* The composer has no visible label of its own; this one says what the box is for. */}
      <h4 className="armada-wm__caption" aria-hidden="true">
        Tell an agent what to change
      </h4>
      <SessionComposer
        working={false}
        bare
        label="Tell an agent what to change"
        send="secondary"
        sendLabel="Send to agent"
        model={null}
        effort={null}
        models={NONE}
        efforts={NONE}
        onTune={NOBODY}
        commands={NONE}
        taggable={NONE}
        tags={NONE}
        onTags={NOBODY}
        drawn={NONE}
        onDraw={NOBODY}
        onRemoveDrawn={NOBODY}
        draftKey={`workflows:agent:${keep}`}
        onSend={send}
      />
    </div>
  );
}

const SIGNS = {
  added: { words: "Added", Glyph: ListPlus },
  removed: { words: "Removed", Glyph: ListMinus },
  changed: { words: "Changed", Glyph: GitCompare },
} as const;

/**
 * One step a proposed change touches: its place and name, how it changed, and its facts. A changed step draws each
 * fact before and after; an added one only after, and a removed one only before, struck through, its edge in the
 * diff's hue for the kind, so no column stands empty.
 */
function StepChangeCard({ change }: { change: StepChange }) {
  const before = change.kind === "added" ? undefined : change.before;
  const after = change.kind === "removed" ? undefined : change.after;
  const label = (after ?? before)!.label;
  const sign = SIGNS[change.kind];
  const sides: readonly ("Before" | "After")[] = change.kind === "changed" ? ["Before", "After"] : change.kind === "added" ? ["After"] : ["Before"];
  // A changed step also sets its place first, so a step that only moved shows what moved; every changed card has the same rows.
  const rows = [
    ...(change.kind === "changed" ? [{ name: "Place", was: String(change.was), is: String(change.n) }] : []),
    ...STEP_FIELDS.map((field) => ({ name: field.name, was: before === undefined ? "" : field.of(before), is: after === undefined ? "" : field.of(after) })),
  ];
  return (
    <div role="group" aria-label={label} className="armada-wm-diff__card" data-kind={change.kind}>
      <div className="armada-wm-diff__card-band">
        <span className="armada-wm-step__n">{change.n}</span>
        <span className="armada-wm-step__label">{label}</span>
        <span className="armada-wm-diff__card-sign">
          <Tooltip label={sign.words}>
            <span role="img" aria-label={sign.words}>
              <sign.Glyph size={14} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        </span>
      </div>
      <Table className="armada-wm-diff__fields" data-sides={sides.length}>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Field</TableHeaderCell>
            {sides.map((side) => (
              <TableHeaderCell key={side}>{side}</TableHeaderCell>
            ))}
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((field) => {
            const { was, is } = field;
            const differs = change.kind === "changed" && was !== is;
            const state = change.kind === "changed" ? (differs ? "changed" : "unchanged") : change.kind;
            return (
              <TableRow key={field.name} aria-label={`${field.name}, ${state}`} data-differs={differs || undefined}>
                <TableHeaderCell scope="row">{field.name}</TableHeaderCell>
                {sides.map((side) => (
                  <TableCell key={side} variant={change.kind === "changed" && !differs ? "secondary" : "primary"}>
                    <FieldValue
                      value={side === "Before" ? was : is}
                      words={field.name === "Words"}
                      whole={differs}
                      against={change.kind === "changed" ? (side === "Before" ? is : was) : undefined}
                      mark={side === "Before" ? "removed" : "added"}
                    />
                  </TableCell>
                ))}
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}

/**
 * One fact on one side: `nothing` where the step has none; words held to a few lines, a sentence marked where the
 * sides part. Words that differ are drawn `whole`, so the marked sentence is never behind View more.
 */
function FieldValue({ value, words, whole, against, mark }: { value: string; words: boolean; whole: boolean; against: string | undefined; mark: "added" | "removed" }) {
  if (value === "") return <span className="armada-wm-diff__nothing">nothing</span>;
  // Several ends, one a line as `STEP_FIELDS` reads them: the list the step's own facts draw.
  if (!words && value.includes("\n")) {
    return (
      <ul className="armada-wm-step__ends">
        {value.split("\n").map((one) => (
          <li key={one}>{one}</li>
        ))}
      </ul>
    );
  }
  if (!words) return <>{value}</>;
  const other = against === undefined ? undefined : sentencesOf(against);
  const drawn = (
    <p className="armada-wm-step__words">
      {sentencesOf(value).map((one) => (
        <span key={one} data-mark={other === undefined || other.includes(one) ? undefined : mark}>
          {one}{" "}
        </span>
      ))}
    </p>
  );
  return whole ? drawn : <Clamped lines={3}>{drawn}</Clamped>;
}
