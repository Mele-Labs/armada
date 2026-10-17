// The layers over a Session's stage, opened from Flight control's keys: Artifacts, Subagents, Checks and Slot. Each covers the thread and
// composer, which stay mounted under it, so Back leaves them exactly as they were. Esc is Back. The first thing in a layer holds focus: the
// search field in Artifacts, the first row elsewhere; the arrows walk the rows. A layer is drawn from what the ledger and the thread already
// hold (`artifacts.ts`, `slot-trouble.ts`) and reads nothing of its own.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { Check, CircleDot, CircleX, LoaderCircle, Megaphone, Power, ShieldCheck, ShieldEllipsis, ShieldX, Terminal, X, type LucideIcon } from "lucide-react";
import { Button, Input, PullRequestChecksMark, PullRequestStateMark, Tooltip, type LedgerEntry, type SessionLedgerTone } from "@armada/components";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import { openFrom } from "../cockpit/deck/store";
import { slotKey } from "../cockpit/deck/bays";
import { artifactsOf, matching, type ArtifactRow } from "./artifacts";
import { closeLayer, type LayerId } from "./layers";
import type { SlotTrouble } from "./slot-trouble";
import { CallCard, useCallAt } from "./CallViews";
import type { Call } from "./calls";

const TONE: Record<SessionLedgerTone, LucideIcon> = {
  pending: ShieldEllipsis,
  passed: ShieldCheck,
  failed: ShieldX,
  running: CircleDot,
  done: Check,
  escalated: Megaphone,
  piloted: Terminal,
  stopped: Power,
};

/** A state as one glyph in its hue, its tooltip naming it. */
function Mark({ tone, said }: { tone: SessionLedgerTone; said: string }) {
  const Glyph = TONE[tone];
  return (
    <Tooltip label={said}>
      <span className="sv-layer__mark" data-tone={tone} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

const ROW = "[data-layer-row]";

/** The frame every layer shares: its name, what is beside it in the head, Back, and a body that scrolls. */
function Shell({ label, head, children }: { label: string; head?: ReactNode; children: ReactNode }) {
  const root = useRef<HTMLElement>(null);
  // Esc is Back, wherever focus is, unless a sheet is open over the layer: that one's Esc is its own.
  useEffect(() => {
    const listen = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      event.preventDefault();
      closeLayer();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);
  // Focus goes in: to the field when there is one, else to the first row.
  useEffect(() => {
    const first = root.current?.querySelector<HTMLElement>(`[data-layer-field], ${ROW}`);
    first?.focus();
  }, [label]);
  const walk = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    const stops = [...(root.current?.querySelectorAll<HTMLElement>(`[data-layer-field], ${ROW}`) ?? [])];
    const at = stops.findIndex((one) => one === document.activeElement);
    const to = stops[Math.min(stops.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))];
    if (to === undefined) return;
    event.preventDefault();
    to.focus();
  };
  return (
    <section ref={root} className="sv-layer" aria-label={label} data-keeps-escape="" onKeyDown={walk}>
      <header className="sv-layer__head">
        <h3 className="sv-layer__title">{label}</h3>
        {head}
        <Tooltip label="Back" shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={closeLayer}>
            <X size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      </header>
      <div className="sv-layer__body">{children}</div>
    </section>
  );
}

/** One row: its glyph, its words, its state. A row with somewhere to go is a button; one without is read. */
function Row({ row }: { row: ArtifactRow }) {
  const { Glyph } = row;
  const inside = (
    <>
      <Tooltip label={row.said}>
        <span className="sv-layer__kind" role="img" aria-label={row.said}>
          <Glyph size={16} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-layer__title-line">{row.title}</span>
      {row.sub === undefined ? null : <code className="sv-layer__sub">{row.sub}</code>}
      <span className="sv-layer__state">
        {row.pr === undefined ? null : (
          <>
            <PullRequestStateMark merged={row.pr.merged} />
            <PullRequestChecksMark checks={row.pr.checks} said={row.pr.checksSaid} />
          </>
        )}
        {row.mark === undefined ? null : <Mark tone={row.mark.glyph} said={row.mark.said} />}
      </span>
    </>
  );
  return (
    <li className="sv-layer__item">
      {row.open === undefined ? (
        <div className="sv-layer__row" data-plain="">
          {inside}
        </div>
      ) : (
        <button type="button" className="sv-layer__row" data-layer-row="" aria-label={row.title} onClick={row.open}>
          {inside}
        </button>
      )}
    </li>
  );
}

function Groups({ groups }: { groups: readonly { id: string; label: string; rows: readonly ArtifactRow[] }[] }) {
  return (
    <>
      {groups.map((group) => (
        <section key={group.id} className="sv-layer__group" aria-label={group.label}>
          <h4 className="sv-layer__group-label">{group.label}</h4>
          <ul className="sv-layer__rows">
            {group.rows.map((row) => (
              <Row key={row.key} row={row} />
            ))}
          </ul>
        </section>
      ))}
    </>
  );
}

/** Everything the Session made or used, in groups; the field at the top narrows them as it is typed in. */
export function ArtifactsLayer({ session, entries }: { session: Session; entries: readonly LedgerEntry[] }) {
  const [words, setWords] = useState("");
  const all = useMemo(() => artifactsOf(session, entries), [session, entries]);
  const shown = useMemo(() => matching(all, words), [all, words]);
  return (
    <Shell
      label="Artifacts"
      head={
        <div className="sv-layer__search">
          <Input type="search" aria-label="Search artifacts" placeholder="Search" data-layer-field="" value={words} onChange={(event) => setWords(event.target.value)} autoComplete="off" spellCheck={false} />
        </div>
      }
    >
      <Groups groups={shown} />
    </Shell>
  );
}

/** Every subagent, the running ones first. Each opens its own thread, as the ledger's rows do. */
export function SubagentsLayer({ session, entries }: { session: Session; entries: readonly LedgerEntry[] }) {
  const groups = useMemo(() => artifactsOf(session, entries).filter((one) => one.id === "subagent"), [session, entries]);
  return (
    <Shell label="Subagents">
      <ul className="sv-layer__rows">
        {(groups[0]?.rows ?? []).map((row) => (
          <Row key={row.key} row={row} />
        ))}
      </ul>
    </Shell>
  );
}

const CHECK = { running: "running", passed: "passed", failed: "failed" } as const;
const CHECK_SAID = { running: "Running", passed: "Passed", failed: "Failed" } as const;

/** The Checks the Session ran in its slot, newest first. Each opens its run on the Checks page. */
export function ChecksLayer({ session, onOpenCheck }: { session: Session; onOpenCheck: (run: number) => void }) {
  const runs = session.rows.filter((one): one is Extract<SessionRow, { kind: "check" }> => one.kind === "check").reverse();
  return (
    <Shell label="Checks">
      <ul className="sv-layer__rows">
        {runs.map((one) => (
          <li key={one.id} className="sv-layer__item">
            <button type="button" className="sv-layer__row" data-layer-row="" aria-label={`${one.name}, ${CHECK_SAID[one.state].toLowerCase()}`} onClick={() => onOpenCheck(one.run)}>
              <span className="sv-layer__kind">
                {one.state === "running" ? <LoaderCircle size={16} strokeWidth={2} aria-hidden data-spinning="" /> : one.state === "failed" ? <CircleX size={16} strokeWidth={2} aria-hidden /> : <ShieldCheck size={16} strokeWidth={2} aria-hidden />}
              </span>
              <span className="sv-layer__title-line">{one.name}</span>
              <code className="sv-layer__sub">{one.at}</code>
              <span className="sv-layer__state">
                <Mark tone={CHECK[one.state]} said={CHECK_SAID[one.state]} />
              </span>
            </button>
          </li>
        ))}
      </ul>
    </Shell>
  );
}

const STATE = { stranded: "Stranded", behind: "Behind", conflicting: "Conflicting" } as const;

/** The slot and branch the Session works in, and why they are worth a look. Opened from the Slot key or from the call in the thread. */
export function SlotLayer({ session, trouble }: { session: Session; trouble: SlotTrouble | undefined }) {
  const slot = trouble?.slot;
  const stranded = slot?.stranded;
  const facts: [string, ReactNode][] = [];
  if (trouble !== undefined && slot !== undefined) {
    facts.push(["Slot", <code key="slot">{slot.slot}</code>]);
    if (trouble.branch !== undefined) facts.push(["Branch", <code key="branch">{trouble.branch}</code>]);
    facts.push(["State", STATE[trouble.kind]]);
    if (slot.behind !== undefined) facts.push([`Behind ${slot.base}`, `${slot.behind} ${slot.behind === 1 ? "commit" : "commits"}`]);
    if (stranded?.last_holder !== undefined) facts.push(["Held last by", stranded.last_holder.title ?? stranded.last_holder.id]);
    if (stranded !== undefined) {
      if (stranded.uncommitted.length > 0) facts.push(["Uncommitted", stranded.uncommitted.map((one) => <code key={one}>{one}</code>)]);
      facts.push(["Unpushed", `${stranded.unpushed} ${stranded.unpushed === 1 ? "commit" : "commits"}`]);
    }
    for (const clash of trouble.clashes) {
      const other = clash.one.kind === "session" && clash.one.id === session.id ? clash.other : clash.one;
      facts.push([`Clashes with ${other.label}`, clash.paths.map((one) => <code key={one}>{one}</code>)]);
    }
  }
  return (
    <Shell label="Slot">
      {trouble === undefined || slot === undefined ? null : (
        <div className="sv-layer__slot">
          <dl className="sv-layer__facts">
            {facts.map(([term, value]) => (
              <div key={term} className="sv-layer__fact">
                <dt>{term}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
          <Button variant="secondary" size="sm" data-layer-row="" onClick={() => openFrom({ kind: "slot", key: slotKey(slot) })}>
            Open the bay
          </Button>
        </div>
      )}
    </Shell>
  );
}

/** The layer that is open, if one is. */
/** One call, whole: the one the composer's line shows, with its detail and the hint on each answer. */
function CallLayer({ session, calls }: { session: Session; calls: readonly Call[] }) {
  const [at] = useCallAt(session.id, calls);
  const call = calls[at];
  // Answered from here, the call is gone and so is the layer.
  useEffect(() => {
    if (call === undefined) closeLayer();
  }, [call]);
  if (call === undefined) return null;
  return (
    <Shell label={call.word} head={<span className="sv-layer__spacer" />}>
      <CallCard call={call} size="full" keep={`call:${session.id}:${call.key}`} />
    </Shell>
  );
}

export function Layer({ id, session, entries, trouble, calls, onOpenCheck }: { id: LayerId; session: Session; entries: readonly LedgerEntry[]; trouble: SlotTrouble | undefined; calls: readonly Call[]; onOpenCheck: (run: number) => void }) {
  if (id === "call") return <CallLayer session={session} calls={calls} />;
  if (id === "artifacts") return <ArtifactsLayer session={session} entries={entries} />;
  if (id === "subagents") return <SubagentsLayer session={session} entries={entries} />;
  if (id === "checks") return <ChecksLayer session={session} onOpenCheck={onOpenCheck} />;
  return <SlotLayer session={session} trouble={trouble} />;
}
