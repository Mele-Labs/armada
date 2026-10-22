// The side panel in a window Fleet serves: its reads over the preload (`fleet-reads.ts`) and its host beside the app.
// **The window's state is followed here and nowhere above it**, so a state that moves draws only the panel again.

import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { MergeLines } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import { openLink } from "../commands";
import { pullRequestPlaceOf } from "../pull-request-place";
import { withState } from "../state-feed";
import type { StateSource } from "../state-feed";
import { fleetPanelReads } from "./fleet-reads";
import type { FleetPanelApi } from "./fleet-reads";
import { SharedPanelHost } from "./host";
import { SharedPanelFrom } from "./reads";
import type { PanelReads } from "./reads";

type Api = FleetPanelApi & StateSource;

/** The pull requests a `#number` in the panel can name: the ones the merge lines hold open. */
const numberedOf = (lines: MergeLines | null): PanelReads["numbered"] =>
  (lines?.lines ?? []).flatMap((one) => (one.hub?.pull_requests ?? []).map((pr) => ({ kind: "pull_request" as const, number: String(pr.number), title: pr.title, url: pr.url })));

export function SharedPanelOver({ api, children, testsReaching }: { api: Api; children: ReactNode; testsReaching?: PanelReads["testsReaching"] | undefined }) {
  const now = useRef<BridgeState | undefined>(undefined);
  const [lines, setLines] = useState<MergeLines | null>(null);
  const [browser, setBrowser] = useState(false);
  const [followed, setFollowed] = useState(0);
  useEffect(() => {
    let live = true;
    const take = (state: BridgeState) => {
      if (now.current?.landFollowed !== state.landFollowed) setFollowed((one) => one + 1);
      now.current = state;
      setLines(state.mergeLines);
      setBrowser(pullRequestPlaceOf(state.settings) === "browser");
    };
    void api.state().then((state) => live && take(state));
    const off = api.subscribe(take);
    return () => {
      live = false;
      off();
    };
  }, [api]);
  const base = useMemo(() => fleetPanelReads(api, () => now.current), [api]);
  const reads = useMemo(
    (): PanelReads => ({ ...base, lines, numbered: numberedOf(lines), followed, ...(browser ? { browse: (address: string) => (void openLink(address), true) } : {}), ...(testsReaching === undefined ? {} : { testsReaching }) }),
    [base, lines, browser, followed, testsReaching],
  );
  return (
    <SharedPanelFrom reads={reads}>
      {children}
      <SharedPanelHost />
    </SharedPanelFrom>
  );
}

/** The real window's: over `window.armada`. */
export function WiredSharedPanel({ children }: { children: ReactNode }) {
  const api = useMemo(() => withState(window.armada as BridgeApi) as unknown as Api, []);
  return <SharedPanelOver api={api}>{children}</SharedPanelOver>;
}
