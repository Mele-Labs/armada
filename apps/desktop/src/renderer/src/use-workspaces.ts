// The Workspaces and autonomy layers Fleet keeps, as System > Workspaces and the repository picker hold them (armada-cwj). Read when the window's repositories change and after every save: no event
// says a piece changed, and the acts here are the only ones that change them from this window. A press is sent in order, each planned from what Fleet held when the one before it was read back.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Autonomy, AutonomyKind, AutonomyLayers, Repository, WorkspaceRow, WorkspacesPick } from "@armada/components";
import type { RepositorySummary, WorkspaceOfRepositories } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { missing } from "./lifecycle-editing/fleet-pieces";
import { autonomyActs, layersOf, membersAfter, perform, readAutonomy, readWorkspaces, removalActs, rowOf, servedOf, workspaceAct, workspaceIdFrom } from "./workspaces-fleet";
import type { Act, LayerFile, WorkspacesApi } from "./workspaces-fleet";

export type WorkspacesHeld = {
  /** The repositories with a Manifest, which a Workspace can hold. */
  served: readonly Repository[];
  workspaces: readonly WorkspaceRow[];
  layers: AutonomyLayers;
  picked: WorkspacesPick;
  pick: (next: WorkspacesPick) => void;
  /** Fleet's words for the last read or save it refused. Absent when it was kept. */
  notice: string | undefined;
  set: (key: string, kind: AutonomyKind, level: Autonomy | undefined) => void;
  create: (name: string, repositories: readonly string[]) => void;
  save: (id: string, name: string, repositories: readonly string[]) => void;
  remove: (id: string) => void;
};

const NOTHING: readonly never[] = [];

export function useWorkspaces(api: () => WorkspacesApi, repositories: readonly RepositorySummary[]): WorkspacesHeld {
  // The window's list is a new array on every render it is empty, so the reads follow what it names.
  const key = repositories.map((one) => (one.manifest === undefined ? "" : `${one.manifest.id}:${one.manifest.repository}`)).join("|");
  const served = useMemo(() => servedOf(repositories), [key]); // eslint-disable-line react-hooks/exhaustive-deps
  const [read, setRead] = useState<{ held: readonly WorkspaceOfRepositories[]; files: readonly LayerFile[] }>({ held: NOTHING, files: NOTHING });
  const [picked, pick] = useState<WorkspacesPick>({ kind: "everywhere" });
  const [notice, setNotice] = useState<string | undefined>(undefined);
  const now = useRef({ served, read });
  now.current = { served, read };
  const reading = useRef(0);
  const queue = useRef<Promise<void>>(Promise.resolve());

  /** Reads both back. `said` is the refusal of the press that led here, which stays on the page unless a read is refused in its turn. */
  const refresh = useCallback(async (said?: string): Promise<void> => {
    const mine = ++reading.current;
    const among = now.current.served;
    const [workspaces, autonomy] = await Promise.all([readWorkspaces(api()), readAutonomy(api(), among)]);
    if (mine !== reading.current) return;
    const failed = !workspaces.ok ? workspaces.outcome : !autonomy.ok ? autonomy.outcome : undefined;
    if (failed !== undefined) {
      // A Fleet without the routes has nothing to say, and the page draws what it holds.
      setNotice(missing(failed) ? undefined : refusalWords(failed));
      return;
    }
    if (!workspaces.ok || !autonomy.ok) return;
    setNotice(said);
    const next = { held: workspaces.workspaces, files: autonomy.files };
    now.current = { ...now.current, read: next };
    setRead(next);
  }, [api]);

  useEffect(() => {
    void refresh();
  }, [key, refresh]);

  /** Sends what a press comes to, planned when its turn comes from what Fleet held after the press before it, and reads Fleet back. */
  const act = useCallback(
    (plan: (files: readonly LayerFile[], held: readonly WorkspaceOfRepositories[]) => Act[], after?: () => void): void => {
      queue.current = queue.current.then(async () => {
        const done = await perform(api(), plan(now.current.read.files, now.current.read.held));
        if (done.ok) after?.();
        await refresh(done.ok || done.said === "" ? undefined : done.said);
      });
    },
    [api, refresh],
  );

  return {
    served,
    workspaces: useMemo(() => read.held.map((one) => rowOf(one, served)), [read.held, served]),
    layers: useMemo(() => layersOf(read.files), [read.files]),
    picked,
    pick,
    notice,
    set: (level, kind, value) => act((files) => autonomyActs(files, level, kind, value)),
    create: (name, ids) => {
      const id = workspaceIdFrom(name, now.current.read.held.map((one) => one.id), served.map((one) => one.id));
      act(() => [workspaceAct({ id, name, repositories: [...ids] }, false)], () => pick({ kind: "workspace", id }));
    },
    save: (id, name, ids) =>
      act((_files, held) => {
        const was = held.find((one) => one.id === id);
        return was === undefined ? [] : [workspaceAct({ id, name, repositories: membersAfter(was.repositories, ids, served) }, was.home === undefined)];
      }),
    remove: (id) => act((files) => removalActs(files, id), () => pick({ kind: "everywhere" })),
  };
}
