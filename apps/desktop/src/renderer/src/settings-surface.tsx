// The Settings surface as the window draws it, and the one setting the window reads itself: whether a
// guide opens the first time a piece is met. Apart from `App.tsx`, which is at its length.

import type { GuidanceProviderProps } from "@armada/components";
import type { SettingsList } from "@armada/protocol";
import { BridgeSettings } from "@armada/settings";
import { Boundary, type BoundaryProps } from "@armada/shell";

import { openInPanel } from "./shared-panel/store";

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

export type SettingsSurfaceProps = {
  settings: SettingsList | null;
  live: boolean;
  guarded: Pick<BoundaryProps, "bridge" | "onCopied">;
  onReadGuides: () => void;
  /** Opens the account, where the window keeps it. */
  onOpenAccount?: () => void;
};

/** settings.json's page under its own boundary. Saves and the file's opening go through main. */
export function SettingsSurface({ settings, live, guarded, onReadGuides, onOpenAccount }: SettingsSurfaceProps) {
  return (
    <Boundary region="Settings" {...guarded}>
      <BridgeSettings
        settings={settings}
        live={live}
        onSaveSettings={(changes) => window.armada.saveSettings(changes)}
        // The file opens in the side panel where one is drawn, and in the machine's editor where none is.
        onOpenSettingsFile={async () => (settings?.path !== undefined && openInPanel({ kind: "file", path: settings.path, from: "settings" }) ? { ok: true } : window.armada.openSettingsFile())}
        onReadGuides={onReadGuides}
        {...(onOpenAccount === undefined ? {} : { onOpenAccount })}
        {...(guarded.onCopied === undefined ? {} : { onCopied: guarded.onCopied })}
      />
    </Boundary>
  );
}
