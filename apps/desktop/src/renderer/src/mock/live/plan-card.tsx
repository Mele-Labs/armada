// The page the `now-and-plain-plans` Session builds, drawn in its live view: the plan card as it was, then with its groups folded under
// Details, then with a summary in plain words. The real card, on an example plan, at the stage the scenario has reached (`stage.ts`).

import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { SessionThread, type PlanApproval } from "@armada/components";

import "../../styles/index.css";
import "@armada/components/src/look";
import "./live.css";
import { onStage, stageNow } from "./stage";

const GROUPS: PlanApproval["groups"] = [
  { id: "g1", agent: "rust-engineer", title: "Retry the slot lease when the pool is contended, with back off", gives: "A Job waits for a free slot instead of failing", paths: ["crates/fleet"], tasks: [{ id: "T1", text: "Retry the lease when the pool is contended" }, { id: "T2", text: "Back off between attempts" }] },
  { id: "g2", agent: "protocol-engineer", title: "Attempt count on the wire", gives: "Bridge knows how many tries it took", paths: ["crates/ipc", "packages/protocol"], tasks: [{ id: "T3", text: "Add attempts to the lease reply" }] },
  { id: "g3", agent: "bridge-engineer", title: "Attempt on the bay", gives: "The bay shows the try it is on", paths: ["apps/desktop"], tasks: [{ id: "T4", text: "Show the attempt on the bay tile" }] },
];

/** What the summary says at each stage: none, the groups counted by where they work, then what he gets. */
const SUMMARY: readonly (string | undefined)[] = [
  undefined,
  "Three groups across crates/fleet, crates/ipc, packages/protocol and apps/desktop.",
  "A Job that finds every slot busy waits and tries again, instead of failing.\nIts bay shows which try it is on.",
];

function PlanCardPage() {
  const [stage, setStage] = useState(stageNow);
  useEffect(() => onStage(setStage), []);
  const summary = SUMMARY[Math.min(stage, SUMMARY.length - 1)];
  const approval: PlanApproval = { version: 1, ...(summary === undefined ? {} : { summary }), groups: GROUPS, approved: false, onApprove: () => undefined, onSetAside: () => undefined };
  return <SessionThread rows={[]} approval={approval} onAnswer={() => undefined} onOpenSession={() => undefined} />;
}

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <PlanCardPage />
    </StrictMode>,
  );
}
