// Where a Job's own shell opens, off `terminal.openJobTerminalIn`: docked (in its own panel under the Job in the Cockpit, under the thread elsewhere), or in the side panel.

import { useEffect, useState } from "react";
import type { SettingsList } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { stateOf } from "./state-feed";

export type JobTerminalPlace = "docked" | "side panel";

const KEY = "terminal.openJobTerminalIn";

/** `side panel` only where Fleet has said so; unread, absent or any other value is docked. */
export function jobTerminalPlaceOf(settings: SettingsList | null | undefined): JobTerminalPlace {
  return settings?.settings.find((one) => one.key === KEY)?.value === "side panel" ? "side panel" : "docked";
}

/** Saves the place, as Settings does: the terminal's own toggle, which moves the shell there as well. */
export function saveJobTerminalPlace(place: JobTerminalPlace): void {
  const api = window.armada as Partial<BridgeApi> | undefined;
  // A test's fake window with no settings to save into.
  if (typeof api?.saveSettings === "function") void api.saveSettings({ [KEY]: place });
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
