// Command Central: the cockpit. What is running is the viewscreen; what needs the owner comes forward
// over it as a call, and waits at the edge when put off. `cockpit/` holds the parts. Mock only, as the
// Dashboard is.

import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";

import type { BridgeState } from "../../shared/bridge";
import { Cockpit } from "./cockpit/Cockpit";
import type { Hosts } from "./Dashboard";

export function CommandCentral(props: Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  nows?: Readonly<Record<string, NowView>> | undefined;
}) {
  return <Cockpit {...props} />;
}
