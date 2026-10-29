// The rows the story's zones are made of (`StoryColumn.tsx`): a made thing as the ledger draws it, a finished group with what it produced beneath,
// a step of the workflow, and a beat the Job is on. A state is a glyph or a lamp with a tooltip and no row prints a state word.

import { useState, type KeyboardEvent, type ReactNode } from "react";
import { Circle, Clock, Footprints, Megaphone, Milestone, Tags, type LucideIcon } from "lucide-react";
import { Button, LinkedSession, Readout, Stamp, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessions } from "../sessions-draft";
import { Mark } from "./DeliveryRoute";
import { TaskTrack } from "./FlightPlan";
import { IssueRow, issueOfGroup } from "./IssuesBlock";
import { childrenOf, linkedPropsOf } from "./linked";
import type { Made } from "./ledger";
import { PullRow } from "./LedgerRow";
import { Report } from "./LandingPanel";
import { ModsMade } from "./ModsPanel";
import { GroupIssue, StepMark, TaskLine } from "./PlanGroup";
import { stackedRows } from "./pull-stacks";
import type { RouteStep } from "./route";
import type { ViewGroup } from "./plan-marks";
import { MARK } from "./StatusPanel";
import { LEDGER_KEY } from "./story";
import type { LedgerView } from "./ledger-view";

/**
 * Arrows, Home and End move between the rows of a zone, which are the buttons marked `data-nav` and a pull request's main button. A list with arrows of its own, a tab strip
 * and a field keep theirs.
 */
export function arrowsIn(event: KeyboardEvent<HTMLElement>): void {
  const keys: Record<string, (at: number, last: number) => number> = { ArrowDown: (at, last) => Math.min(at + 1, last), ArrowUp: (at) => Math.max(at - 1, 0), Home: () => 0, End: (_, last) => last };
  const to = keys[event.key];
  if (to === undefined || event.defaultPrevented || event.altKey || event.metaKey || event.ctrlKey) return;
  if ((event.target as Element).closest('[data-own-arrows], [role="tablist"], input, textarea, select, [role="combobox"]') !== null) return;
  const rows = [...event.currentTarget.querySelectorAll<HTMLElement>("[data-nav], button.sv-ledger__main")].filter((one) => !(one as HTMLButtonElement).disabled);
  if (rows.length === 0) return;
  const at = rows.findIndex((one) => one === document.activeElement || one.contains(document.activeElement));
  rows[to(at < 0 && event.key === "ArrowDown" ? -1 : at, rows.length - 1)]?.focus();
  event.preventDefault();
}

/** A glyph that names itself on hover, in the row's lead column. */
function Lead({ Glyph, said, pulse = false }: { Glyph: LucideIcon; said: string; pulse?: boolean }) {
  return (
    <Tooltip asChild label={said}>
      <span className="armada-session-mark sv-beat__lead" role="img" aria-label={said} {...(pulse ? { "data-pulsing": "" } : {})}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A step of the workflow: done, the one the Job is at, or to come. What the step made, or still owes, stands under it. */
/** A workflow step's name alone does not say it is one: the name's tooltip does. */
const STEP_SAID = { done: "A workflow step, done", now: "The workflow step the Job is on", next: "A workflow step still to come" } as const;

export function StepBeat({ name, state, under }: { name: string; state: "done" | "now" | "next"; under?: ReactNode }) {
  const { Glyph, said } = MARK[state];
  return (
    <li className="sv-beat" data-state={state} data-workflow="">
      <Lead Glyph={Glyph} said={said} pulse={state === "now"} />
      <Tooltip asChild label={STEP_SAID[state]}>
        <span className="sv-beat__label">{name}</span>
      </Tooltip>
      {under === undefined ? null : <div className="sv-beat__made">{under}</div>}
    </li>
  );
}

/** A step of the delivery route that is running, failed or still to come. */
export function RouteBeat({ step }: { step: RouteStep }) {
  return (
    <li className="sv-beat" data-state={step.state} data-route="">
      <Mark state={step.state} />
      <span className="sv-beat__label">{step.system === true ? <Readout>{step.label}</Readout> : step.label}</span>
    </li>
  );
}

/** Something the person is owed: a lamp that pulses, and what it is. Where it is a rule's own ask, pressing it opens the rule in the side panel, as the rule on the Workflow canvas does. */
export function AskBeat({ said, text, onOpen }: { said: string; text: string; onOpen?: () => void }) {
  return (
    <li className="sv-beat" data-lamp="needs">
      <Lead Glyph={Megaphone} said={said} pulse />
      {onOpen === undefined ? (
        <span className="sv-beat__label" data-wraps="">{text}</span>
      ) : (
        <button type="button" className="sv-beat__label sv-beat__open" data-nav="" data-wraps="" onClick={onOpen}>
          {text}
        </button>
      )}
    </li>
  );
}

/** A step held on a state an instance has yet to reach, such as a pull request being approved. It is a fact and no call: the agent rests and nothing is owed, so the clock does not pulse. */
export function WaitBeat({ text }: { text: string }) {
  return (
    <li className="sv-beat" data-wait="">
      <Lead Glyph={Clock} said="Waiting" />
      <span className="sv-beat__label" data-wraps="">{text}</span>
    </li>
  );
}

/** The agent or a Check at work: its mark and its name, and what it is doing under it where it says. */
export function WorkBeat({ mark, name, line, system = false, working = false }: { mark: ReactNode; name: string; line?: string | undefined; /** The name is one the system reports, a Check's: drawn as a reading. */ system?: boolean; /** The name is what Fleet sees the Job doing, and reads as the head of Now. */ working?: boolean }) {
  return (
    <li className="sv-beat" data-state={working ? "working" : undefined}>
      {mark}
      <span className="sv-beat__label">{system ? <Readout>{name}</Readout> : name}</span>
      {line === undefined ? null : <span className="sv-beat__line">{line}</span>}
    </li>
  );
}

/** A tick for each take of a walk, hollow while none is looked at. The takes are the ticks and are never counted in words. */
function Takes({ n }: { n: number }) {
  return (
    <Tooltip asChild label="Takes">
      <span className="sv-ticks" role="img" aria-label="Takes">
        {Array.from({ length: n }, (_, i) => (
          <span key={i} className="sv-tick" data-state="open">
            <Circle size={12} strokeWidth={2} aria-hidden />
          </span>
        ))}
      </span>
    </Tooltip>
  );
}

/** A walk shown to him: the lamp pulses, its name opens it as the window opens a walk, and a tick stands for each take. */
export function WalkBeat({ title, takes, onOpen }: { title: string; takes: number | undefined; onOpen: () => void }) {
  return (
    <li className="sv-beat" data-lamp="needs" data-walk="">
      <Lead Glyph={Footprints} said="Walk waiting on you" pulse />
      <button type="button" className="sv-beat__label sv-beat__open" data-nav="" onClick={onOpen}>
        {title}
      </button>
      {takes === undefined ? null : <Takes n={takes} />}
    </li>
  );
}

/** The walk a step will show, before it is shown. */
export function WalkToCome() {
  return (
    <div className="sv-made" data-pending="">
      <Tooltip asChild label="Walk, not shown yet">
        <span className="sv-made__mark" role="img" aria-label="Walk, not shown yet">
          <Footprints size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-made__title">Walk</span>
      <span />
    </div>
  );
}

const VIEWS: readonly { id: LedgerView; label: string; Glyph: LucideIcon }[] = [
  { id: "step", label: "By step", Glyph: Milestone },
  { id: "kind", label: "By kind", Glyph: Tags },
];

/** How Completed is read, as two icon toggles of one radio group. ← → and ↑ ↓ move between them and choose; the key (`story.ts`) flips them. */
export function LedgerToggle({ view, onChange }: { view: LedgerView; onChange: (next: LedgerView) => void }) {
  const move = (event: KeyboardEvent<HTMLElement>) => {
    const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    const group = event.currentTarget;
    const next = VIEWS[(VIEWS.findIndex((one) => one.id === view) + step + VIEWS.length) % VIEWS.length]!;
    onChange(next.id);
    window.setTimeout(() => group.querySelector<HTMLElement>(`[data-view="${next.id}"]`)?.focus(), 0);
  };
  return (
    <div className="sv-ledger-view" role="radiogroup" aria-label="View" data-own-arrows="" onKeyDown={move}>
      {VIEWS.map(({ id, label, Glyph }) => (
        <Tooltip key={id} label={label} shortcut={LEDGER_KEY.cap}>
          <Button variant="ghost" size="sm" iconOnly role="radio" aria-checked={view === id} aria-label={label} data-view={id} tabIndex={view === id ? 0 : -1} onClick={() => onChange(id)}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      ))}
    </div>
  );
}

/** A made thing that is neither a pull request nor an issue: its glyph, its words, and a button where it opens. A commit says how long ago. */
function MadeLine({ made }: { made: Extract<Made, { shape: "row" }> }) {
  const { row } = made;
  return (
    <button type="button" className="sv-made" data-nav="" data-kind={made.bucket} aria-label={row.title} disabled={row.open === undefined} onClick={row.open}>
      <Tooltip asChild label={row.said}>
        <span className="sv-made__mark" role="img" aria-label={row.said}>
          <row.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-made__title">{row.title}</span>
      {row.key.startsWith("commit:") && row.sub !== undefined ? <Stamp at={row.sub} first="age" still className="sv-made__age" /> : <span />}
    </button>
  );
}

/** Pull requests as the ledger draws them, a stack read parent first. */
function PullStack({ session, list }: { session: Session; list: readonly Extract<Made, { shape: "pr" }>[] }) {
  const prs = [...list].map((one) => one.pr).sort((a, b) => a.number - b.number);
  return (
    <ul className="sv-landing__prs" aria-label="Pull requests">
      {stackedRows(prs).map((row) => (
        <PullRow key={row.pr.number} session={session} pr={row.pr} {...(row.lane === "none" ? {} : { lane: row.lane })} {...(row.rootBase === undefined ? {} : { rootBase: row.rootBase })} {...(row.chip === undefined ? {} : { chip: row.chip })} />
      ))}
    </ul>
  );
}

/** How many of the most recent things made a list shows before he asks for the rest. */
const RECENT = 5;

/**
 * What was made, in the order it is handed in. Every pull request stands in one stack, where the first of them stands. A list given `newest` keeps the most
 * recent `RECENT` and a press shows the rest: Show earlier above a list that runs oldest first, Show more under one that runs newest first.
 */
export function MadeList({ session, list, label, newest }: { session: Session; list: readonly Made[]; label?: string; newest?: "first" | "last" }) {
  const [all, setAll] = useState(false);
  if (list.length === 0) return null;
  const shown = newest === undefined || all || list.length <= RECENT ? list : newest === "last" ? list.slice(-RECENT) : list.slice(0, RECENT);
  const more =
    shown.length === list.length ? null : (
      <li className="sv-made-list__more">
        <Button variant="ghost" size="sm" onClick={() => setAll(true)}>
          {newest === "last" ? "Show earlier" : "Show more"}
        </Button>
      </li>
    );
  const prs = shown.filter((one): one is Extract<Made, { shape: "pr" }> => one.shape === "pr");
  const first = prs[0];
  return (
    <ul className="sv-rows sv-made-list" {...(label === undefined ? {} : { "aria-label": label })}>
      {newest === "last" ? more : null}
      {shown.map((one) => {
        if (one.shape === "pr") return one === first ? <li key="pull-requests"><PullStack session={session} list={prs} /></li> : null;
        return (
          <li key={one.key}>
            {one.shape === "issue" ? (
              <IssueRow issue={one.issue} manifestId={session.manifestId} />
            ) : one.shape === "mods" ? (
              <ModsMade id={session.id} />
            ) : one.report === true ? (
              <Report session={session} row={one.row} />
            ) : (
              <MadeLine made={one} />
            )}
          </li>
        );
      })}
      {newest === "first" ? more : null}
    </ul>
  );
}

/** A finished group: its mark, its title and its tasks as one track, with what it produced beneath it (the issue it worked, its pull requests, the Job it was handed to). */
export function DoneGroup({ group, open = false, session, onOpenSession }: { group: ViewGroup; /** Whole plan: its tasks listed under it. */ open?: boolean; session: Session; onOpenSession: (id: string) => void }) {
  const sessions = useSessions();
  // What is still open stands in the current step and the Open group (`StoryColumn.tsx`), never under a finished step.
  const found = issueOfGroup(session, group.issue);
  const named = found === undefined ? undefined : { issue: found.issue, prs: found.prs.filter((pr) => pr.state === "merged") };
  const child = childrenOf(sessions, session).find((one) => one.group === group.id)?.session;
  return (
    <li className="sv-beat sv-beat--group" data-state="done" data-group={group.id}>
      <StepMark step="done" />
      <span className="sv-beat__label">{group.title}</span>
      {open ? null : <TaskTrack tasks={group.tasks} />}
      {!open ? null : (
        <div className="sv-beat__made">
          <ul className="sv-group__tasks" aria-label="Tasks">
            {group.tasks.map((task) => (
              <TaskLine key={task.id} task={task} here={false} now={undefined} />
            ))}
          </ul>
        </div>
      )}
      {named === undefined && child === undefined ? null : (
        <div className="sv-beat__made">
          {named === undefined || named.issue.state === "open" ? null : <GroupIssue issue={named.issue} manifestId={session.manifestId} />}
          {named === undefined || named.prs.length === 0 ? null : (
            <ul className="sv-landing__prs" aria-label="Pull requests">
              {named.prs.map((pr) => (
                <PullRow key={pr.number} session={session} pr={pr} />
              ))}
            </ul>
          )}
          {child === undefined ? null : <LinkedSession {...linkedPropsOf(child, onOpenSession)} />}
        </div>
      )}
    </li>
  );
}
