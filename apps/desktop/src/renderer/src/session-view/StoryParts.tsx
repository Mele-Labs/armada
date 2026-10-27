// The rows the story's zones are made of (`StoryColumn.tsx`): a made thing as the ledger draws it, a finished group with what it produced beneath,
// a step of the workflow, and a beat the Job is on. A state is a glyph or a lamp with a tooltip and no row prints a state word.

import type { KeyboardEvent, ReactNode } from "react";
import { Megaphone, type LucideIcon } from "lucide-react";
import { LinkedSession, Stamp, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessions } from "../sessions-draft";
import { Mark } from "./DeliveryRoute";
import { Tick } from "./FlightPlan";
import { IssueRow, issueOfGroup } from "./IssuesBlock";
import { childrenOf, linkedPropsOf } from "./linked";
import type { Made } from "./ledger";
import { PullRow } from "./LedgerRow";
import { Report } from "./LandingPanel";
import { ModsMade } from "./ModsPanel";
import { GroupIssue, StepMark } from "./PlanGroup";
import { stackedRows } from "./pull-stacks";
import type { RouteStep } from "./route";
import type { ViewGroup } from "./plan-marks";
import { MARK } from "./StatusPanel";

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

/** A step of the workflow: done, the one the Job is at, or to come. Where the step is the plan's, a press opens what the plan drew. */
export function StepBeat({ name, state, open }: { name: string; state: "done" | "now" | "next"; open?: (() => void) | undefined }) {
  const { Glyph, said } = MARK[state];
  return (
    <li className="sv-beat" data-state={state} data-workflow="">
      <Lead Glyph={Glyph} said={said} pulse={state === "now"} />
      {open === undefined ? (
        <span className="sv-beat__label">{name}</span>
      ) : (
        <button type="button" className="sv-beat__label sv-beat__open" data-nav="" aria-label={name} onClick={open}>
          {name}
        </button>
      )}
    </li>
  );
}

/** A step of the delivery route that is running, failed or still to come. */
export function RouteBeat({ step }: { step: RouteStep }) {
  return (
    <li className="sv-beat" data-state={step.state} data-route="">
      <Mark state={step.state} />
      <span className="sv-beat__label">{step.label}</span>
    </li>
  );
}

/** Something the person is owed: a lamp that pulses, and what it is. */
export function AskBeat({ said, text }: { said: string; text: string }) {
  return (
    <li className="sv-beat" data-lamp="needs">
      <Lead Glyph={Megaphone} said={said} pulse />
      <span className="sv-beat__label" data-wraps="">{text}</span>
    </li>
  );
}

/** The agent or a Check at work: its mark and its name, and what it is doing under it where it says. */
export function WorkBeat({ mark, name, line }: { mark: ReactNode; name: string; line?: string | undefined }) {
  return (
    <li className="sv-beat">
      {mark}
      <span className="sv-beat__label">{name}</span>
      {line === undefined ? null : <span className="sv-beat__line">{line}</span>}
    </li>
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

/** What was made, in the order it is handed in. Every pull request stands in one stack, where the first of them stands. */
export function MadeList({ session, list, label }: { session: Session; list: readonly Made[]; label?: string }) {
  const prs = list.filter((one): one is Extract<Made, { shape: "pr" }> => one.shape === "pr");
  if (list.length === 0) return null;
  const first = prs[0];
  return (
    <ul className="sv-rows sv-made-list" {...(label === undefined ? {} : { "aria-label": label })}>
      {list.map((one) => {
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
    </ul>
  );
}

/** A finished group: its mark, its title and a tick for each task, with what it produced beneath it (the issue it worked, its pull requests, the Job it was handed to). */
export function DoneGroup({ group, session, onOpenSession }: { group: ViewGroup; session: Session; onOpenSession: (id: string) => void }) {
  const sessions = useSessions();
  const named = issueOfGroup(session, group.issue);
  const child = childrenOf(sessions, session).find((one) => one.group === group.id)?.session;
  return (
    <li className="sv-beat sv-beat--group" data-state="done" data-group={group.id}>
      <StepMark step="done" />
      <span className="sv-beat__label">{group.title}</span>
      <span className="sv-ticks">
        {group.tasks.map((task) => (
          <Tick key={task.id} task={task} />
        ))}
      </span>
      {named === undefined && child === undefined ? null : (
        <div className="sv-beat__made">
          {named === undefined ? null : <GroupIssue issue={named.issue} manifestId={session.manifestId} />}
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
