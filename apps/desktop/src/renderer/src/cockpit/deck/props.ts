// What the Cockpit window hands the deck: the Bridge state, the clock, the picked repository, the Now views and every act the
// deck can press. One type, read by every pane, so a pane asks for the props and not for the component. #2301.

import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";

import type { BridgeState } from "../../../../shared/bridge";
import type { Hosts } from "../../Dashboard";

export type CockpitProps = Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  nows?: Readonly<Record<string, NowView>> | undefined;
  /** A call an alert asked for, brought to the front once this is on screen. `at` makes a second press land. */
  bring?: { key: string; at: number } | null | undefined;
  /** Taken, so the token is not acted on twice. */
  onBrought?: (() => void) | undefined;
};

/**
 * How long an answered call is held out of the deck if Fleet still carries it. **Fleet answers the POST before it has re-derived
 * what a Session waits on or whether a Job asks**, so the next update can carry the call again; a call is held gone until an update
 * stops carrying it, and this is the longest that holds before the call is allowed back. An object so a test can shorten it.
 */
export const holdFor = { ms: 30_000 };
