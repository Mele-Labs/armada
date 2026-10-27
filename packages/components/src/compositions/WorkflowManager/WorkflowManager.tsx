import { Check, Layers, LoaderCircle, Minus, PencilLine, Plus } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { STEP_GLYPHS, keyOf } from "./types";
import type { AgentState, Drift, ManagedStep, ManagedWorkflow, WorkflowRef, WorkflowSource } from "./types";

/**
 * Workflows as a person manages them: a list grouped by where each comes from, and the picked one drawn
 * as its steps, one card each, with the words the agent is told in a box that is edited where it stands.
 * `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`.
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
  workflows: readonly ManagedWorkflow[];
  drift: Drift;
  picked: WorkflowRef | null;
  onPick: (one: WorkflowRef) => void;
  agent: AgentState;
  onMakeCopy: (one: WorkflowRef) => void;
  onPropose: (one: WorkflowRef) => void;
  onSaveWords: (one: WorkflowRef, stepId: string, words: string) => void;
  onReset: (one: WorkflowRef) => void;
  onDelete: (one: WorkflowRef) => void;
  onShare: (one: WorkflowRef) => void;
  onAsk: (one: WorkflowRef, request: string) => void;
  onApprove: (one: WorkflowRef) => void;
  onSendBack: (one: WorkflowRef, notes: string) => void;
};

const SOURCES: readonly WorkflowSource[] = ["armada", "repository", "yours"];

/** What a copy replaces, said at the glyph on its row. */
function replacesOf(one: ManagedWorkflow, all: readonly ManagedWorkflow[]): string | undefined {
  const has = (source: WorkflowSource) => all.some((other) => other.id === one.id && other.source === source);
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
    return SOURCES.map((source) => ({
      source,
      label: source === "armada" ? "Armada" : source === "yours" ? "Yours" : `${repository} repository`,
      rows: workflows.filter((one) => one.source === source && one.id.toLowerCase().includes(wanted)),
    })).filter((group) => group.rows.length > 0);
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
        <div className="armada-wm__groups">
          {groups.map((group) => (
            <div key={group.source} role="group" aria-label={group.label} className="armada-wm__group">
              <h3 className="armada-wm__caption" aria-hidden="true">
                {group.label}
              </h3>
              <ul className="armada-wm__rows">
                {group.rows.map((one) => {
                  const on = picked !== null && keyOf(one) === keyOf(picked);
                  const replaces = replacesOf(one, workflows);
                  return (
                    <li key={keyOf(one)} className="armada-wm__row">
                      <button
                        type="button"
                        className="armada-wm__entry"
                        data-nav-item=""
                        aria-current={on ? "true" : undefined}
                        tabIndex={on || (!pickedHere && one === shown[0]) ? 0 : -1}
                        onClick={() => onPick(one)}
                      >
                        {one.id}
                      </button>
                      <span className="armada-wm__replaces">
                        {replaces === undefined ? null : (
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
            armada={workflows.find((one) => one.source === "armada" && one.id === current.id)}
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

function Detail({ workflow, armada, drafts, setDrafts, drift, agent, ...acts }: DetailProps) {
  const ref: WorkflowRef = { id: workflow.id, source: workflow.source };
  const key = keyOf(workflow);
  const editable = workflow.source !== "armada";
  const comparable = editable && armada !== undefined;
  const [comparing, setComparing] = useState(false);
  // What the last act said, as a fact beside the acts.
  const [said, setSaid] = useState<string | null>(null);
  const say = (words: string, act: () => void) => () => {
    act();
    setSaid(words);
  };
  return (
    <>
      <header className="armada-wm__head">
        <div className="armada-wm__title">
          <h2 className="armada-wm__name">{workflow.id}</h2>
          <p className="armada-wm__for">
            <span className="armada-wm__fact-label">For requests</span>
            <span>{workflow.forRequests}</span>
          </p>
        </div>
        <div className="armada-wm__acts">
          {workflow.source === "armada" ? (
            <>
              <Button variant="secondary" size="sm" onClick={() => acts.onMakeCopy(ref)}>
                Make my copy
              </Button>
              <Button variant="secondary" size="sm" disabled={said !== null} onClick={say("Sent. A Job starts on the Armada repository", () => acts.onPropose(ref))}>
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
                    onClick={say("Matches Armada's", () => {
                      setDrafts((was) => Object.fromEntries(Object.entries(was).filter(([name]) => !name.startsWith(`${key}/`))));
                      acts.onReset(ref);
                    })}
                  >
                    Reset to Armada's
                  </Button>
                </>
              ) : null}
              <Button variant="secondary" size="sm" onClick={say(`Saved ${workflow.id}.json`, () => acts.onShare(ref))}>
                Share
              </Button>
              <Button variant="destructive" size="sm" onClick={() => acts.onDelete(ref)}>
                Delete
              </Button>
            </>
          )}
        </div>
        {said === null ? null : (
          <p className="armada-wm__said" role="status">
            <Check size={14} strokeWidth={2} aria-hidden />
            {said}
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
            onSave={(words) => acts.onSaveWords(ref, step.id, words)}
            drifted={drift[workflow.id]?.[step.id] ?? []}
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
  onSave: (words: string) => void;
  drifted: readonly { job: string; what: string; when: string }[];
  /** Set while comparing: Armada's step of the same id, or none where this copy added it. */
  versus: { armada: ManagedStep | undefined } | undefined;
};

function Step({ n, first, step, keep, editable, draft, onDraft, onSave, drifted, versus }: StepProps) {
  const card = useRef<HTMLDivElement>(null);
  const Glyph = STEP_GLYPHS[step.glyph];
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
        <Glyph size={14} strokeWidth={2} aria-hidden />
        <span className="armada-wm-step__marks">
          {added ? (
            <Tooltip label="Not in Armada's">
              <span role="img" aria-label="Not in Armada's">
                <Plus size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : differs ? (
            <Tooltip label="Differs from Armada's">
              <span role="img" aria-label="Differs from Armada's">
                <PencilLine size={14} strokeWidth={2} aria-hidden />
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
                  onSave(draft);
                  onDraft(undefined);
                  card.current?.focus();
                }}
              >
                Save
              </Button>
            </>
          ) : null}
        </span>
      </div>

      <dl className="armada-wm-step__facts">
        <dt>Ends when</dt>
        <dd>{step.ends}</dd>
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

      {drifted.length === 0 ? null : (
        <div className="armada-wm-drift">
          <h4 className="armada-wm__caption">Drifted here</h4>
          <ul className="armada-wm-drift__rows">
            {drifted.map((one) => (
              <li key={one.job}>
                <button type="button" className="armada-wm-drift__row">
                  <span className="armada-wm-drift__job">{one.job}</span>
                  <span className="armada-wm-drift__what">{one.what}</span>
                  <span className="armada-wm-drift__when">{one.when}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

/** A step's words beside Armada's, a sentence marked where the two part. */
function Compared({ against, words }: { against: string; words: string }) {
  const theirs = sentencesOf(against);
  const ours = sentencesOf(words);
  return (
    <div role="group" aria-label="Compared with Armada's" className="armada-wm-compare">
      <div className="armada-wm-compare__side">
        <span className="armada-wm__fact-label">Armada's</span>
        <p className="armada-wm-step__words">
          {theirs.map((one) => (
            <span key={one} data-mark={ours.includes(one) ? undefined : "removed"}>
              {one}{" "}
            </span>
          ))}
        </p>
      </div>
      <div className="armada-wm-compare__side">
        <span className="armada-wm__fact-label">This copy</span>
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
  onSendBack,
}: {
  keep: string;
  agent: AgentState;
  onAsk: (request: string) => void;
  onApprove: () => void;
  onSendBack: (notes: string) => void;
}) {
  const [text, setText] = useState("");
  const [asked, setAsked] = useState("");
  const [notes, setNotes] = useState<string | null>(null);
  const diff = useRef<HTMLElement>(null);
  const send = () => {
    if (text.trim() === "") return;
    setAsked(text.trim());
    setText("");
    onAsk(text.trim());
  };
  useEffect(() => {
    if (agent.phase === "proposed") diff.current?.scrollIntoView({ block: "nearest" });
  }, [agent.phase]);
  useEffect(() => {
    if (agent.phase !== "proposed") setNotes(null);
  }, [agent.phase]);

  if (agent.phase === "working") {
    return (
      <div className="armada-wm-agent" data-phase="working">
        <Tooltip label="The agent is working">
          <span className="armada-wm-agent__working" role="img" aria-label="The agent is working">
            <LoaderCircle size={14} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-wm-agent__asked">{asked}</span>
      </div>
    );
  }

  if (agent.phase === "proposed") {
    const after = new Set(agent.after.map((one) => one.id));
    const before = new Set(agent.before.map((one) => one.id));
    return (
      <section ref={diff} className="armada-wm-diff" role="region" aria-label="Proposed change">
        <div className="armada-wm-diff__band">Proposed change</div>
        <p className="armada-wm-diff__asked">{asked}</p>
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
                  onSendBack(notes.trim());
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
        <div className="armada-wm-diff__acts">
          {notes === null ? (
            <Button variant="secondary" size="sm" onClick={() => setNotes("")}>
              Send back
            </Button>
          ) : (
            <Button variant="secondary" size="sm" disabled={notes.trim() === ""} onClick={() => onSendBack(notes.trim())}>
              Send notes
            </Button>
          )}
          <Button variant="primary" size="sm" onClick={onApprove}>
            Approve
          </Button>
        </div>
      </section>
    );
  }

  return (
    <div className="armada-wm-agent" data-phase="idle">
      {agent.phase === "unchanged" ? (
        <p className="armada-wm__said" role="status">
          <Check size={14} strokeWidth={2} aria-hidden />
          No step changes
        </p>
      ) : null}
      <Textarea
        label="Tell an agent what to change"
        rows={2}
        keep={`workflows:agent:${keep}`}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            send();
          }
        }}
      />
      <div className="armada-wm-agent__acts">
        <Kbd>⌘Enter</Kbd>
        <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={send}>
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
                      {sign === "removed" ? <Minus size={14} strokeWidth={2} aria-hidden /> : <Plus size={14} strokeWidth={2} aria-hidden />}
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
