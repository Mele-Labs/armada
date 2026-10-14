// What the three Cockpit body prototypes share: the mark a state wears, the pull request chip, the acts
// every Job and Session carries in place (Reply, Peek, Stop, Open, and Merge on a ready pull request),
// the keys, and the key sheet. Nothing here is registered in the real keymap: the keys are handlers on
// this mock's window. Mock only. proto/cockpit-three-directions.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { createPortal } from "react-dom";
import { Box, Check, CircleDot, ExternalLink, GitCommitHorizontal, GitMerge, GitPullRequest, Merge, Pause, Plane, Reply, Rows3, ShieldCheck, ShieldX, Square, SquareTerminal, Terminal, X, type LucideIcon } from "lucide-react";
import { Button, Kbd, SessionThread, StepBar, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import { useSessions, useSessionsDraft } from "../../sessions-draft";
import { SessionBox } from "../../sessions";
import { threadRowsOf } from "../../session-thread-rows";
import type { CockpitProps } from "../../cockpit/body-seam";
import { patchLocal, type Main, type Mark, type Pr, type Ship, type Station, type StationId, type Subject, type Vote } from "./ship";
import "./proto.css";

/** A Job's place in its workflow, in the step bar its row already wears. */
export function StepsBar({ step, tone }: { step: NonNullable<Subject["step"]>; tone: Mark["tone"] }) {
  return <StepBar total={step.of} current={step.at} activity={tone === "bad" ? "failed" : tone === "ask" ? "awaiting_human" : "running"} label={`Step ${step.at} of ${step.of}: ${step.label}`} />;
}

/** One state as its registry glyph, named by a tooltip and never by a word. */
export function MarkGlyph({ mark, size = 16 }: { mark: Mark; size?: 12 | 16 | 20 }) {
  const { Glyph } = mark;
  return (
    <Tooltip label={mark.said}>
      <span className="proto-mark" data-tone={mark.tone} role="img" aria-label={mark.said}>
        <Glyph size={size} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

const STAGE: Record<Pr["stage"], { Glyph: LucideIcon; said: string }> = {
  checking: { Glyph: CircleDot, said: "Checks running" },
  red: { Glyph: ShieldX, said: "Checks failed" },
  queued: { Glyph: GitMerge, said: "In the merge queue" },
  ready: { Glyph: ShieldCheck, said: "Checks passed" },
  landed: { Glyph: Check, said: "Merged" },
};
export const stageOf = (pr: Pr) => STAGE[pr.stage];

/** A pull request as a chip on the strip or unit that owns it: its number and the stage it is at. */
export function PrChip({ pr, onOpen }: { pr: Pr; onOpen?: () => void }) {
  const { Glyph, said } = STAGE[pr.stage];
  const body = (
    <>
      <GitPullRequest size={12} aria-hidden />
      <span className="proto-pr__number">#{pr.number}</span>
      <Glyph size={12} aria-hidden />
    </>
  );
  return (
    <Tooltip label={`#${pr.number} ${said}`}>
      {onOpen === undefined ? (
        <span className="proto-pr" data-stage={pr.stage} role="img" aria-label={`#${pr.number}, ${said}`}>
          {body}
        </span>
      ) : (
        <button type="button" className="proto-pr" data-stage={pr.stage} aria-label={`#${pr.number}, ${said}`} onClick={onOpen}>
          {body}
        </button>
      )}
    </Tooltip>
  );
}

/** What the owner can do here that Fleet would answer. A Session's words go where its page's box sends them. */
export type Acts = {
  reply: (subject: Subject, text: string) => void;
  stop: (subject: Subject) => void;
  merge: (subject: Subject) => void;
  fix: (main: Main) => void;
  open: (subject: Subject) => void;
};

export function useShipActs(props: CockpitProps): Acts {
  const draft = useSessionsDraft();
  const { onKill, onFix, onOpen, onOpenSession, onOpenLink } = props;
  return useMemo(
    () => ({
      reply: (subject, text) => {
        void draft?.send(subject.id, { text, files: [], sketches: [], tags: [] });
      },
      stop: (subject) => {
        if (subject.kind === "session") {
          if (draft?.stop !== undefined) draft.stop(subject.id);
          else patchLocal((was) => ({ stopped: new Set(was.stopped).add(subject.key) }));
        } else if (subject.kind === "job") onKill?.(subject.id);
      },
      merge: (subject) => {
        const pr = subject.pr;
        if (pr === undefined) return;
        if (subject.kind === "session") void draft?.act(subject.id, pr.number, "merge");
        patchLocal((was) => ({ landed: new Set(was.landed).add(pr.number) }));
      },
      fix: (main) => {
        onFix?.({ root: main.root });
        patchLocal((was) => ({ fixing: new Set(was.fixing).add(main.root) }));
      },
      open: (subject) => {
        if (subject.kind === "job") onOpen(subject.id);
        else if (subject.kind === "session") onOpenSession(subject.id);
        else if (subject.pr !== undefined) onOpenLink(subject.pr.url);
      },
    }),
    [draft, onKill, onFix, onOpen, onOpenSession, onOpenLink],
  );
}

/** Which subject is selected, whose Reply box is open, whose Peek is, and whether the key sheet is. */
export type ShipUi = {
  selected: string | undefined;
  select: (key: string | undefined) => void;
  replying: string | undefined;
  setReplying: (key: string | undefined) => void;
  peeking: string | undefined;
  setPeeking: (key: string | undefined) => void;
  sheet: boolean;
  setSheet: (on: boolean) => void;
  /** The station held on the large tile by hand, over what the alarms would put there. */
  pin: StationId | undefined;
  setPin: (id: StationId | undefined) => void;
  /** A call's card is open, wherever the window is. */
  cardOpen: boolean;
};

/**
 * The window's view state lives here and not in the body that draws it, so a body that unmounts while the owner
 * is on another page comes back to the same selection, the same open Peek and Reply, and the same station.
 */
type UiState = { selected?: string; replying?: string; peeking?: string; sheet: boolean; pin?: StationId; cardOpen: boolean; scroll: Readonly<Record<string, number>> };
const FRESH: UiState = { sheet: false, cardOpen: false, scroll: {} };
let uiState: UiState = FRESH;
const uiListeners = new Set<() => void>();
const uiSubscribe = (on: () => void) => (uiListeners.add(on), () => void uiListeners.delete(on));
const setUi = (change: Partial<UiState>) => {
  uiState = { ...uiState, ...change };
  uiListeners.forEach((one) => one());
};
export const forgetUi = () => {
  uiState = FRESH;
  uiListeners.forEach((one) => one());
};
export const setCardOpen = (open: boolean) => {
  if (uiState.cardOpen !== open) setUi({ cardOpen: open });
};
/** Where a pane was scrolled to, kept for the body that comes back. Held without a render. */
export const scrolled = {
  get: (key: string) => uiState.scroll[key] ?? 0,
  set: (key: string, top: number) => {
    uiState = { ...uiState, scroll: { ...uiState.scroll, [key]: top } };
  },
};

export function useShipUi(): ShipUi {
  const now = useSyncExternalStore(uiSubscribe, () => uiState);
  return useMemo(
    () => ({
      selected: now.selected,
      select: (key) => setUi({ selected: key }),
      replying: now.replying,
      setReplying: (key) => setUi({ replying: key }),
      peeking: now.peeking,
      setPeeking: (key) => setUi({ peeking: key }),
      sheet: now.sheet,
      setSheet: (on) => setUi({ sheet: on }),
      pin: now.pin,
      setPin: (id) => setUi({ pin: id }),
      cardOpen: now.cardOpen,
    }),
    [now],
  );
}

/** `g` has been pressed and the next key names a place: the keys of a body stand down, and the one that took it is named. */
export const chord: { armed: boolean; took: KeyboardEvent | undefined } = { armed: false, took: undefined };

/** The mark a key stands for on the strip, in the tooltip and on the sheet. */
const KEYS = {
  reply: "r",
  peek: "p",
  stop: "s",
  open: "↵",
  merge: "m",
  fix: "f",
  call: "w",
} as const;
export { KEYS };

export function IconAct({ label, keys, glyph: Glyph, pressed, onPress }: { label: string; keys: string; glyph: LucideIcon; pressed?: boolean; onPress: () => void }) {
  return (
    <Tooltip label={`${label} · ${keys}`}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} {...(pressed === undefined ? {} : { "aria-pressed": pressed })} onClick={onPress}>
        <Glyph size={16} aria-hidden />
      </Button>
    </Tooltip>
  );
}

/** Every act a subject carries, in place. A key does what its button does. */
export function ActBar({ subject, acts, ui, labeled = false }: { subject: Subject; acts: Acts; ui: ShipUi; labeled?: boolean }) {
  const reply = ui.replying === subject.key;
  const peek = ui.peeking === subject.key;
  return (
    <span className="proto-acts" data-labeled={labeled || undefined}>
      {subject.repliable ? <IconAct label="Reply" keys={KEYS.reply} glyph={Reply} pressed={reply} onPress={() => ui.setReplying(reply ? undefined : subject.key)} /> : null}
      <IconAct label="Peek" keys={KEYS.peek} glyph={Terminal} pressed={peek} onPress={() => ui.setPeeking(peek ? undefined : subject.key)} />
      {subject.stoppable ? <IconAct label="Stop" keys={KEYS.stop} glyph={Square} onPress={() => acts.stop(subject)} /> : null}
      <IconAct label="Open" keys={KEYS.open} glyph={ExternalLink} onPress={() => acts.open(subject)} />
      {subject.mergeable ? (
        <Button variant="secondary" size="sm" onClick={() => acts.merge(subject)}>
          Merge
          <Kbd>{KEYS.merge}</Kbd>
        </Button>
      ) : null}
    </span>
  );
}

/** A Session's box on its strip or unit: Enter sends, Shift Enter breaks the line, Escape closes and keeps the draft. */
export function ReplyBox({ subject, acts, ui }: { subject: Subject; acts: Acts; ui: ShipUi }) {
  const [text, setText] = useState("");
  const keep = `cockpit-proto:reply:${subject.key}`;
  const send = () => {
    const said = text.trim();
    if (said === "") return;
    acts.reply(subject, said);
    forgetKept(keep);
    ui.setReplying(undefined);
  };
  return (
    <div className="proto-reply">
      <Textarea
        autoFocus
        rows={2}
        keep={keep}
        aria-label={`Reply to ${subject.title}`}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            ui.setReplying(undefined);
          } else if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={send}>
        Send
      </Button>
    </div>
  );
}

/** How many of a thread's last rows a Peek draws. */
const PEEK_ROWS = 6;
/** Room a Peek wants below its anchor to open downward. */
const PEEK_ROOM = 440;

/**
 * Peek: the end of a Session's own thread, drawn by the thread's own rows, with the live working mark while
 * its turn runs, rows landing at the end as they arrive, and the Session's own box at the foot. A Job has no
 * thread, so its last lines are drawn as the thread's tool rows. Nothing here is a row style of its own.
 */
export function PeekPop({ subject, ui, acts }: { subject: Subject; ui: ShipUi; acts: Acts }) {
  const [at, setAt] = useState<{ top?: number; bottom?: number; left: number } | undefined>();
  const panel = useRef<HTMLDivElement>(null);
  const draft = useSessionsDraft();
  const session = useSessions().find((one) => one.id === subject.id && subject.kind === "session");
  useEffect(() => draft?.watch?.(subject.id), [draft, subject.id]);
  useLayoutEffect(() => {
    const anchor = document.querySelector<HTMLElement>(`[data-subject="${subject.key}"]`);
    if (anchor === null) return;
    const box = anchor.getBoundingClientRect();
    const width = 440;
    const left = Math.min(Math.max(8, box.right - width), window.innerWidth - width - 8);
    setAt(box.bottom + 4 + PEEK_ROOM > window.innerHeight ? { bottom: Math.max(8, window.innerHeight - box.top + 4), left } : { top: box.bottom + 4, left });
  }, [subject.key]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      ui.setPeeking(undefined);
    };
    const onDown = (event: MouseEvent) => {
      if (panel.current?.contains(event.target as Node) !== true) ui.setPeeking(undefined);
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [ui]);
  const rows: SessionThreadRow[] = useMemo(
    () => (session === undefined ? subject.peek.map((text, index): SessionThreadRow => ({ id: `${subject.key}:${index}`, at: "", kind: "tool", text })) : threadRowsOf(session).slice(-PEEK_ROWS)),
    [session, subject.key, subject.peek],
  );
  if (rows.length === 0) return null;
  return createPortal(
    <div ref={panel} className="proto-peek" role="region" aria-label={`Peek at ${subject.title}`} style={at === undefined ? { visibility: "hidden" } : at}>
      <header className="proto-peek__band">
        <span>{subject.title}</span>
        <Tooltip label="Open">
          <Button variant="ghost" size="sm" iconOnly aria-label={`Open ${subject.title}`} onClick={() => acts.open(subject)}>
            <ExternalLink size={14} aria-hidden />
          </Button>
        </Tooltip>
      </header>
      <div className="proto-peek__thread">
        <SessionThread
          sessionId={subject.id}
          rows={rows}
          working={session === undefined ? subject.stoppable : session.turn.state === "working"}
          onAnswer={() => {}}
          onOpenSession={() => acts.open(subject)}
          agentName={session === undefined ? "Drone" : "Agent"}
        />
      </div>
      {session === undefined ? null : (
        <div className="proto-peek__foot">
          <SessionBox session={session} onOpen={() => acts.open(subject)} />
        </div>
      )}
    </div>,
    document.body,
  );
}

type KeysOf = {
  ship: Ship;
  /** Subjects in the order j and k walk them. */
  order: readonly Subject[];
  ui: ShipUi;
  acts: Acts;
  /** A call's card is in front, and has the keys. */
  cardOpen: boolean;
  openCall: () => void;
  /** A direction's own keys and arrows, tried first; true says it took the press. */
  extra?: (event: KeyboardEvent, current: Subject | undefined) => boolean;
};

/**
 * j and k and the arrows move, Enter opens, r replies, p peeks, s stops, m merges, f fixes red main, w opens
 * the call that arrived, ? lists the keys. A press aimed at a strip or unit acts on that one, so a walk
 * can aim a key at what it names; otherwise the selected one, otherwise the first.
 */
export function useShipKeys({ ship, order, ui, acts, cardOpen, openCall, extra }: KeysOf): Subject | undefined {
  const current = order.find((one) => one.key === ui.selected) ?? order[0];
  useListKeydown((event) => {
    if (cardOpen || chord.armed || chord.took === event || event.metaKey || event.ctrlKey || event.altKey) return;
    if (holdsText(event.target)) return;
    const aimed = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-subject]")?.dataset.subject;
    const subject = order.find((one) => one.key === aimed) ?? current;
    if (extra?.(event, subject) === true) return void event.preventDefault();
    const at = subject === undefined ? -1 : order.indexOf(subject);
    const go = (by: number) => {
      const next = order[Math.min(order.length - 1, Math.max(0, at + by))];
      if (next !== undefined) ui.select(next.key);
    };
    const take = () => event.preventDefault();
    switch (event.key) {
      case "j":
      case "ArrowDown":
      case "ArrowRight":
        return take(), go(1);
      case "k":
      case "ArrowUp":
      case "ArrowLeft":
        return take(), go(-1);
      case "w":
        return take(), openCall();
      case "?":
        return take(), ui.setSheet(!ui.sheet);
      case "Escape":
        if (ui.sheet || ui.peeking !== undefined || ui.replying !== undefined) return take(), ui.setSheet(false), ui.setPeeking(undefined), ui.setReplying(undefined);
        return;
      case "f": {
        const red = ship.mains.find((one) => one.state === "red" && !one.fixing);
        return red === undefined ? undefined : (take(), acts.fix(red));
      }
    }
    if (subject === undefined) return;
    switch (event.key) {
      case "Enter":
        if ((event.target as HTMLElement).tagName === "BUTTON") return;
        return take(), acts.open(subject);
      case "r":
        return subject.repliable ? (take(), ui.select(subject.key), ui.setPeeking(undefined), ui.setReplying(subject.key)) : undefined;
      case "p":
        return take(), ui.select(subject.key), ui.setReplying(undefined), ui.setPeeking(ui.peeking === subject.key ? undefined : subject.key);
      case "s":
        return subject.stoppable ? (take(), acts.stop(subject)) : undefined;
      case "m":
        return subject.mergeable ? (take(), acts.merge(subject)) : undefined;
    }
  });
  return current;
}

/** Fix on red main: the one act main has. */
export function FixAct({ main, acts, labeled = true }: { main: Main; acts: Acts; labeled?: boolean }) {
  if (main.state !== "red" || main.fixing) return null;
  return (
    <Button variant="primary" size="sm" onClick={() => acts.fix(main)} aria-label={`Fix main in ${main.repo}`}>
      {labeled ? "Fix" : null}
      <Kbd>{KEYS.fix}</Kbd>
    </Button>
  );
}

const SHEET: readonly (readonly [readonly string[], string])[] = [
  [["j", "k", "←", "↑", "↓", "→"], "Move"],
  [["↵"], "Open"],
  [["r"], "Reply"],
  [["p"], "Peek"],
  [["s"], "Stop"],
  [["m"], "Merge"],
  [["f"], "Fix main"],
  [["w"], "Open the call that arrived"],
  [["?"], "These keys"],
];

/** The keys, listed. A direction adds its own rows. */
export function KeySheet({ ui, more = [] }: { ui: ShipUi; more?: readonly (readonly [readonly string[], string])[] }) {
  if (!ui.sheet) return null;
  return (
    <div className="proto-sheet" role="region" aria-label="Keys">
      <header className="proto-sheet__band">Keys</header>
      <dl className="proto-sheet__rows">
        {[...SHEET, ...more].map(([keys, does]) => (
          <div key={does} className="proto-sheet__row">
            <dt>
              {keys.map((one) => (
                <Kbd key={one}>{one}</Kbd>
              ))}
            </dt>
            <dd>{does}</dd>
          </div>
        ))}
      </dl>
    </div>
  );
}

/**
 * The registry glyph each station wears. Borrowed, not minted: the rail's own for the merge line, the
 * Autopilot lane's aircraft, the bays' rows, the Sessions and Jobs kind marks, and a commit for main.
 */
export const STATION_GLYPH: Record<StationId, LucideIcon> = {
  sessions: SquareTerminal,
  jobs: Box,
  checks: ShieldCheck,
  merge: Merge,
  main: GitCommitHorizontal,
  bays: Rows3,
  autopilot: Plane,
};

const VOTE: Record<Exclude<Vote, "off">, { Glyph: LucideIcon }> = {
  go: { Glyph: Check },
  hold: { Glyph: Pause },
  nogo: { Glyph: X },
};

type Marked = Pick<Station, "id" | "name" | "vote"> & { why?: readonly string[] };

/** What a station says of itself in words: its name while it is well, else one line per exception. Never the vote. */
export const stationLines = (station: Marked): string[] => (station.why === undefined || station.why.length === 0 ? [station.name] : station.why.map((line) => `${station.name}: ${line}`));

/** A station as one mark: its own glyph, and the check, pause or cross it polled beside it. The tooltip names why, in words. */
export function StationMark({ station, size = 16 }: { station: Marked; size?: 12 | 16 | 20 }) {
  const Glyph = STATION_GLYPH[station.id];
  const vote = station.vote === "off" ? undefined : VOTE[station.vote];
  const lines = stationLines(station);
  return (
    <Tooltip
      label={
        <span className="proto-why">
          {lines.map((line) => (
            <span key={line}>{line}</span>
          ))}
        </span>
      }
    >
      <span className="proto-station" data-vote={station.vote} role="img" aria-label={lines.join("; ")}>
        <Glyph size={size} strokeWidth={2} aria-hidden />
        {vote === undefined ? null : <vote.Glyph size={12} strokeWidth={2.5} aria-hidden className="proto-station__vote" />}
      </span>
    </Tooltip>
  );
}
