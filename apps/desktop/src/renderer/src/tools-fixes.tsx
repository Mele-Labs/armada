// The acts behind the tools mark's menu: what puts right each thing a Job's agent came up without. Held here, not in a component, so the Job's
// header and its row in the Jobs list show one run, and a run goes on while the menu is shut.

import { useSyncExternalStore } from "react";

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

const subscribe = (on: () => void) => (listeners.add(on), () => void listeners.delete(on));

/** The Job's own terminal, as its view docks it: the update runs there, where the person sees all of it and can answer a prompt. */
type Terminal = Pick<TerminalDockRun, "start">;

function press(draft: SessionsDraft, id: string, piece: ToolsPiece, terminal: Terminal | undefined): void {
  // Left to the terminal: its own pane shows the run and its exit, so no phase is held here.
  if (piece === "task_tools") {
    terminal?.start(UPDATE);
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
export function useToolsFixes(session: Session, terminal?: Terminal): { fixes: ToolsFix[]; onFix: (piece: string) => void } | undefined {
  const draft = useSessionsDraft();
  const phases = useSyncExternalStore(subscribe, () => held);
  if (session.toolsMissing === undefined || draft === undefined) return undefined;
  const can: Record<ToolsPiece, boolean> = { mod: draft.mendTools !== undefined, door: draft.actOnMcp !== undefined, task_tools: terminal !== undefined };
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
  return { fixes, onFix: (piece) => press(draft, session.id, piece as ToolsPiece, terminal) };
}

/** The tools mark of a Job, with its menu where the build can act on it. Nothing where the Job came up whole. */
export function ToolsMarkOf({ session, size }: { session: Session; size?: 12 | 16 }) {
  const fixing = useToolsFixes(session);
  if (session.toolsMissing === undefined) return null;
  return <ToolsMark said={toolsMissingSaid(session.toolsMissing)} {...(size === undefined ? {} : { size })} {...(fixing ?? {})} />;
}
