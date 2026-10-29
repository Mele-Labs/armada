// The acts behind the tools mark's menu: what puts right each thing a Job's agent came up without. Held here, not in a component, so the Job's
// header and its row in the Jobs list show one run, and a run goes on while the menu is shut.

import { useEffect, useSyncExternalStore } from "react";

import { ToolsMark, type TerminalDockRun, type ToolsFix } from "@armada/components";
import { toolsMissingSaid, toolsPieceSaid, type Session, type SessionsDraft, type ToolsPiece } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "./sessions-draft";

const ACT: Record<ToolsPiece, string> = { mod: "Restart the agent", door: "Reconnect", task_tools: "Update Claude Code" };
const UPDATE = "claude update";

type Phase = { running: true } | { error: string };

let held: ReadonlyMap<string, Phase> = new Map();
const listeners = new Set<() => void>();

function put(key: string, phase: Phase | undefined): void {
  const next = new Map(held);
  if (phase === undefined) next.delete(key);
  else next.set(key, phase);
  held = next;
  listeners.forEach((on) => on());
}

/** A command a row's menu asked for in a Job whose view was not open yet: the view starts it in its terminal once it is drawn. */
let asked: ReadonlyMap<string, string> = new Map();

function ask(id: string, command: string | undefined): void {
  const next = new Map(asked);
  if (command === undefined) next.delete(id);
  else next.set(id, command);
  asked = next;
  listeners.forEach((on) => on());
}

const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));

/** The Job's own terminal, as its view docks it: the update runs there, where the person sees all of it and can answer a prompt. */
type Terminal = Pick<TerminalDockRun, "start">;

function press(draft: SessionsDraft, id: string, piece: ToolsPiece, terminal: Terminal | undefined, open: (() => void) | undefined): void {
  // Left to the terminal: its own pane shows the run and its exit, so no phase is held here. From a row the Job opens first.
  if (piece === "task_tools") {
    if (terminal !== undefined) terminal.start(UPDATE);
    else if (open !== undefined) {
      ask(id, UPDATE);
      open();
    }
    return;
  }
  const key = `${id}:${piece}`;
  if ("running" in (held.get(key) ?? {})) return;
  put(key, { running: true });
  const done = () => put(key, undefined);
  const failed = (error: string) => put(key, { error });
  if (piece === "door") void draft.actOnMcp?.(id, "armada-fleet", "reconnect").then((one) => (one.ok ? done() : failed(one.said)));
  else void draft.mendTools?.(id).then((one) => (one.ok ? done() : failed(one.said)));
}

/** What the mark's menu lists for a Job, with the press on each act. Absent where the Job came up whole. */
export function useToolsFixes(session: Session, terminal?: Terminal, open?: () => void): { fixes: ToolsFix[]; onFix: (piece: string) => void } | undefined {
  const draft = useSessionsDraft();
  const phases = useSyncExternalStore(subscribe, () => held);
  const wanted = useSyncExternalStore(subscribe, () => asked.get(session.id));
  // The Job's view, drawn, starts what a row asked for.
  useEffect(() => {
    if (wanted === undefined || terminal === undefined) return;
    ask(session.id, undefined);
    terminal.start(wanted);
  }, [wanted, terminal, session.id]);
  if (session.toolsMissing === undefined || draft === undefined) return undefined;
  const can: Record<ToolsPiece, boolean> = { mod: draft.mendTools !== undefined, door: draft.actOnMcp !== undefined, task_tools: terminal !== undefined || open !== undefined };
  const fixes = session.toolsMissing.missing.map((piece): ToolsFix => {
    const phase = phases.get(`${session.id}:${piece}`);
    return {
      piece,
      said: toolsPieceSaid(piece),
      ...(can[piece] ? { act: ACT[piece] } : {}),
      // The terminal takes over from the menu.
      ...(piece === "task_tools" ? { closes: true } : {}),
      ...(phase !== undefined && "running" in phase ? { running: true } : {}),
      ...(phase !== undefined && "error" in phase ? { error: phase.error } : {}),
    };
  });
  return { fixes, onFix: (piece) => press(draft, session.id, piece as ToolsPiece, terminal, open) };
}

/** What each Job's person put away the band over its message box for: the pieces missing then. In memory, for the life of the window. */
let away: ReadonlyMap<string, readonly ToolsPiece[]> = new Map();

function putAway(id: string, pieces: readonly ToolsPiece[] | undefined): void {
  const next = new Map(away);
  if (pieces === undefined) next.delete(id);
  else next.set(id, pieces);
  away = next;
  listeners.forEach((on) => on());
}

/**
 * Whether the band over a Job's message box shows, and the act that puts it away. It stays away while every piece missing is one that was missing when
 * it was put away: a piece fixed leaves it away, a piece newly missing brings it back. Nothing missing forgets the dismissal.
 */
export function useToolsBand(session: Session): { shown: boolean; dismiss: () => void } {
  const gone = useSyncExternalStore(subscribe, () => away.get(session.id));
  const missing = session.toolsMissing?.missing;
  useEffect(() => {
    if (missing === undefined && away.has(session.id)) putAway(session.id, undefined);
  }, [missing, session.id]);
  return {
    shown: missing !== undefined && (gone === undefined || missing.some((piece) => !gone.includes(piece))),
    dismiss: () => {
      if (missing !== undefined) putAway(session.id, missing);
    },
  };
}

/** The tools mark of a Job on its row, with its menu where the build can act on it, pulsing: the row has no band. Nothing where the Job came up whole. */
export function ToolsMarkOf({ session, size, open }: { session: Session; size?: 12 | 16; open?: () => void }) {
  const fixing = useToolsFixes(session, undefined, open);
  if (session.toolsMissing === undefined) return null;
  return <ToolsMark said={toolsMissingSaid(session.toolsMissing)} {...(size === undefined ? {} : { size })} {...(fixing ?? {})} apparent />;
}
