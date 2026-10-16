// The Walk tool's body and its two sheets. The body replaces the reader's, as every tool's results do: **how the place was
// found, one line per source tried** (the pull request, what Armada remembers, then him), the setup a pulled-down change
// needed, and the walk itself. A web walk is numbered steps with a walk window to play them in; an API walk is a card per
// request that runs by key. The sheets ask where a change runs when nobody knows, and list the places Armada remembers.
// Mock only: the window is drawn here and nothing is sent anywhere.

import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { ArrowLeft, Check, CircleHelp, Eye, Footprints, Globe, GitBranch, GitPullRequest, Hourglass, LoaderCircle, Minus, NotebookText, Pencil, Play, RefreshCw, RotateCw, Send, SquareDashed, SquareTerminal, Trash2, X } from "lucide-react";
import { Badge, Button, Checkbox, Input, Kbd, PrMark, Radio, RadioGroup, Sheet, Tooltip, useKept } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { REVIEW_KEYS } from "./key-table";
import { SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, ToolDef } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { toolsOf } from "./tools";
import { areaOf, areaShown, placeSaid } from "./walk-model";
import type { ApiStep, Phase, Place, Remembered, Source, SourceState, WalkPlan, WalkState, WebStep } from "./walk-model";
import { useWalkState, useWalks } from "./walk-store";
import "./walk.css";

const SOURCES: Record<Source, { Glyph: LucideIcon; said: string }> = {
  pr: { Glyph: GitPullRequest, said: "From the pull request" },
  memory: { Glyph: NotebookText, said: "From what Armada remembers of this area" },
  ask: { Glyph: CircleHelp, said: "From you" },
};

const FOUND: Record<SourceState, { Glyph: LucideIcon; said: string; tone?: string }> = {
  looking: { Glyph: LoaderCircle, said: "Looking", tone: "running" },
  found: { Glyph: Check, said: "Found", tone: "success" },
  nothing: { Glyph: Minus, said: "Nothing found" },
  asking: { Glyph: CircleHelp, said: "Waiting on you" },
};

const PHASE: Record<Phase, { Glyph: LucideIcon; said: string; tone?: string }> = {
  pending: { Glyph: SquareDashed, said: "Waiting" },
  running: { Glyph: LoaderCircle, said: "Running", tone: "running" },
  done: { Glyph: Check, said: "Done", tone: "success" },
  failed: { Glyph: X, said: "Failed", tone: "failed" },
};

const SETUP_MARK: Record<string, LucideIcon> = { checkout: GitBranch, run: SquareTerminal, wait: Hourglass };

/** A mark that turns while it runs, as the checks mark does. */
function Turning({ phase, Glyph, said, tone }: { phase: string; Glyph: LucideIcon; said: string; tone: string | undefined }) {
  return (
    <span className="pr-review__mark" data-checks={phase === "running" || phase === "looking" ? "running" : undefined}>
      <PrMark Glyph={Glyph} said={said} tone={tone} />
    </span>
  );
}

export function WalkView({ pr, tool }: { pr: ReviewPr; tool: ToolDef }) {
  const store = useReviewStore()!;
  const walks = useWalks();
  const state = useWalkState().by[pr.key];
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const run = work.runs[tool.id];
  const mark = TOOL_MARK[tool.mark];
  const source = SOURCE_MARK[tool.source];
  const plan = walks.plan(pr.key);
  const stale = state !== undefined && state.head !== pr.head;
  const asking = state?.tried.at(-1)?.state === "asking";
  const writing = state?.place !== undefined && run?.state === "running";
  const ready = state?.place !== undefined && run?.state === "done";
  return (
    <div className="pr-results pr-walk" aria-label={`${tool.name} results`} role="region" data-tool={tool.mark} data-source={tool.source}>
      <header className="pr-results__head">
        <Tooltip label="Back to the overview" shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back to the overview" onClick={() => store.setView(pr.key, "explanation")}>
            <ArrowLeft size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <PrMark Glyph={mark.Glyph} said={tool.name} />
        <h3 className="pr-results__title">{tool.name}</h3>
        <PrMark Glyph={source.Glyph} said={source.said} />
        <span className="pr-results__acts">
          <Tooltip label="Walk it again, for this pull request">
            <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={run?.state === "running"} onClick={() => store.spend(tool.name, () => walks.begin(pr, tool))}>
              <RotateCw size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {state === undefined ? null : (
        <>
          {stale ? (
            <p className="pr-results__stale" role="note" aria-label="Ran on an older commit">
              <PrMark Glyph={RefreshCw} said="Out of date" />
              {`Ran on ${state.head.slice(0, 7)}. The pull request is at ${pr.head.slice(0, 7)}.`}
            </p>
          ) : null}
          <Found state={state} />
          <Setup state={state} />
          {asking ? (
            <article className="pr-change pr-walk__ask" tabIndex={0} data-stop aria-label="Question: where it runs">
              <Button variant="primary" size="sm" data-run="" onClick={() => walks.ask(pr.key)}>
                <CircleHelp size={16} strokeWidth={2} aria-hidden />
                Answer
                <Kbd aria-hidden="true">↵</Kbd>
              </Button>
            </article>
          ) : null}
          {writing ? (
            <p className="pr-results__running">
              <Turning phase="running" Glyph={LoaderCircle} said="Writing the walk" tone="running" />
            </p>
          ) : null}
          {ready && plan === undefined ? (
            <p className="pr-results__clean">
              <PrMark Glyph={Check} said="Nothing to walk" tone="success" />
            </p>
          ) : null}
          {ready && plan !== undefined && state.place !== undefined ? <Walk pr={pr} plan={plan} place={state.place} state={state} /> : null}
        </>
      )}
    </div>
  );
}

/** One line per source tried, in the order he chose: the pull request, what Armada remembers, then him. */
function Found({ state }: { state: WalkState }) {
  return (
    <ol className="pr-walk__facts" aria-label="Where it runs">
      {state.tried.map((one) => {
        const source = SOURCES[one.source];
        const result = FOUND[one.state];
        return (
          <li key={one.source} className="pr-walk__fact" aria-label={source.said} data-state={one.state}>
            <PrMark Glyph={source.Glyph} said={one.by === undefined ? source.said : `${source.said}: ${one.by}`} />
            <Turning phase={one.state} Glyph={result.Glyph} said={result.said} tone={result.tone} />
            <span className="pr-walk__said">{one.state === "asking" ? "Not in the pull request or in memory" : one.said}</span>
          </li>
        );
      })}
    </ol>
  );
}

/** What a pulled-down change needed before it could be walked. */
function Setup({ state }: { state: WalkState }) {
  if (state.setup.length === 0) return null;
  return (
    <ol className="pr-walk__facts" aria-label="Setup">
      {state.setup.map((one) => {
        const phase = PHASE[one.phase];
        const Glyph = SETUP_MARK[one.id] ?? SquareTerminal;
        return (
          <li key={one.id} className="pr-walk__fact" aria-label={`${one.said} ${one.code ?? ""}`.trim()} data-state={one.phase}>
            <PrMark Glyph={Glyph} said={one.id === "checkout" ? "Checkout" : one.id === "run" ? "Command" : "Address"} />
            <Turning phase={one.phase} Glyph={phase.Glyph} said={phase.said} tone={phase.tone} />
            <span className="pr-walk__said">
              {one.said}
              {one.code === undefined ? null : <code className="pr-walk__code">{one.code}</code>}
            </span>
          </li>
        );
      })}
    </ol>
  );
}

function Walk({ pr, plan, place, state }: { pr: ReviewPr; plan: WalkPlan; place: Place; state: WalkState }) {
  const base = plan.kind === "api" ? (place.api !== "" ? place.api : place.address) : place.address !== "" ? place.address : place.api;
  return (
    <section className="pr-walk__body" aria-label={plan.kind === "web" ? "Web walk" : "API walk"}>
      <p className="pr-walk__against">
        <PrMark Glyph={plan.kind === "web" ? Globe : Send} said={plan.kind === "web" ? "Web address" : "API base address"} />
        <code className="pr-walk__code">{base}</code>
      </p>
      {plan.kind === "web" ? <WebWalk pr={pr} steps={plan.steps} address={base} state={state} /> : <ApiWalk pr={pr} steps={plan.steps} state={state} />}
    </section>
  );
}

function WebWalk({ pr, steps, address, state }: { pr: ReviewPr; steps: readonly WebStep[]; address: string; state: WalkState }) {
  const walks = useWalks();
  return (
    <>
      <article className="pr-change pr-walk__card" tabIndex={0} data-stop aria-label="Walk window">
        <Button variant="primary" size="sm" data-run="" onClick={() => walks.walkIt(pr.key)}>
          <Footprints size={16} strokeWidth={2} aria-hidden />
          Walk it
          <Kbd aria-hidden="true">↵</Kbd>
        </Button>
        {state.window === undefined ? null : <WalkWindow pr={pr} steps={steps} address={address} at={state.window} />}
      </article>
      <ol className="pr-steps" aria-label="Steps">
        {steps.map((step, at) => (
          <li key={step.id} className="pr-steps__step" aria-label={`Walk step ${at + 1}`}>
            <span className="pr-steps__n mono">{at + 1}</span>
            <div className="pr-steps__body">
              <p className="pr-steps__do">{step.say}</p>
              <p className="pr-walk__act">
                <code className="pr-walk__verb">{step.verb}</code>
                <span className="pr-walk__target">{`${step.role} “${step.name}”`}</span>
                {step.with === undefined ? null : <code className="pr-walk__code">{step.with}</code>}
              </p>
            </div>
          </li>
        ))}
      </ol>
    </>
  );
}

/** The window a walk plays in, drawn where it opens. Next moves along the steps, and the last one closes it. */
function WalkWindow({ pr, steps, address, at }: { pr: ReviewPr; steps: readonly WebStep[]; address: string; at: number }) {
  const walks = useWalks();
  const step = steps[at];
  const last = at >= steps.length - 1;
  useEffect(() => {
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>(".pr-walk__window [data-next]")?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, []);
  if (step === undefined) return null;
  return (
    <div className="pr-walk__window" role="group" aria-label="Walk window frame">
      <header className="pr-walk__window-head">
        <Footprints size={16} strokeWidth={2} aria-hidden />
        <code className="pr-walk__code">{address}</code>
        <Tooltip label="Close the walk window">
          <Button variant="ghost" size="sm" iconOnly aria-label="Close the walk window" onClick={() => walks.closeWindow(pr.key)}>
            <X size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      </header>
      <div className="pr-walk__window-card" role="note" aria-label={`Say: ${step.say}`}>
        <p className="pr-steps__do">{step.say}</p>
        <p className="pr-walk__act">
          <code className="pr-walk__verb">{step.verb}</code>
          <span className="pr-walk__target">{`${step.role} “${step.name}”`}</span>
        </p>
      </div>
      <footer className="pr-walk__window-foot">
        <Button variant="ghost" size="sm" disabled={at === 0} onClick={() => walks.moveWindow(pr.key, -1)}>
          Back
        </Button>
        <Button variant="primary" size="sm" data-next="" onClick={() => (last ? walks.closeWindow(pr.key) : walks.moveWindow(pr.key, 1))}>
          {last ? "Done" : "Next"}
          <Kbd aria-hidden="true">↵</Kbd>
        </Button>
      </footer>
    </div>
  );
}

function ApiWalk({ pr, steps, state }: { pr: ReviewPr; steps: readonly ApiStep[]; state: WalkState }) {
  const walks = useWalks();
  return (
    <ol className="pr-walk__requests" aria-label="Requests">
      {steps.map((step, at) => {
        const sent = state.sent[step.id];
        const ok = step.response.status < 400;
        return (
          <li key={step.id}>
            <article className="pr-change pr-walk__request" tabIndex={0} data-stop aria-label={`Request ${at + 1}: ${step.method} ${step.path}`}>
              <header className="pr-change__head">
                <span className="pr-steps__n mono">{at + 1}</span>
                <code className="pr-walk__verb">{step.method}</code>
                <code className="pr-walk__path">{step.path}</code>
                <span className="pr-change__marks">
                  <Button variant="ghost" size="sm" data-run="" aria-label={`Run ${step.method} ${step.path}`} pending={sent === "running"} onClick={() => walks.send(pr.key, step.id)}>
                    <Play size={16} strokeWidth={2} aria-hidden />
                    {sent === "done" ? "Run again" : "Run"}
                    <Kbd aria-hidden="true">↵</Kbd>
                  </Button>
                </span>
              </header>
              <p className="pr-change__why">{step.say}</p>
              {step.body === undefined ? null : <pre className="pr-walk__json" aria-label="Request body">{step.body}</pre>}
              {sent === "done" ? (
                <div className="pr-walk__response" role="group" aria-label="Response">
                  <p className="pr-walk__status">
                    <Badge status={ok ? "completed-success" : "completed-failed"} icon={ok ? Check : X}>
                      {String(step.response.status)}
                    </Badge>
                    <Tooltip label="Time the response took">
                      <span className="pr-walk__ms mono">{`${step.response.ms} ms`}</span>
                    </Tooltip>
                  </p>
                  <pre className="pr-walk__json">{step.response.body}</pre>
                </div>
              ) : null}
              <p className="pr-steps__look" role="note" aria-label={`Look for: ${step.look}`}>
                <PrMark Glyph={Eye} said="Look for" />
                {step.look}
              </p>
            </article>
          </li>
        );
      })}
    </ol>
  );
}

// ---- The sheets ---------------------------------------------------------------------------------------------

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** Both sheets, and the key that opens the list of places. Mounted with the Flight control, which is there wherever the review is. */
export function WalkSheets() {
  const walks = useWalks();
  const { sheet } = useWalkState();
  const reviews = useReviews();
  const { prs } = reviews;
  const floor = useAtFloor();
  // ⇧W opens the places, from anywhere a box is not being typed in and nothing else is up.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.key !== REVIEW_KEYS.where || event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      event.preventDefault();
      walks.showPlaces();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [walks]);
  const pr = sheet?.kind === "ask" ? prs.find((one) => one.key === sheet.key) : undefined;
  const tool = pr === undefined ? undefined : toolsOf(reviews, pr.manifestId).find((one) => one.mark === "walk");
  return (
    <>
      <Sheet kind="walk-place" open={pr !== undefined && tool !== undefined} floating floor={floor} title="Where it runs" closeLabel="Close" closeBinding="Esc" onClose={() => walks.closeSheet()}>
        {pr === undefined || tool === undefined ? null : <AskPlace pr={pr} tool={tool} />}
      </Sheet>
      <Sheet kind="walk-places" open={sheet?.kind === "places"} floating floor={floor} title="Where changes run" closeLabel="Close" closeBinding="Esc" onClose={() => walks.closeSheet()}>
        <Places />
      </Sheet>
    </>
  );
}

/** Asks where this change runs, and offers to remember it for the area of code it touches. */
function AskPlace({ pr, tool }: { pr: ReviewPr; tool: ToolDef }) {
  const walks = useWalks();
  const id = `pr-review:walk-place:${pr.key}`;
  const [how, setHow, forgetHow] = useKept<Place["how"]>(`${id}:how`, "deployed");
  const [address, setAddress, forgetAddress] = useKept(`${id}:address`, "");
  const [command, setCommand, forgetCommand] = useKept(`${id}:command`, "");
  const [api, setApi, forgetApi] = useKept(`${id}:api`, "");
  const [area, setArea] = useState(areaOf(pr));
  const [remember, setRemember] = useState(true);
  const where = address.trim() !== "" || api.trim() !== "";
  const ready = where && (how === "deployed" || command.trim() !== "");
  useEffect(() => {
    const frame = window.requestAnimationFrame(() => document.querySelector<HTMLElement>(".pr-walk-ask input[type='radio']:checked")?.focus());
    return () => window.cancelAnimationFrame(frame);
  }, []);
  const save = () => {
    if (!ready) return;
    walks.answer(pr, tool, { how, address: address.trim(), command: how === "pull" ? command.trim() : "", api: api.trim() }, remember ? area : undefined);
    [forgetHow, forgetAddress, forgetCommand, forgetApi].forEach((forget) => forget());
  };
  return (
    <form
      className="pr-add pr-walk-ask"
      aria-label="Where it runs"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
      onKeyDown={(event) => {
        // Enter sends the form from a choice as it does from a box, and from the box without waiting on the browser to do it.
        if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
          event.preventDefault();
          save();
        }
      }}
    >
      <RadioGroup label="Where">
        <span className="pr-add__choice">
          <Radio name="how" checked={how === "deployed"} onChange={() => setHow("deployed")}>
            Deployed at
          </Radio>
        </span>
        <span className="pr-add__choice">
          <Radio name="how" checked={how === "pull"} onChange={() => setHow("pull")}>
            Pull it down and run
          </Radio>
        </span>
      </RadioGroup>
      {how === "pull" ? <Input mono label="Command" value={command} onChange={(event) => setCommand(event.target.value)} /> : null}
      <Input mono label="Address" value={address} onChange={(event) => setAddress(event.target.value)} />
      <Input mono label="API base address" value={api} onChange={(event) => setApi(event.target.value)} />
      <div className="pr-walk-ask__remember">
        <Checkbox checked={remember} onChange={(event) => setRemember(event.target.checked)}>
          Remember for
        </Checkbox>
        <Input mono aria-label="Area" value={areaShown(area)} disabled={!remember} onChange={(event) => setArea(event.target.value)} />
      </div>
      <div className="pr-add__bar">
        <Button variant="ghost" onClick={() => walks.closeSheet()}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={!ready}>
          Walk
        </Button>
      </div>
    </form>
  );
}

/** The places Armada remembers, by repository and area, each one editable or forgotten. */
function Places() {
  const { memory } = useWalkState();
  const [editing, setEditing] = useState<string | undefined>(undefined);
  if (memory.length === 0) return null;
  return (
    <ul
      className="pr-walk-places"
      aria-label="Remembered places"
      onKeyDown={(event) => {
        // Down and up, and j and k, move between the rows; the row's own keys are on the row.
        if (typing(event.target)) return;
        const rows = [...event.currentTarget.querySelectorAll<HTMLElement>("article[data-stop]")];
        const at = rows.findIndex((one) => one === event.target || one.contains(event.target as Node));
        const by = event.key === "ArrowDown" || event.key === "j" ? 1 : event.key === "ArrowUp" || event.key === "k" ? -1 : 0;
        if (by !== 0) {
          event.preventDefault();
          rows[Math.min(rows.length - 1, Math.max(0, at + by))]?.focus();
        }
      }}
    >
      {memory.map((one) => (
        <li key={one.id}>
          <Row one={one} editing={editing === one.id} onEdit={(on) => setEditing(on ? one.id : undefined)} />
        </li>
      ))}
    </ul>
  );
}

function Row({ one, editing, onEdit }: { one: Remembered; editing: boolean; onEdit: (on: boolean) => void }) {
  const walks = useWalks();
  const [place, setPlace] = useState(one.place);
  return (
    <article
      className="pr-change pr-walk-place"
      tabIndex={0}
      data-stop
      aria-label={`Remembered: ${areaShown(one.area)} in ${one.repository}`}
      onKeyDown={(event) => {
        if (typing(event.target) || event.target !== event.currentTarget) return;
        if (event.key === "e" || event.key === "Enter") {
          event.preventDefault();
          setPlace(one.place);
          onEdit(true);
        } else if (event.key === "Delete" || event.key === "Backspace") {
          event.preventDefault();
          walks.forgetPlace(one.id);
        }
      }}
    >
      <header className="pr-change__head">
        <code className="pr-walk__path">{areaShown(one.area)}</code>
        <span className="pr-change__path">{one.repository}</span>
        <span className="pr-change__marks">
          <Tooltip label="Edit" shortcut="e">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Edit ${areaShown(one.area)}`} onClick={() => (setPlace(one.place), onEdit(!editing))}>
              <Pencil size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          <Tooltip label="Forget" shortcut="Del">
            <Button variant="ghost" size="sm" iconOnly aria-label={`Forget ${areaShown(one.area)}`} onClick={() => walks.forgetPlace(one.id)}>
              <Trash2 size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {editing ? (
        <form
          className="pr-add"
          aria-label={`Edit ${areaShown(one.area)}`}
          onSubmit={(event) => {
            event.preventDefault();
            walks.editPlace(one.id, place);
            onEdit(false);
          }}
        >
          {place.how === "pull" ? <Input mono label="Command" value={place.command} onChange={(event) => setPlace({ ...place, command: event.target.value })} /> : null}
          <Input mono label="Address" value={place.address} onChange={(event) => setPlace({ ...place, address: event.target.value })} />
          <Input mono label="API base address" value={place.api} onChange={(event) => setPlace({ ...place, api: event.target.value })} />
          <div className="pr-add__bar">
            <Button variant="ghost" onClick={() => onEdit(false)}>
              Cancel
            </Button>
            <Button variant="primary" type="submit">
              Save
            </Button>
          </div>
        </form>
      ) : (
        <p className="pr-change__why">{placeSaid(one.place)}</p>
      )}
    </article>
  );
}
