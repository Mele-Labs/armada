// Where a Job's own shell opens, off `terminal.openJobTerminalIn`: docked under the thread, or in the side panel.

import { useEffect, useState } from "react";
import type { SettingsList } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { stateOf } from "./state-feed";

export type JobTerminalPlace = "docked" | "side panel";

/** `side panel` only where Fleet has said so; unread, absent or any other value is docked. */
export function jobTerminalPlaceOf(settings: SettingsList | null | undefined): JobTerminalPlace {
  return settings?.settings.find((one) => one.key === "terminal.openJobTerminalIn")?.value === "side panel" ? "side panel" : "docked";
}

/** The place in force, followed from the window's state while `on`; docked until Fleet has said otherwise. */
export function useJobTerminalPlace(on: boolean): JobTerminalPlace {
  const [place, setPlace] = useState<JobTerminalPlace>("docked");
  useEffect(() => {
    const api = window.armada as Partial<BridgeApi> | undefined;
    // A test's fake window with no state to follow.
    if (!on || typeof api?.syncState !== "function" || typeof api.subscribeChanges !== "function") return;
    const source = stateOf(api as BridgeApi);
    let live = true;
    const take = (state: BridgeState) => setPlace(jobTerminalPlaceOf(state.settings));
    void source.state().then((state) => live && take(state));
    const off = source.subscribe(take);
    return () => {
      live = false;
      off();
    };
  }, [on]);
  return place;
}
