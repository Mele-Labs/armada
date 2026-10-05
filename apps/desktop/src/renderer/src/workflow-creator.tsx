// The Workflow creator, as the window draws it: the definitions Fleet holds for
// the picked repository, one open for editing, and a save through Fleet.
//
// **Fleet is the authority on every refusal.** What a save is refused for comes
// back in Fleet's sentence and is drawn as it said it; the creator checks only
// what the loader would refuse for the shape of the draft. A repository nobody
// picked, or one with no Manifest, has nothing to list and draws nothing.

import { useCallback, useEffect, useMemo, useState } from "react";

import { entriesOf, readDefinition, WORKFLOW_KIT, WorkflowCreator, writeDefinition } from "@armada/components";
import type { WorkflowDefinitionDraft, WorkflowEntry, WorkflowRead, WorkflowSavedAnswer } from "@armada/components";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import { said } from "@armada/screens/src/copy";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { askHelm, readWorkflowDefinition, readWorkflows, saveWorkflow } from "./commands";
import { contextOf } from "./helm-context";

/** The rail mark on Workflows while Fleet has left a file out: a file that cannot run is not something to find by opening the surface. */
export function workflowsWarned(health: HealthRead): Readonly<Record<string, string>> {
  return health.state === "read" && health.health.workflows_left_out === true ? { workflows: "A workflow file cannot run" } : {};
}

/** Fleet's spelling of a place, for the definition route. */
const WIRE = { carried: "armada", kit: "kit", repository: "repository" } as const;

type Surfaced = Pick<BridgeState, "holds" | "repository">;

export function WorkflowCreatorSurface({
  bridge,
  onCopied,
  state,
}: {
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  state: Surfaced;
}) {
  const manifest = state.holds.repositories?.find((one) => one.root === state.repository)?.manifest;
  const current = manifest?.id ?? null;
  return (
    <Boundary region="Workflows" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        {current === null ? null : (
          <Held
            key={current}
            current={current}
            manifests={state.holds.manifests.map((one) => ({ id: one.id, name: one.repository }))}
          />
        )}
      </div>
    </Boundary>
  );
}

function Held({ current, manifests }: { current: string; manifests: { id: string; name: string }[] }) {
  const [read, setRead] = useState<Awaited<ReturnType<typeof readWorkflows>> | null>(null);
  const reread = useCallback(async () => setRead(await readWorkflows()), []);
  useEffect(() => {
    void reread();
  }, [reread]);

  const entries = useMemo(() => (read?.ok === true ? entriesOf(read.workflows, read.leftOut) : []), [read]);

  const onRead = useCallback(
    async (entry: WorkflowEntry): Promise<WorkflowRead> => {
      const answer = await readWorkflowDefinition(entry.id, WIRE[entry.source]);
      if (!answer.ok) return { ok: false, said: said(answer.outcome) };
      return readDefinition(answer.definition.definition, entry.source, current);
    },
    [current],
  );

  const onSave = useCallback(
    async (def: WorkflowDefinitionDraft, overwrite: boolean): Promise<WorkflowSavedAnswer> => {
      const kit = def.scope === WORKFLOW_KIT;
      const answer = await saveWorkflow({
        manifestId: kit ? null : def.scope,
        body: { scope: kit ? "kit" : "repository", definition: writeDefinition(def), ...(overwrite ? { overwrite: true } : {}) },
      });
      if (answer.ok) {
        await reread();
        return { ok: true };
      }
      const { outcome } = answer;
      if (outcome.ok === false && outcome.why === "refused") {
        return {
          ok: false,
          said: outcome.error.message,
          ...(outcome.error.code === "fleet.workflow_exists" ? { exists: true as const } : {}),
        };
      }
      return { ok: false, said: said(outcome) };
    },
    [reread],
  );

  return (
    <WorkflowCreator
      repository={current}
      manifests={manifests}
      entries={entries}
      onRead={onRead}
      onSave={onSave}
      onDiscuss={(draft) =>
        void askHelm(draft, contextOf({ screen: "workflows", picked: current, chip: null, cursor: null }))
      }
    />
  );
}
