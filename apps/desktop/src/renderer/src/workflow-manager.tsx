// The Workflow manager, as the window draws it under Kit › Workflows. A mock for now: the workflows, the
// nudged Jobs and the agent's answer are the fixture in `mock/workflows-managed.ts`, and every edit lives in
// this component's state until the owner has walked it. Nothing here reaches Fleet.

import { useCallback, useEffect, useRef, useState } from "react";

import { WorkflowManager, keyOf } from "@armada/components";
import type { AgentState, ManagedStep, ManagedWorkflow, WorkflowRef } from "@armada/components";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { MANAGED_DRIFT, MANAGED_WORKFLOWS, WIRE_AND_TESTS } from "./mock/workflows-managed";

type Surfaced = Pick<BridgeState, "holds" | "repository">;

/** How long the agent is shown working before its diff comes back. */
const WORKING_MS = 1200;

/** What the agent makes of a request: the split the owner asked for, and no change to anything else. */
function answerTo(request: string, steps: readonly ManagedStep[]): AgentState {
  if (!/split\s+refine/i.test(request) || !steps.some((one) => one.id === "refine")) return { phase: "unchanged" };
  return { phase: "proposed", before: steps, after: steps.flatMap((one) => (one.id === "refine" ? WIRE_AND_TESTS : [one])) };
}

export function WorkflowManagerSurface({
  bridge,
  onCopied,
  state,
}: {
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  state: Surfaced;
}) {
  const repository = state.holds.repositories?.find((one) => one.root === state.repository)?.manifest?.repository ?? "repository";
  const [workflows, setWorkflows] = useState<readonly ManagedWorkflow[]>(MANAGED_WORKFLOWS);
  const [picked, setPicked] = useState<WorkflowRef | null>(MANAGED_WORKFLOWS[0] ?? null);
  const [agent, setAgent] = useState<AgentState>({ phase: "idle" });
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const pick = useCallback((one: WorkflowRef) => {
    window.clearTimeout(timer.current);
    setAgent({ phase: "idle" });
    setPicked({ id: one.id, source: one.source });
  }, []);

  const change = (one: WorkflowRef, edit: (was: ManagedWorkflow) => ManagedWorkflow) =>
    setWorkflows((all) => all.map((each) => (keyOf(each) === keyOf(one) ? edit(each) : each)));
  const armadaOf = (one: WorkflowRef) => workflows.find((each) => each.source === "armada" && each.id === one.id);
  const stepsOf = (one: WorkflowRef) => workflows.find((each) => keyOf(each) === keyOf(one))?.steps ?? [];

  /** The agent is shown working, then answers. */
  const answer = (steps: readonly ManagedStep[], request: string) => {
    setAgent({ phase: "working" });
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setAgent(answerTo(request, steps)), WORKING_MS);
  };
  const asked = useRef("");

  return (
    <Boundary region="Workflows" bridge={bridge} onCopied={onCopied}>
      <WorkflowManager
        repository={repository}
        workflows={workflows}
        drift={MANAGED_DRIFT}
        picked={picked}
        onPick={pick}
        agent={agent}
        onMakeCopy={(one) => {
          const mine: WorkflowRef = { id: one.id, source: "yours" };
          const from = workflows.find((each) => keyOf(each) === keyOf(one));
          if (!workflows.some((each) => keyOf(each) === keyOf(mine)) && from !== undefined) setWorkflows((all) => [...all, { ...from, source: "yours" }]);
          pick(mine);
        }}
        onPropose={() => undefined}
        onSaveWords={(one, stepId, words) =>
          change(one, (was) => ({ ...was, steps: was.steps.map((step) => (step.id === stepId ? { ...step, words } : step)) }))
        }
        onReset={(one) => {
          const armada = armadaOf(one);
          if (armada !== undefined) change(one, (was) => ({ ...was, steps: armada.steps }));
        }}
        onDelete={(one) => {
          const rest = workflows.filter((each) => keyOf(each) !== keyOf(one));
          setWorkflows(rest);
          pick(rest.find((each) => each.id === one.id) ?? rest[0] ?? { id: "", source: "armada" });
        }}
        onShare={() => undefined}
        onAsk={(one, request) => {
          asked.current = request;
          answer(stepsOf(one), request);
        }}
        onApprove={(one) => {
          if (agent.phase === "proposed") {
            const after = agent.after;
            change(one, (was) => ({ ...was, steps: after }));
          }
          setAgent({ phase: "idle" });
        }}
        onSendBack={(one) => answer(stepsOf(one), asked.current)}
      />
    </Boundary>
  );
}
