// The Workflow manager, as the window draws it under Kit › Workflows, over what Fleet holds: the list from
// `GET /workflows`, a copy's steps from `GET /workflows/steps`, its words saved through `save_workflow`, and the
// changes an agent drafts read again while one is drafting. **Fleet is the authority on every refusal** and
// each is drawn as it said it. `docs/practices/protocol/the-workflows-page.md`.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { WorkflowManager, forRequestsOf, keyOf, withWords } from "@armada/components";
import type { DriftedJob, ManagedStep, ManagedWorkflow, WorkflowActAnswer, WorkflowRef } from "@armada/components";
import type { Outcome, WorkflowChange } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import type { WorkflowsRead } from "../../shared/workflows";
import {
  approveWorkflowChange,
  askWorkflowChange,
  buildWorkflowSupport,
  deleteWorkflowCopy,
  dismissWorkflowChange,
  markNotADrift,
  proposeWorkflowToArmada,
  readWorkflowChanges,
  readWorkflowDefinition,
  readWorkflowSteps,
  readWorkflows,
  resetWorkflowCopy,
  saveWorkflow,
  sendBackWorkflowChange,
  suggestWorkflowChange,
} from "./commands";
import { askToOpenSessionInCockpit } from "./open-session";
import { PieceActs } from "./sharing/PieceActs";
import { WIRE_OF, agentOf, armadaOf, askedChange, changeAt, driftOf, listedOf, stepOf, suggestionsOf } from "./workflow-manager-wire";
import type { Listed } from "./workflow-manager-wire";

type Surfaced = Pick<BridgeState, "holds" | "repository">;

/** A repository with a Manifest: what names its copies' group and what a share of one is made under. */
type Served = { manifestId: string; name: string; root: string };

/** How often the changes are read again while an agent is drafting one. */
const POLL_MS = 1000;

/** What was read of one copy: its steps, the Jobs Fleet nudged at them, and the line it says it is for. */
type Read = { steps: readonly ManagedStep[]; drift: Readonly<Record<string, readonly DriftedJob[]>>; forRequests: string };

/** Fleet's place for a copy: the page's `yours` is Kit's. */
const wireOf = (one: WorkflowRef) => WIRE_OF[one.source];

const refusedWith = (outcome: Outcome, code: string): boolean => !outcome.ok && outcome.why === "refused" && outcome.error.code === code;

export function WorkflowManagerSurface({
  bridge,
  onCopied,
  state,
}: {
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  state: Surfaced;
}) {
  const manifest = state.holds.repositories?.find((one) => one.root === state.repository)?.manifest;
  const served = (state.holds.repositories ?? []).flatMap((one): Served[] =>
    one.manifest === undefined ? [] : [{ manifestId: one.manifest.id, name: one.manifest.repository, root: one.root }],
  );
  return (
    <Boundary region="Workflows" bridge={bridge} onCopied={onCopied}>
      <Held key={manifest?.id ?? ""} current={manifest?.id ?? null} repository={manifest?.repository ?? "repository"} root={state.repository} served={served} />
    </Boundary>
  );
}

function Held({ current, repository, root, served }: { current: string | null; repository: string; root: string | null; served: readonly Served[] }) {
  const [listed, setListed] = useState<WorkflowsRead | null>(null);
  const [reads, setReads] = useState<Readonly<Record<string, Read>>>({});
  const [picked, setPicked] = useState<WorkflowRef | null>(null);
  const [changes, setChanges] = useState<readonly WorkflowChange[]>([]);
  const [notice, setNotice] = useState<string | undefined>(undefined);
  // The copy the changes below belong to, so an answer that comes back after another was picked is let go.
  const asking = useRef<string | null>(null);

  /** The name of the repository a Manifest is, for the group its copies sit in. */
  const nameOf = useCallback(
    (manifestId: string | undefined): { repository?: string } => {
      const name = served.find((one) => one.manifestId === manifestId)?.name;
      return name === undefined ? {} : { repository: name };
    },
    [served],
  );

  const rows = useMemo((): readonly Listed[] => (listed === null || !listed.ok ? [] : listedOf(listed.workflows, listed.leftOut, current)), [listed, current]);
  const workflows = useMemo(
    (): readonly ManagedWorkflow[] =>
      rows.map((row) => {
        const read = reads[keyOf(row)];
        return {
          id: row.id,
          source: row.source,
          ...(row.manifestId === undefined ? {} : { manifestId: row.manifestId }),
          ...(row.source === "repository" ? nameOf(row.manifestId) : {}),
          forRequests: read?.forRequests ?? row.forRequests ?? "",
          steps: read?.steps ?? [],
          ...(row.leftOut === undefined ? {} : { leftOut: row.leftOut }),
        };
      }),
    [rows, reads, nameOf],
  );

  // Every read and act names the Manifest the copy was read under, which is not the pick's on All.
  const manifestFor = (one: WorkflowRef) => one.manifestId ?? null;
  const copyOf = (one: WorkflowRef) => ({ workflow_id: one.id, source: wireOf(one), ...(one.file === undefined ? {} : { file: one.file }) });

  const reread = useCallback(async (): Promise<WorkflowsRead> => {
    const answer = await readWorkflows();
    setListed(answer);
    setNotice(answer.ok ? undefined : refusalWords(answer.outcome));
    return answer;
  }, []);

  /** Reads one copy's steps and the line it is for, from Fleet. */
  const load = useCallback(async (one: WorkflowRef): Promise<void> => {
    if (one.file !== undefined) return;
    const [steps, definition] = await Promise.all([
      readWorkflowSteps(one.id, WIRE_OF[one.source], one.manifestId ?? null),
      readWorkflowDefinition(one.id, WIRE_OF[one.source], one.manifestId ?? null),
    ]);
    if (!steps.ok) {
      setNotice(refusalWords(steps.outcome));
      return;
    }
    const forRequests = definition.ok ? (forRequestsOf(definition.definition.definition) ?? "") : "";
    setReads((all) => ({
      ...all,
      [keyOf(one)]: { steps: steps.steps.steps.map(stepOf), drift: driftOf(steps.steps.steps, Date.now()), forRequests },
    }));
  }, []);

  /** Reads the changes open on a copy, and keeps them if it is still the one picked. */
  const loadChanges = useCallback(async (one: WorkflowRef): Promise<void> => {
    if (one.source === "armada" || one.file !== undefined) {
      setChanges([]);
      return;
    }
    const answer = await readWorkflowChanges(one.id, WIRE_OF[one.source], one.manifestId ?? null);
    if (asking.current === keyOf(one) && answer.ok) setChanges(answer.changes.changes);
  }, []);

  const open = useCallback(
    (one: WorkflowRef, armada: Listed | undefined) => {
      const ref: WorkflowRef = { id: one.id, source: one.source, ...(one.manifestId === undefined ? {} : { manifestId: one.manifestId }), ...(one.file === undefined ? {} : { file: one.file }) };
      asking.current = keyOf(ref);
      setPicked(ref);
      setChanges([]);
      void load(ref);
      void loadChanges(ref);
      // Compared with Armada's, and reset to it, so its steps are read with the copy's.
      if (ref.source !== "armada" && armada !== undefined) void load({ id: ref.id, source: "armada", ...(armada.manifestId === undefined ? {} : { manifestId: armada.manifestId }) });
    },
    [load, loadChanges],
  );

  const first = useRef(true);
  useEffect(() => {
    void reread().then((answer) => {
      if (!first.current || !answer.ok) return;
      first.current = false;
      const all = listedOf(answer.workflows, answer.leftOut, current);
      const start = all.find((one) => one.source === "armada" && one.leftOut === undefined) ?? all.find((one) => one.leftOut === undefined);
      if (start !== undefined) open({ id: start.id, source: start.source, ...(start.manifestId === undefined ? {} : { manifestId: start.manifestId }) }, armadaOf(all, start.id));
    });
  }, [reread, open, current]);

  const drafting = changes.some((one) => one.state === "drafting");
  useEffect(() => {
    if (!drafting || picked === null) return;
    const timer = window.setInterval(() => void loadChanges(picked), POLL_MS);
    return () => window.clearInterval(timer);
  }, [drafting, picked, loadChanges]);

  /** What an act changed, read again: the list, the copy's steps and its changes. */
  const refresh = async (one: WorkflowRef): Promise<void> => {
    await Promise.all([reread(), load(one), loadChanges(one)]);
  };

  const readOfPicked = picked === null ? undefined : reads[keyOf(picked)];
  const known = readOfPicked?.steps ?? [];
  const { suggestions, suggesting } = suggestionsOf(changes, picked?.id ?? "");
  const stepChange = (stepId: string) => changeAt(changes, stepId);

  /** The action on a change Fleet holds: its answer is the refusal, or nothing, and the changes are read again. */
  const onChange = async (one: WorkflowRef, change: WorkflowChange | undefined, act: (id: number) => Promise<{ ok: true } | { ok: false; outcome: Outcome }>): Promise<WorkflowActAnswer> => {
    if (change === undefined) return undefined;
    const answer = await act(change.id);
    await refresh(one);
    return answer.ok ? undefined : refusalWords(answer.outcome);
  };

  return (
    <WorkflowManager
      repository={repository}
      workflows={workflows}
      notice={notice}
      drift={picked === null ? {} : { [picked.id]: readOfPicked?.drift ?? {} }}
      picked={picked}
      onPick={(one) => open(one, armadaOf(rows, one.id))}
      agent={agentOf(changes, known)}
      suggestions={picked === null ? {} : { [picked.id]: suggestions }}
      suggesting={suggesting}
      actsFor={(one) => {
        // The copy's own repository: on All there is no pick to share under.
        const at = served.find((each) => each.manifestId === one.manifestId)?.root ?? root;
        return at === null ? null : <PieceActs repository={at} piece={{ kind: "workflow", workflow_id: one.id }} />;
      }}
      onMakeCopy={async (one) => {
        const armada = await readWorkflowDefinition(one.id, "armada", manifestFor(one));
        if (!armada.ok) return refusalWords(armada.outcome);
        const saved = await saveWorkflow({ manifestId: manifestFor(one), body: { scope: "kit", definition: armada.definition.definition, overwrite: false } });
        // A copy already under Yours is the one to open.
        if (!saved.ok && !refusedWith(saved.outcome, "fleet.workflow_exists")) return refusalWords(saved.outcome);
        await reread();
        open({ id: one.id, source: "yours", ...(one.manifestId === undefined ? {} : { manifestId: one.manifestId }) }, armadaOf(rows, one.id));
        return undefined;
      }}
      onPropose={async (one) => {
        const armada = await readWorkflowDefinition(one.id, "armada", manifestFor(one));
        if (!armada.ok) return refusalWords(armada.outcome);
        const answer = await proposeWorkflowToArmada({ workflow_id: one.id, definition: armada.definition.definition });
        if (!answer.ok) return refusalWords(answer.outcome);
        const { session_id, send_failed } = answer.proposed;
        return { jobId: session_id, ...(send_failed === undefined ? {} : { note: `Job started on the Armada repository. Brief not sent: ${send_failed}` }) };
      }}
      onSaveWords={async (one, stepId, words) => {
        const held = await readWorkflowDefinition(one.id, wireOf(one), manifestFor(one));
        if (!held.ok) return refusalWords(held.outcome);
        const definition = withWords(held.definition.definition, stepId, words);
        if (definition === undefined) return `${stepId} is not a step of ${held.definition.file}`;
        const saved = await saveWorkflow({
          manifestId: manifestFor(one),
          body: { scope: one.source === "repository" ? "repository" : "kit", definition, overwrite: true },
        });
        if (!saved.ok) return refusalWords(saved.outcome);
        await refresh(one);
        return undefined;
      }}
      onReset={async (one) => {
        const answer = await resetWorkflowCopy({ manifestId: manifestFor(one), copy: copyOf(one) });
        if (!answer.ok) return refusalWords(answer.outcome);
        await refresh(one);
        return undefined;
      }}
      onDelete={async (one) => {
        const answer = await deleteWorkflowCopy({ manifestId: manifestFor(one), copy: copyOf(one) });
        if (!answer.ok) return refusalWords(answer.outcome);
        const after = await reread();
        if (!after.ok) return undefined;
        const left = listedOf(after.workflows, after.leftOut, current).filter((each) => each.leftOut === undefined);
        const next = left.find((each) => each.id === one.id) ?? left[0];
        if (next === undefined) setPicked(null);
        else open({ id: next.id, source: next.source, ...(next.manifestId === undefined ? {} : { manifestId: next.manifestId }) }, armadaOf(left, next.id));
        return undefined;
      }}
      onAsk={async (one, text) => {
        const answer = await askWorkflowChange({ manifestId: manifestFor(one), body: { workflow_id: one.id, source: wireOf(one), text } });
        if (!answer.ok) return refusalWords(answer.outcome);
        await loadChanges(one);
        return undefined;
      }}
      onApprove={(one) => onChange(one, askedChange(changes), approveWorkflowChange)}
      onDismiss={(one) => onChange(one, askedChange(changes), dismissWorkflowChange)}
      onSendBack={async (one, notes) => {
        const change = askedChange(changes);
        if (change === undefined) return undefined;
        const answer = await sendBackWorkflowChange({ change: change.id, notes });
        await loadChanges(one);
        return answer.ok ? undefined : refusalWords(answer.outcome);
      }}
      onBuildSupport={async (one) => {
        const change = askedChange(changes);
        if (change === undefined) return undefined;
        const answer = await buildWorkflowSupport({ change: change.id });
        await loadChanges(one);
        return answer.ok ? { jobId: answer.building.job_id } : refusalWords(answer.outcome);
      }}
      onOpenJob={(_one, jobId) => askToOpenSessionInCockpit(jobId)}
      onNotDrift={async (one, stepId, driftId) => {
        const marked = await markNotADrift(Number(driftId));
        if (!marked.ok) return refusalWords(marked.outcome);
        await load(one);
        // A Job that is not a drift no longer counts toward a suggestion; one with nothing left under it is taken back.
        const held = stepChange(stepId);
        if (held?.origin.kind === "drift") {
          const left = new Set((await readWorkflowSteps(one.id, wireOf(one), manifestFor(one)).then((answer) => (answer.ok ? answer.steps.steps.find((each) => each.id === stepId)?.nudges ?? [] : []))).map((each) => each.id));
          if (held.origin.nudges.every((each) => !left.has(each))) await dismissWorkflowChange(held.id);
        }
        await loadChanges(one);
        return undefined;
      }}
      onSuggest={async (one, stepId) => {
        const answer = await suggestWorkflowChange({ manifestId: manifestFor(one), body: { workflow_id: one.id, source: wireOf(one), step_id: stepId } });
        if (!answer.ok) return refusalWords(answer.outcome);
        await loadChanges(one);
        return undefined;
      }}
      onApproveSuggestion={(one, stepId) => onChange(one, stepChange(stepId), approveWorkflowChange)}
      onDismissSuggestion={(one, stepId) => onChange(one, stepChange(stepId), dismissWorkflowChange)}
    />
  );
}
