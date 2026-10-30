// The Workspace the Workflows page draws when no scenario hands one: the lifecycles and rules Fleet keeps for the repository the page is on, edited by hand and saved as they are
// changed. **A change is drawn at once and kept in order**: each is compared with the Workspace as it was drawn (`write.ts`), the pieces it changes are saved or deleted one change after
// another, and Fleet is read again after each. What the last read says is what is drawn once nothing is waiting, so a piece Fleet refused goes back to what it holds.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Ref, Workspace } from "@armada/components";
import type { LifecycleRule, PieceToCommit } from "@armada/protocol";

import type { WorkspaceHeld } from "../workspace-draft";
import { readPieces, runOps } from "./fleet-pieces";
import type { PiecesApi, Served } from "./fleet-pieces";
import { lifecycleLayers, ruleLayers } from "./levels";
import type { PiecesRead } from "./levels";
import { landing, namedOf, withDrawn, withPlaces, workspaceOf } from "./read";
import { kitPieceAt, opsFrom, withWires } from "./write";
import type { PieceId } from "./write";

export type FleetWorkspace = {
  held: WorkspaceHeld;
  /** This Fleet does not keep a piece that is saved: the editor is drawn and cannot change it. */
  readOnly: boolean;
  /** What the last change could not do, in Fleet's own words. */
  notice: string | undefined;
  /** The piece of the person's own that a pick on a tab runs, which Commit to repository copies, and the repository it must go to where its scope is a Workspace's. */
  pieceAt: (pick: { kind: "rule" | "lifecycle"; id: string }, tab: string) => { piece: PieceToCommit; home?: string } | undefined;
  /** Whether a rule lands on a node of the lanes, which is how it is drawn and picked. */
  lands: (rule: LifecycleRule) => boolean;
  /** Whether a connector from one node to another makes a rule Fleet can run: a ticket moved to a state, or a pull request made ready or merged. */
  canConnect: (from: Ref, to: Ref) => boolean;
  /** The rules and lifecycles whose removal waits on a pull request, as `rule:<id>` and `lifecycle:<id>`. */
  pending: ReadonlySet<string>;
};

const keyOf = (piece: PieceId): string => `${piece.kind}:${piece.id}`;

/** Whether a repository still holds a piece a pull request is removing. */
function stillHeld(read: PiecesRead, key: string): boolean {
  const [kind, id = ""] = key.split(":");
  const layers = (kind === "rule" ? ruleLayers(read) : lifecycleLayers(read)).get(id);
  return [...(layers?.values() ?? [])].some((one) => one.kept.rank.source === "repository");
}

/** The Workspace Fleet keeps for `current`, or none while it is read, where there is no Manifest, or where Fleet has no lifecycle to draw. `workflows` are the ids the lanes are placed for. */
export function useFleetWorkspace(api: () => PiecesApi, current: string | null, served: readonly Served[], workflows: readonly string[], enabled: boolean): FleetWorkspace | undefined {
  const [read, setRead] = useState<PiecesRead | undefined>(undefined);
  const [shown, setShown] = useState<Workspace | undefined>(undefined);
  const [readOnly, setReadOnly] = useState(false);
  const [notice, setNotice] = useState<string | undefined>(undefined);
  const [pending, setPending] = useState<ReadonlySet<string>>(new Set());
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
      // A piece the repository no longer holds has gone with the pull request that removed it.
      setPending((was) => (was.size === 0 ? was : new Set([...was].filter((key) => stillHeld(answer.read, key)))));
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
      // A rule a connector drew is kept as the piece the connector comes to, and each rule is drawn from its piece.
      const made = withDrawn(withWires(before, next));
      setShown(made);
      if (opsFrom(kept, before, made).length === 0) return;
      waiting.current += 1;
      queue.current = queue.current.then(async () => {
        try {
          // Against what Fleet holds now: the change before this one has been saved and read back.
          const ran = await runOps(latest.current.api(), opsFrom(latest.current.read ?? kept, before, made), latest.current.served);
          if (!open.current) return;
          if (!ran.ok) {
            if (ran.old) setReadOnly(true);
            else setNotice(ran.said);
          } else if (ran.removing.length > 0) setPending((was) => new Set([...was, ...ran.removing.map(keyOf)]));
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
      const scope = found.scope;
      const home = scope.kind === "within" ? read.workspaces.find((one) => one.id === scope.id)?.home : undefined;
      return { piece: { kind: found.kind === "rule" ? "lifecycle_rule" : "lifecycle", id: found.id, scope: found.scope }, ...(home === undefined ? {} : { home }) };
    },
    [read],
  );
  const lands = useCallback<FleetWorkspace["lands"]>((rule) => landing(rule, namedOf(drawn.current?.lifecycles ?? [])) !== undefined, []);
  const canConnect = useCallback<FleetWorkspace["canConnect"]>((from, to) => {
    const lifecycles = drawn.current?.lifecycles ?? [];
    const lane = lifecycles.find((one) => one.id === to.lane);
    if (lane === undefined || (from.lane !== "workflow" && !lifecycles.some((one) => one.id === from.lane))) return false;
    // A ticket is moved to any state of its lifecycle; a pull request is made ready or merged; no act brings about any other.
    return lane.kind === "tracker" || lane.kind === undefined || (lane.kind === "forge" && (to.item === "ready" || to.item === "merged"));
  }, []);
  return held === undefined ? undefined : { held, readOnly, notice, pieceAt, lands, canConnect, pending };
}
