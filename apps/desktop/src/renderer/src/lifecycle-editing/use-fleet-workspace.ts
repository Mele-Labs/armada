// The Workspace the Workflows page draws when no scenario hands one: the lifecycles and rules Fleet keeps for the repository the page is on, edited by hand and saved as they are
// changed. **A change is drawn at once and kept in order**: each is compared with the Workspace as it was drawn (`write.ts`), the pieces it changes are saved or deleted one change after
// another, and Fleet is read again after each. What the last read says is what is drawn once nothing is waiting, so a piece Fleet refused goes back to what it holds.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Workspace } from "@armada/components";
import type { PieceToCommit } from "@armada/protocol";

import type { WorkspaceHeld } from "../workspace-draft";
import { readPieces, runOps, sent } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import type { PiecesRead } from "./levels";
import { withPlaces, workspaceOf } from "./read";
import { kitPieceAt, opsFrom } from "./write";

export type FleetWorkspace = {
  held: WorkspaceHeld;
  /** This Fleet does not keep a piece that is saved: the editor is drawn and cannot change it. */
  readOnly: boolean;
  /** What the last change could not do, in Fleet's words or the page's own. */
  notice: string | undefined;
  /** The piece of the person's own that a pick on a tab runs, which Commit to repository copies, and the repository it must go to where its scope is a Workspace's. */
  pieceAt: (pick: { kind: "rule" | "lifecycle"; id: string }, tab: string) => { piece: PieceToCommit; home?: string } | undefined;
};

/** The Workspace Fleet keeps for `current`, or none while it is read, where there is no Manifest, or where Fleet has no lifecycle to draw. `workflows` are the ids the lanes are placed for. */
export function useFleetWorkspace(api: () => PiecesApi, current: string | null, served: readonly Served[], workflows: readonly string[], enabled: boolean): FleetWorkspace | undefined {
  const [read, setRead] = useState<PiecesRead | undefined>(undefined);
  const [shown, setShown] = useState<Workspace | undefined>(undefined);
  const [readOnly, setReadOnly] = useState(false);
  const [notice, setNotice] = useState<string | undefined>(undefined);
  const latest = useRef({ read, served, api });
  latest.current = { read, served, api };
  const waiting = useRef(0);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const open = useRef(true);
  // Set again on mount: a strict render mounts, unmounts and mounts once more.
  useEffect(() => {
    open.current = true;
    return () => void (open.current = false);
  }, []);

  /** Reads Fleet again and keeps what it said. Returns whether it could be read. */
  const reload = useCallback(
    async (draw: boolean): Promise<boolean> => {
      if (current === null) return false;
      const answer = await readPieces(latest.current.api(), current, latest.current.served);
      if (!open.current) return false;
      if (!answer.ok) {
        if (!answer.outcome.ok && answer.outcome.why === "refused") setNotice(answer.outcome.error.message);
        return false;
      }
      latest.current = { ...latest.current, read: answer.read };
      setRead(answer.read);
      if (draw) setShown(workspaceOf(answer.read));
      return true;
    },
    [current],
  );

  useEffect(() => {
    if (enabled) void reload(true);
  }, [enabled, reload]);

  const workspace = useMemo(() => (shown === undefined ? undefined : withPlaces(shown, workflows)), [shown, workflows]);
  const drawn = useRef(workspace);
  drawn.current = workspace;

  const change = useCallback(
    (next: Workspace) => {
      const before = drawn.current;
      const kept = latest.current.read;
      if (before === undefined || kept === undefined || readOnly) return;
      setNotice(undefined);
      const ops = opsFrom(kept, before, next);
      const refused = ops.find((one) => one.op === "refuse");
      if (refused !== undefined) return setNotice(refused.said);
      setShown(next);
      const unsaved = ops.find((one) => one.op === "unsaved");
      if (unsaved !== undefined) setNotice(unsaved.said);
      if (sent(ops).length === 0) return;
      waiting.current += 1;
      queue.current = queue.current.then(async () => {
        try {
          // Against what Fleet holds now: the change before this one has been saved and read back.
          const acts = sent(opsFrom(latest.current.read ?? kept, before, next));
          const ran = await runOps(latest.current.api(), acts, latest.current.served);
          if (!open.current) return;
          if (!ran.ok) {
            if (ran.old) setReadOnly(true);
            else setNotice(ran.said);
          } else if (ran.said !== undefined) setNotice(ran.said);
          await reload(false);
        } finally {
          waiting.current -= 1;
          if (waiting.current === 0 && open.current && latest.current.read !== undefined) setShown(workspaceOf(latest.current.read));
        }
      });
    },
    [readOnly, reload],
  );

  const held = useMemo<WorkspaceHeld | undefined>(
    () => (workspace === undefined ? undefined : { workspace, change, ask: () => undefined, add: (rule) => change({ ...workspace, rules: [...workspace.rules, rule] }) }),
    [workspace, change],
  );
  const pieceAt = useCallback<FleetWorkspace["pieceAt"]>(
    (pick, tab) => {
      if (read === undefined) return undefined;
      const found = kitPieceAt(read, pick.kind, pick.id, tab);
      if (found === undefined) return undefined;
      const home = found.scope.kind === "within" && found.scope.id === read.workspace?.id ? read.workspace.home : undefined;
      return { piece: { kind: found.kind === "rule" ? "lifecycle_rule" : "lifecycle", id: found.id, scope: found.scope }, ...(home === undefined ? {} : { home }) };
    },
    [read],
  );
  return held === undefined ? undefined : { held, readOnly, notice, pieceAt };
}
