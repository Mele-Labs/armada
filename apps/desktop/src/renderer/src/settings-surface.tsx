// What the window reads off settings.json itself: whether a guide opens the first time a piece is met, and this
// machine's name. Apart from `App.tsx`, which is at its length.

import type { GuidanceProviderProps } from "@armada/components";
import type { SettingsList } from "@armada/protocol";

/** `GuidanceProvider.firstContact` off settings.json, or `undefined` until Fleet has answered with it. */
export function firstContactOf(settings: SettingsList | null): GuidanceProviderProps["firstContact"] {
  const setting = settings?.settings.find((one) => one.key === "features.openGuidesFirstTime");
  if (setting === undefined) return undefined;
  return {
    off: setting.value === false,
    saved: setting.saved !== null,
    onOff: (off) => void window.armada.saveSettings({ "features.openGuidesFirstTime": !off }),
  };
}

/** This machine's name as the person set it, or `undefined` while it is blank and Fleet names it by the host. */
export function machineOf(settings: SettingsList | null): string | undefined {
  const value = settings?.settings.find((one) => one.key === "machine.name")?.value;
  return typeof value === "string" && value !== "" ? value : undefined;
}
