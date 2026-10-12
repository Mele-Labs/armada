// The Settings surface as the window draws it, and the one setting the window reads itself: whether a
// guide opens the first time a piece is met. Apart from `App.tsx`, which is at its length.

import type { GuidanceProviderProps } from "@armada/components";
import type { SettingsList } from "@armada/protocol";
import { BridgeSettings } from "@armada/settings";
import { Boundary, type BoundaryProps } from "@armada/shell";

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

export type SettingsSurfaceProps = {
  settings: SettingsList | null;
  live: boolean;
  guarded: Pick<BoundaryProps, "bridge" | "onCopied">;
  onReadGuides: () => void;
};

/** settings.json's page under its own boundary. Saves and the file's opening go through main. */
export function SettingsSurface({ settings, live, guarded, onReadGuides }: SettingsSurfaceProps) {
  return (
    <Boundary region="Settings" {...guarded}>
      <BridgeSettings
        settings={settings}
        live={live}
        onSaveSettings={(changes) => window.armada.saveSettings(changes)}
        onOpenSettingsFile={() => window.armada.openSettingsFile()}
        onReadGuides={onReadGuides}
        {...(guarded.onCopied === undefined ? {} : { onCopied: guarded.onCopied })}
      />
    </Boundary>
  );
}
