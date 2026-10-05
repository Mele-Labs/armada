// The Workflow creator surface, as the window draws it. **A mock**: the list
// and the definitions are written down in `@armada/components`, and the one
// thing that leaves the screen is the draft handed to Helm.

import { MOCK_DEFINITIONS, MOCK_ENTRIES, MOCK_MANIFESTS, MOCK_REPOSITORY, WorkflowCreator } from "@armada/components";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { askHelm } from "./commands";

export function WorkflowCreatorSurface({
  bridge,
  onCopied,
}: {
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
}) {
  return (
    <Boundary region="Workflows" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <WorkflowCreator
          repository={MOCK_REPOSITORY}
          manifests={MOCK_MANIFESTS}
          entries={MOCK_ENTRIES}
          definitions={MOCK_DEFINITIONS}
          onDiscuss={(draft) => void askHelm(draft)}
        />
      </div>
    </Boundary>
  );
}
