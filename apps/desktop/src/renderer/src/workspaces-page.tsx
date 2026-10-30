// System > Workspaces over the Workspaces the window holds (armada-cwj). Mock-only for now: the seed keeps them, and no Fleet call is behind it yet.

import { WorkspacesPage } from "@armada/components";
import type { WorkspaceRow } from "@armada/components";

import { openSystemEntry } from "./system-entry";
import { useWorkspaceHeld } from "./workspace-draft";

export function WorkspacesSurface() {
  const book = useWorkspaceHeld()?.book;
  if (book === undefined) return null;
  const workspaces = book.records.map((one): WorkspaceRow => ({ id: one.id, name: one.name, repositories: one.repositories, ...(one.home === undefined ? {} : { home: one.home }) }));
  return (
    <WorkspacesPage
      repositories={book.served}
      workspaces={workspaces}
      layers={book.layers}
      picked={book.picked}
      onPick={book.pick}
      onSet={book.set}
      onCreate={book.create}
      onSave={book.save}
      onRemove={book.remove}
      onOpenWorkflows={() => openSystemEntry("kit:workflows")}
    />
  );
}
