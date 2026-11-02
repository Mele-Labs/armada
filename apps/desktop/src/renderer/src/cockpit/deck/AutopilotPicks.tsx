// What a running Autopilot is choosing from and the picks it set aside, drawn in the setup beside the
// boxes it was started with: Next up, and Set aside with Run it and Dismiss on each. A pick run goes to the
// approval gate as any dispatch does. #2092.

import { useEffect, useState } from "react";
import type { Autopilot, AutopilotPick, IssueList } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../../../shared/bridge";
import { nextUpOf, setAsideOf } from "../autopilot";
import { AutopilotSidings, type SidingPress } from "../AutopilotSidings";
import { fold } from "./autopilots";
import "../autopilot.css";

/** The open issues of each Autopilot that is on, read again as its picks change. */
function useIssues(on: readonly Autopilot[]): ReadonlyMap<string, IssueList> {
  const [lists, setLists] = useState<ReadonlyMap<string, IssueList>>(new Map());
  const asked = on.map((one) => `${one.manifest_id}:${one.picks.length}`).join(" ");
  useEffect(() => {
    if (typeof window.armada?.listIssues !== "function") return;
    let current = true;
    for (const one of on) {
      void window.armada.listIssues(one.manifest_id, false).then((read) => current && read.ok && setLists((was) => new Map(was).set(one.manifest_id, read.value)));
    }
    return () => {
      current = false;
    };
  }, [asked]);
  return lists;
}

/** The lists of each Autopilot that is on. A refusal is said by `onRefused`. */
export function AutopilotPicks({ on, state, onRefused }: { on: readonly Autopilot[]; state: BridgeState; onRefused: (why: string) => void }) {
  const issues = useIssues(on);
  const [pending, setPending] = useState<SidingPress>();
  const nameOf = (manifest: string) => state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest;

  const press = async (manifest: string, pick: AutopilotPick, which: "run" | "dismiss") => {
    setPending({ pickId: pick.pick_id, act: which });
    const send = which === "run" ? window.armada.dispatchSetAside : window.armada.dismissSetAside;
    const answer = await send(manifest, pick.pick_id);
    setPending(undefined);
    if (answer.ok) fold(answer.value);
    else onRefused(refusalWords(answer.outcome));
  };

  const sidings = on.map((autopilot) => ({
    manifest: autopilot.manifest_id,
    name: on.length > 1 ? nameOf(autopilot.manifest_id) : undefined,
    nextUp: nextUpOf(autopilot, issues.get(autopilot.manifest_id)),
    asides: setAsideOf(autopilot),
  }));
  return <AutopilotSidings sidings={sidings} pending={pending} onRun={(manifest, pick) => void press(manifest, pick, "run")} onDismiss={(manifest, pick) => void press(manifest, pick, "dismiss")} />;
}
