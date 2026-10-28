import { Ban, Check, GitCompare, Layers, Link2, ListMinus, ListPlus, LoaderCircle } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
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
 * **Armada's rows are read-only.** Make my copy puts one under Yours; Propose to Armada asks for the
 * change to land for every install. A repository's copy and Yours are edited, compared with Armada's,
 * reset to it, shared and deleted. A change of shape is typed to an agent, and nothing saves until its
 * diff is approved.
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
  /** The suggested changes waiting, and the step one is being drawn up for (`workflow/step`). */
  suggestions: Suggestions;
  suggesting: string | null;
  onOpenJob: (one: WorkflowRef, jobId: string) => void;
  onNotDrift: (one: WorkflowRef, stepId: string, driftId: string) => Promise<WorkflowActAnswer>;
  onSuggest: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
  onApproveSuggestion: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
  onDismissSuggestion: (one: WorkflowRef, stepId: string) => Promise<WorkflowActAnswer>;
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
    if (event.key.toLowerCase() !== "f" || typing || event.metaKey || event.ctrlKey || event.altKey) return;
    event.preventDefault();
    event.stopPropagation();
    search.current?.focus();
    search.current?.select();
  };

  return (
    <div className="armada-wm" onKeyDown={onKeyDown}>
      <div className="armada-wm__list" data-col="list">
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
            key={keyOf(current)}
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

function Detail({ workflow, armada, drafts, setDrafts, drift, agent, suggestions, suggesting, actsFor, ...acts }: DetailProps) {
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
  return (
    <>
      <header className="armada-wm__head">
        <div className="armada-wm__title">
          <h2 className="armada-wm__name">{workflow.id}</h2>
          {header}
        </div>
        <div className="armada-wm__acts">
          {workflow.source === "armada" ? (
            <>
              <Button variant="secondary" size="sm" onClick={() => void run(() => acts.onMakeCopy(ref))}>
                Make my copy
              </Button>
              <Button
                variant="secondary"
                size="sm"
                disabled={said !== null && said.refused !== true}
                onClick={() =>
                  void (async () => {
                    const proposed = await acts.onPropose(ref);
                    if (typeof proposed === "string") setSaid({ words: proposed, refused: true });
                    else setSaid({ words: proposed?.note ?? "Sent. A Job started on the Armada repository", ...(proposed === undefined ? {} : { job: proposed.jobId }) });
                  })()
                }
              >
                Propose to Armada
              </Button>
            </>
          ) : (
            <>
              {comparable ? (
                <>
                  <Button variant="secondary" size="sm" aria-pressed={comparing} onClick={() => setComparing(!comparing)}>
                    Compare with Armada's
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      void run(async () => {
                        const refused = await acts.onReset(ref);
                        if (refused === undefined) setDrafts((was) => Object.fromEntries(Object.entries(was).filter(([name]) => !name.startsWith(`${key}/`))));
                        return refused;
                      }, "Matches Armada's")
                    }
                  >
                    Reset to Armada's
                  </Button>
                </>
              ) : null}
              {workflow.leftOut !== undefined && armada !== undefined ? (
                <Button variant="secondary" size="sm" onClick={() => void run(() => acts.onReset(ref), "Matches Armada's")}>
                  Reset to Armada's
                </Button>
              ) : null}
              {editable ? actsFor?.(ref) : null}
              <Button variant="destructive" size="sm" onClick={() => void run(() => acts.onDelete(ref))}>
                Delete
              </Button>
            </>
          )}
        </div>
        {said === null ? null : (
          <p className="armada-wm__said" role={said.refused === true ? "alert" : "status"} data-refused={said.refused}>
            {said.refused === true ? null : <Check size={14} strokeWidth={2} aria-hidden />}
            {said.words}
            {said.job === undefined ? null : (
              <Button variant="ghost" size="sm" onClick={() => acts.onOpenJob(ref, said.job!)}>
                Open Job
              </Button>
            )}
          </p>
        )}
      </header>

      <div className="armada-wm__steps">
        {workflow.steps.map((step, at) => (
          <Step
            key={step.id}
            n={at + 1}
            first={at === 0}
            step={step}
            keep={`${key}/${step.id}`}
            editable={editable}
            draft={drafts[`${key}/${step.id}`]}
            onDraft={(words) =>
              setDrafts((was) => {
                const next = { ...was };
                if (words === undefined) delete next[`${key}/${step.id}`];
                else next[`${key}/${step.id}`] = words;
                return next;
              })
            }
            onSave={(words) => run(() => acts.onSaveWords(ref, step.id, words))}
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
        ))}
      </div>

      {editable ? (
        <Agent
          key={key}
          keep={key}
          agent={agent}
          onAsk={(request) => acts.onAsk(ref, request)}
          onApprove={() => acts.onApprove(ref)}
          onDismiss={() => acts.onDismiss(ref)}
          onSendBack={(notes) => acts.onSendBack(ref, notes)}
        />
      ) : null}
    </>
  );
}

type StepProps = {
  n: number;
  first: boolean;
  step: ManagedStep;
  keep: string;
  editable: boolean;
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
  const differs = armada !== undefined && (armada.words !== step.words || armada.ends !== step.ends || armada.passedOver !== step.passedOver);
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
      <div className="armada-wm-step__band">
        <span className="armada-wm-step__n">{n}</span>
        <span className="armada-wm-step__label">{step.label}</span>
        {Glyph === undefined ? null : <Glyph size={14} strokeWidth={2} aria-hidden />}
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
          {changed ? (
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

      <dl className="armada-wm-step__facts">
        {step.ends === "" ? null : (
          <>
            <dt>Ends when</dt>
            <dd>{step.ends}</dd>
          </>
        )}
        {step.passedOver === undefined ? null : (
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
}: {
  keep: string;
  agent: AgentState;
  onAsk: (request: string) => Promise<WorkflowActAnswer>;
  onApprove: () => Promise<WorkflowActAnswer>;
  onDismiss: () => Promise<WorkflowActAnswer>;
  onSendBack: (notes: string) => Promise<WorkflowActAnswer>;
}) {
  const [text, setText] = useState("");
  const [asked, setAsked] = useState("");
  const [notes, setNotes] = useState<string | null>(null);
  // Fleet's sentence for the last act it would not take, said on the card until the next act.
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const diff = useRef<HTMLElement>(null);
  /** Runs an act and keeps its refusal; the answer is whether it went through. */
  const settle = async (act: () => Promise<WorkflowActAnswer>): Promise<boolean> => {
    setRefused(undefined);
    const out = await act();
    setRefused(out);
    return out === undefined;
  };
  const send = async () => {
    const request = text.trim();
    if (request === "") return;
    if (!(await settle(() => onAsk(request)))) return;
    setAsked(request);
    setText("");
  };
  const said =
    refused === undefined ? null : (
      <p className="armada-wm__said" role="alert" data-refused>
        {refused}
      </p>
    );
  useEffect(() => {
    if (agent.phase === "proposed") diff.current?.scrollIntoView({ block: "nearest" });
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

  if (agent.phase === "proposed") {
    const after = new Set(agent.after.map((one) => one.id));
    const before = new Set(agent.before.map((one) => one.id));
    const sendNotes = (written: string) => void settle(() => onSendBack(written));
    return (
      <section ref={diff} className="armada-wm-diff" role="region" aria-label="Proposed change">
        <div className="armada-wm-diff__band">Proposed change</div>
        <p className="armada-wm-diff__asked">{request}</p>
        <div className="armada-wm-diff__cols">
          <DiffSide label="Before" steps={agent.before} gone={(id) => !after.has(id)} sign="removed" />
          <DiffSide label="After" steps={agent.after} gone={(id) => !before.has(id)} sign="added" />
        </div>
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
          {notes === null ? (
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

  return (
    <div className="armada-wm-agent" data-phase="idle">
      {said}
      <Textarea
        label="Tell an agent what to change"
        rows={2}
        keep={`workflows:agent:${keep}`}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            void send();
          }
        }}
      />
      <div className="armada-wm-agent__acts">
        <Kbd>⌘Enter</Kbd>
        <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={() => void send()}>
          Send to agent
        </Button>
      </div>
    </div>
  );
}

function DiffSide({
  label,
  steps,
  gone,
  sign,
}: {
  label: string;
  steps: readonly ManagedStep[];
  gone: (id: string) => boolean;
  sign: "removed" | "added";
}) {
  return (
    <div role="group" aria-label={label} className="armada-wm-diff__side">
      <h4 className="armada-wm__caption">{label}</h4>
      <ol className="armada-wm-diff__steps">
        {steps.map((one, at) => {
          const marked = gone(one.id);
          const words = sign === "removed" ? "Removed" : "Added";
          return (
            <li key={one.id} className="armada-wm-diff__step" data-mark={marked ? sign : undefined}>
              <span className="armada-wm-diff__sign">
                {marked ? (
                  <Tooltip label={words}>
                    <span role="img" aria-label={words}>
                      {sign === "removed" ? <ListMinus size={14} strokeWidth={2} aria-hidden /> : <ListPlus size={14} strokeWidth={2} aria-hidden />}
                    </span>
                  </Tooltip>
                ) : null}
              </span>
              <span className="armada-wm-diff__n">{at + 1}</span>
              <span className="armada-wm-diff__label">{one.label}</span>
              <span className="armada-wm-diff__ends">{one.ends}</span>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
