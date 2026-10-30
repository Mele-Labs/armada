// System > Workspaces over the Workspaces Fleet keeps (armada-cwj). `use-workspaces.ts` holds them; this hands them to the page.

import { WorkspacesPage } from "@armada/components";

import { openSystemEntry } from "./system-entry";
import type { WorkspacesHeld } from "./use-workspaces";

export function WorkspacesSurface({ held }: { held: WorkspacesHeld }) {
  return (
    <WorkspacesPage
      repositories={held.served}
      workspaces={held.workspaces}
      layers={held.layers}
      picked={held.picked}
      notice={held.notice}
      onPick={held.pick}
      onSet={held.set}
      onCreate={held.create}
      onSave={held.save}
      onRemove={held.remove}
      onOpenWorkflows={() => openSystemEntry("kit:workflows")}
    />
  );
}
