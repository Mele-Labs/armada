// The layers over a Session's stage, opened from Flight control's keys: Artifacts, Subagents, Checks and Bays. Each covers the thread and
// composer, which stay mounted under it, so Back leaves them exactly as they were. Esc is Back. The first thing in a layer holds focus: the
// search field in Artifacts, the first row elsewhere; the arrows walk the rows. A layer is drawn from what the ledger and the thread already
// hold (`artifacts.ts`, `slot-trouble.ts`) and reads nothing of its own.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { ArrowDown, Check, CircleDot, CircleX, LoaderCircle, Megaphone, Power, ShieldCheck, ShieldEllipsis, ShieldX, Terminal, X, type LucideIcon } from "lucide-react";
import { Button, ClashPaths, Input, PullRequestChecksMark, PullRequestStateMark, Table, Tabs, TableBody, TableCell, TableHead, TableHeaderCell, TableRow, Tooltip, type ClashView, useOpenClash, type LedgerEntry, type SessionLedgerTone } from "@armada/components";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { setHover } from "../cockpit/deck/store";
import { bayViewOf, ledgerOf, slotKey } from "../cockpit/deck/bays";
import { Berth } from "../cockpit/deck/BayRows";
import { ArtifactTimeline, type TimelineTake } from "./ArtifactTimeline";
import { artifactsOf, matching, type ArtifactRow } from "./artifacts";
import { timelineOf } from "./artifact-timeline";
import { closeLayer, type LayerId } from "./layers";
import { openSlotKeyInPanel } from "../shared-panel/store";
import { openClashFile } from "./ClashFileSheet";
import type { HeldBay, SessionBays } from "./slot-trouble";

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
function Shell({ label, head, keys, owns, children }: { label: string; head?: ReactNode; /** A key the layer owns, heard before the arrows walk the rows. */ keys?: (event: KeyboardEvent<HTMLElement>) => void; /** The letters `keys` answers to, which the deck does not take while focus is in the layer. */ owns?: string; children: ReactNode }) {
  const root = useRef<HTMLElement>(null);
  // Esc is Back, wherever focus is, unless a sheet or a menu is open over the layer: that one's Esc is its own.
  useEffect(() => {
    const listen = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null || popupIsOpen()) return;
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
    keys?.(event);
    if (event.defaultPrevented || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
    const stops = [...(root.current?.querySelectorAll<HTMLElement>(`[data-layer-field], ${ROW}`) ?? [])];
    const at = stops.findIndex((one) => one === document.activeElement);
    const to = stops[Math.min(stops.length - 1, Math.max(0, at + (event.key === "ArrowDown" ? 1 : -1)))];
    if (to === undefined) return;
    event.preventDefault();
    to.focus();
  };
  return (
    <section ref={root} className="sv-layer" aria-label={label} data-keeps-escape="" {...(owns === undefined ? {} : { "data-owns-keys": owns })} onKeyDown={walk}>
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

/** The Commands group lists this many before "View all". */
const COMMANDS_SHOWN = 10;

function Group({ group }: { group: { id: string; label: string; rows: readonly ArtifactRow[] } }) {
  const [all, setAll] = useState(false);
  const capped = group.id === "command" && group.rows.length > COMMANDS_SHOWN;
  const rows = capped && !all ? group.rows.slice(0, COMMANDS_SHOWN) : group.rows;
  return (
    <section className="sv-layer__group" aria-label={group.label}>
      <h4 className="sv-layer__group-label">{group.label}</h4>
      <ul className="sv-layer__rows">
        {rows.map((row) => (
          <Row key={row.key} row={row} />
        ))}
      </ul>
      {capped ? (
        <Button variant="ghost" size="sm" aria-expanded={all} onClick={() => setAll(!all)}>
          {all ? "Show fewer" : `View all ${group.rows.length}`}
        </Button>
      ) : null}
    </section>
  );
}

function Groups({ groups }: { groups: readonly { id: string; label: string; rows: readonly ArtifactRow[] }[] }) {
  return (
    <>
      {groups.map((group) => (
        <Group key={group.id} group={group} />
      ))}
    </>
  );
}

const VIEWS = [
  { id: "list", label: "List" },
  { id: "timeline", label: "Timeline" },
];

/** Everything the Session made or used, as groups or along time; the field at the top narrows both as it is typed in, and T switches them. */
export function ArtifactsLayer({ session, entries, onOpenSession, onOpenSketch }: { session: Session; entries: readonly LedgerEntry[]; onOpenSession: (id: string) => void; onOpenSketch?: (sketch: string, at?: string) => void }) {
  const [words, setWords] = useState("");
  const [view, setView] = useState<"list" | "timeline">("list");
  const sessions = useSessions();
  const take: TimelineTake = useSessionsDraft()?.timelineTake ?? "spine";
  const all = useMemo(() => artifactsOf(session, entries, { sessions, open: onOpenSession }), [session, entries, sessions, onOpenSession]);
  const shown = useMemo(() => matching(all, words), [all, words]);
  const timeline = useMemo(() => timelineOf(session, shown, { sketch: (id, at) => onOpenSketch?.(id, at) }), [session, shown, onOpenSketch]);
  // T switches the view from anywhere in the layer but a field, where it is a letter.
  const keys = (event: KeyboardEvent<HTMLElement>) => {
    const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement;
    if (typing || event.altKey || event.ctrlKey || event.metaKey || event.key.toLowerCase() !== "t") return;
    event.preventDefault();
    setView((now) => (now === "list" ? "timeline" : "list"));
  };
  return (
    <Shell
      label="Artifacts"
      keys={keys}
      owns="t"
      head={
        <>
          <div className="sv-layer__search">
            <Input type="search" aria-label="Search artifacts" placeholder="Search" data-layer-field="" value={words} onChange={(event) => setWords(event.target.value)} autoComplete="off" spellCheck={false} />
          </div>
          <Tooltip label="Switch view" shortcut="T">
            <Tabs items={VIEWS} value={view} onChange={(id) => setView(id === "timeline" ? "timeline" : "list")} />
          </Tooltip>
        </>
      }
    >
      {view === "list" ? <Groups groups={shown} /> : <ArtifactTimeline timeline={timeline} take={take} />}
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
      <p className="sv-layer__note">The agent that ran each Check gets its result when the run ends.</p>
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

/** One bay the Session holds, drawn as the cockpit's Bays panel draws it, with how far it is behind its base. A press shows it there. */
function HeldBayRow({ bay }: { bay: HeldBay }) {
  const { slot } = bay;
  const name = `slot-${slot.slot}`;
  const view = bayViewOf(slot, slotKey(slot), ledgerOf(slot, []), undefined, Date.now());
  const behind =
    slot.behind === undefined || slot.behind === 0 ? null : (
      <Tooltip label={`${slot.behind} ${slot.behind === 1 ? "commit" : "commits"} behind ${slot.base}`}>
        <span className="deck-bay__mark sv-layer__behind" role="img" aria-label={`${slot.behind} behind ${slot.base}`} data-wrong={bay.wrong === "behind" || undefined}>
          <ArrowDown size={12} aria-hidden="true" />
          {slot.behind}
        </span>
      </Tooltip>
    );
  return (
    <section className="sv-layer__group sv-layer__slot" aria-label={name}>
      <Tooltip label="Show in Bays">
        <button type="button" className="deck-bay" data-bay={view.state} data-layer-row="" aria-label={`Show ${name} in Bays`} onClick={() => void openSlotKeyInPanel(slotKey(slot))}>
          <Berth view={view} foot={behind} />
        </button>
      </Tooltip>
    </section>
  );
}

/** The other holder of a clash, linked; hovered or focused, its row in Work is lit too. `main` is named and opens nothing. */
function ClashWith({ clash }: { clash: ClashView }) {
  const open = useOpenClash();
  const holder = clash.with;
  if (holder.kind === "main") return <>{holder.label}</>;
  const row = `${holder.kind}:${holder.id}`;
  return (
    <button
      type="button"
      className="sv-layer__clash-with"
      aria-label={`Open ${holder.label}`}
      onClick={() => (setHover(undefined), open(holder))}
      onMouseEnter={() => setHover(row)}
      onMouseLeave={() => setHover(undefined)}
      onFocus={() => setHover(row)}
      onBlur={() => setHover(undefined)}
    >
      {holder.label}
    </button>
  );
}

/** Every bay the Session holds, and the Session's write clashes: a row to each other holder, linked, and the files a long list folds. Opened
 * from the Bays key or from a call in the thread. */
export function BaysLayer({ session, bays }: { session: Session; bays: SessionBays | undefined }) {
  // A file opens its diff only where Fleet reads one.
  const read = useSessionsDraft()?.clashFile !== undefined;
  const clashes: ClashView[] = (bays?.clashes ?? []).map((clash) => ({ with: clash.one.kind === "session" && clash.one.id === session.id ? clash.other : clash.one, paths: clash.paths }));
  return (
    <Shell label="Bays">
      {clashes.length === 0 ? null : (
        <section className="sv-layer__group" aria-label="Write clashes">
          <h4 className="sv-layer__group-label">Write clashes</h4>
          <Table aria-label="Write clashes">
            <TableHead>
              <TableRow>
                <TableHeaderCell>Clashes with</TableHeaderCell>
                <TableHeaderCell>Files</TableHeaderCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {clashes.map((clash) => (
                <TableRow key={`${clash.with.kind}:${clash.with.id}`}>
                  <TableCell>
                    <ClashWith clash={clash} />
                  </TableCell>
                  <TableCell variant="mono">
                    <ClashPaths clash={clash} {...(read ? { onPath: (path: string) => openClashFile(session.id, clash.with, path) } : {})} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </section>
      )}
      {bays?.bays.map((bay) => <HeldBayRow key={bay.slot.slot} bay={bay} />)}
    </Shell>
  );
}

/** The layer that is open. */
export function Layer({ id, session, entries, bays, onOpenCheck, onOpenSession, onOpenSketch }: { id: LayerId; session: Session; entries: readonly LedgerEntry[]; bays: SessionBays | undefined; onOpenCheck: (run: number) => void; onOpenSession: (id: string) => void; onOpenSketch?: (sketch: string, at?: string) => void }) {
  if (id === "artifacts") return <ArtifactsLayer session={session} entries={entries} onOpenSession={onOpenSession} {...(onOpenSketch === undefined ? {} : { onOpenSketch })} />;
  if (id === "subagents") return <SubagentsLayer session={session} entries={entries} />;
  if (id === "checks") return <ChecksLayer session={session} onOpenCheck={onOpenCheck} />;
  return <BaysLayer session={session} bays={bays} />;
}
