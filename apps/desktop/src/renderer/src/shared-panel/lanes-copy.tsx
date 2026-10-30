// The owner's acts on a Job's own copy, from its lanes (`lanes.tsx`): `+` adds a step, a lifecycle or a rule to this Job, and the changes made to it are listed under the canvas with
// Keep for every Job, which proposes the Job's workflow for the others. **A change applies to this Job at once**; Keep waits on Approve on the Workflows page, so it is not drawn as done.
// A refusal is Fleet's own sentence, drawn in the form that asked. Keys: the menu's, Esc puts the form away, and ⌘↵ adds from any field.

import { useState, type FormEvent, type KeyboardEvent } from "react";
import { BookMarked, Check, GitCompare, SquarePlus } from "lucide-react";
import { Button, DropdownMenu, Input, Select, Textarea, Tooltip } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { SessionLanes } from "@armada/components/src/compositions/LaneCanvas/build";
import type { JobCopyActs } from "@armada/screens/src/draft/sessions";

export const ADD_SAID = "Add to this Job";
export const KEEP_SAID = "Keep for every Job";
export const KEPT_SAID = "Proposed for every Job";

type Adding = "step" | "lifecycle" | "rule";

/** The words a change is listed under: the step, the rule or the lifecycle it changed, as the lanes name them. */
export function changedName(lanes: SessionLanes, change: { lane: string; item: string }): string {
  const { copy } = lanes.job;
  if (change.lane === "workflow") return copy.steps.find((one) => one.id === change.item)?.label ?? change.item;
  if (change.lane === "rule") return copy.rules.find((one) => one.id === change.item)?.label ?? change.item;
  const lifecycle = copy.lifecycles.find((one) => one.id === change.lane);
  const state = lifecycle?.states.find((one) => one.id === change.item)?.label;
  return `${lifecycle?.label ?? change.lane}${state === undefined ? "" : `: ${state}`}`;
}

/** `+` and the form it opens: nothing is drawn where nothing serves the acts. */
export function AddToJob({ sessionId, lanes, acts }: { sessionId: string; lanes: SessionLanes; acts: JobCopyActs }) {
  const [adding, setAdding] = useState<Adding | undefined>();
  const entries: DropdownMenuEntry[] = [
    ...(lanes.job.copy.steps.length === 0 ? [] : [{ kind: "item" as const, id: "step", label: "Step" }]),
    { kind: "item", id: "lifecycle", label: "Lifecycle" },
    { kind: "item", id: "rule", label: "Rule" },
  ];
  return (
    <div className="sv-lanes__add">
      <div className="sv-lanes__bar">
        <DropdownMenu triggerLabel={ADD_SAID} icon={SquarePlus} align="start" entries={entries} onSelect={(id) => setAdding(id as Adding)} />
      </div>
      {adding === undefined ? null : adding === "step" ? (
        <StepForm key="step" sessionId={sessionId} lanes={lanes} act={acts.step} onDone={() => setAdding(undefined)} />
      ) : (
        <PieceForm key={adding} sessionId={sessionId} kind={adding} act={acts.piece} onDone={() => setAdding(undefined)} />
      )}
    </div>
  );
}

/** Esc puts the form away; ⌘↵ or Ctrl↵ adds from any field. */
function keysOf(onCancel: () => void, onAdd: () => void): (event: KeyboardEvent<HTMLFormElement>) => void {
  return (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
    } else if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      onAdd();
    }
  };
}

/** Runs one act: pending while Fleet has not answered, the refusal's words kept where it said no, and the form put away where it took it. */
function useAdding(run: () => Promise<string | undefined>, onDone: () => void) {
  const [pending, setPending] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  const add = async () => {
    if (pending) return;
    setPending(true);
    setRefused(undefined);
    const said = await run();
    setPending(false);
    if (said === undefined) onDone();
    else setRefused(said);
  };
  return { pending, refused, add };
}

function StepForm({ sessionId, lanes, act, onDone }: { sessionId: string; lanes: SessionLanes; act: JobCopyActs["step"]; onDone: () => void }) {
  const { steps } = lanes.job.copy;
  const [label, setLabel] = useState("");
  const [instructions, setInstructions] = useState("");
  // After the step the Job is at, or the last one where it is past them all.
  const [after, setAfter] = useState(steps[Math.min(Math.max(lanes.at, 0), steps.length - 1)]?.id ?? "");
  const { pending, refused, add } = useAdding(
    () => act(sessionId, { kind: "add", ...(after === "" ? {} : { after }), label, ...(instructions.trim() === "" ? {} : { instructions }) }),
    onDone,
  );
  const submit = (event: FormEvent) => {
    event.preventDefault();
    void add();
  };
  return (
    <form className="sv-lanes__form" aria-label="Add a step to this Job" onSubmit={submit} onKeyDown={keysOf(onDone, () => void add())}>
      <Input label="Label" value={label} onChange={(event) => setLabel(event.target.value)} autoFocus />
      <Textarea label="Instructions" rows={3} value={instructions} onChange={(event) => setInstructions(event.target.value)} />
      <Select label="After" value={after} onChange={(event) => setAfter(event.target.value)}>
        <option value="">First</option>
        {steps.map((step) => (
          <option key={step.id} value={step.id}>
            {step.label}
          </option>
        ))}
      </Select>
      <FormActs pending={pending} refused={refused} blank={label.trim() === ""} onCancel={onDone} />
    </form>
  );
}

function PieceForm({ sessionId, kind, act, onDone }: { sessionId: string; kind: "lifecycle" | "rule"; act: JobCopyActs["piece"]; onDone: () => void }) {
  const [text, setText] = useState("");
  const { pending, refused, add } = useAdding(() => act(sessionId, kind, "", text), onDone);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    void add();
  };
  const name = kind === "rule" ? "Rule file" : "Lifecycle file";
  return (
    <form className="sv-lanes__form" data-file="" aria-label={`Add a ${kind} to this Job`} onSubmit={submit} onKeyDown={keysOf(onDone, () => void add())}>
      <Textarea label={name} rows={8} spellCheck={false} value={text} onChange={(event) => setText(event.target.value)} autoFocus />
      <FormActs pending={pending} refused={refused} blank={text.trim() === ""} onCancel={onDone} />
    </form>
  );
}

function FormActs({ pending, refused, blank, onCancel }: { pending: boolean; refused: string | undefined; blank: boolean; onCancel: () => void }) {
  return (
    <>
      {refused === undefined ? null : (
        <p className="sv-lanes__refused" role="alert">
          {refused}
        </p>
      )}
      <div className="sv-lanes__form-acts">
        <Button type="submit" variant="primary" size="sm" pending={pending} disabled={blank}>
          Add
        </Button>
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </>
  );
}

/**
 * What was changed on this Job, a row for each, under the canvas. **Keep for every Job is in the head, once, and only where a step of the workflow changed**: it proposes the
 * workflow the Job holds, whole. A rule or a lifecycle is kept for this Job alone until the Workflows page can save one.
 */
export function JobChanges({ sessionId, lanes, acts }: { sessionId: string; lanes: SessionLanes; acts: JobCopyActs | undefined }) {
  const { changes } = lanes.job;
  const workflow = changes.filter((one) => one.lane === "workflow");
  const signature = workflow.map((one) => one.id).join("\n");
  const [kept, setKept] = useState<string | undefined>();
  const [pending, setPending] = useState(false);
  const [refused, setRefused] = useState<string | undefined>();
  if (changes.length === 0) return null;
  const keep = async () => {
    if (acts === undefined || pending) return;
    setPending(true);
    setRefused(undefined);
    const said = await acts.keep(sessionId);
    setPending(false);
    if (said === undefined) setKept(signature);
    else setRefused(said);
  };
  return (
    <section className="sv-lanes__triage" aria-label="Changed on this Job">
      <h4 className="sv-lanes__triage-head" data-acts={acts !== undefined && workflow.length > 0 ? "" : undefined}>
        Changed on this Job
        {acts === undefined || workflow.length === 0 ? null : kept === signature ? (
          <Tooltip label={KEPT_SAID} asChild>
            <span className="sv-lanes__kept" role="img" aria-label={KEPT_SAID}>
              <Check size={14} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : (
          <Tooltip label={KEEP_SAID} asChild>
            <Button variant="ghost" size="sm" iconOnly aria-label={KEEP_SAID} pending={pending} onClick={() => void keep()}>
              <BookMarked size={16} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </h4>
      {refused === undefined ? null : (
        <p className="sv-lanes__refused" role="alert">
          {refused}
        </p>
      )}
      <ul className="sv-lanes__comments">
        {changes.map((one) => (
          <li key={one.id} className="sv-lanes__item">
            <div className="sv-lanes__change">
              <GitCompare className="sv-lanes__change-mark" size={14} strokeWidth={2} aria-hidden />
              <span className="sv-lanes__author">{changedName(lanes, one)}</span>
              <span className="sv-lanes__text">{one.text}</span>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
