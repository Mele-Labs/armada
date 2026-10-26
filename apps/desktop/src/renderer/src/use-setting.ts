// One setting's value in force, followed from the window's state, and saved back by key. `job-terminal-place.ts`' reading, for a key a component asks for by name.

import { useEffect, useState } from "react";
import type { Outcome, SettingValue } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { stateOf } from "./state-feed";

/** The value in force for `key`, or `undefined` until Fleet has answered or where it has no such key. */
export function useSetting(key: string): SettingValue | undefined {
  const [value, setValue] = useState<SettingValue | undefined>();
  useEffect(() => {
    const api = window.armada as Partial<BridgeApi> | undefined;
    // A test's fake window with no state to follow.
    if (typeof api?.syncState !== "function" || typeof api.subscribeChanges !== "function") return;
    const source = stateOf(api as BridgeApi);
    let live = true;
    const take = (state: BridgeState) => setValue(state.settings?.settings.find((one) => one.key === key)?.value);
    void source.state().then((state) => live && take(state));
    const off = source.subscribe(take);
    return () => {
      live = false;
      off();
    };
  }, [key]);
  return value;
}

/** Save one key, or say it could not be: a window with no settings to save into refuses. */
export async function saveSetting(key: string, value: SettingValue): Promise<Outcome> {
  const api = window.armada as Partial<BridgeApi> | undefined;
  if (typeof api?.saveSettings !== "function") return { ok: false, why: "not_connected" };
  return api.saveSettings({ [key]: value });
}
