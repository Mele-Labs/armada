// System › Thread: how a Job's thread reads and the quick bar's lines, the same form the Aa panel in a Job's head draws, with
// Voice under it. Spacing is Bridge's own `spacing.density`, the setting System › Spacing writes.

import { useState } from "react";
import { DEFAULT_LOOK, ThreadSettings, useThreadLook } from "@armada/components";
import type { ThreadLook, ThreadSpacing } from "@armada/components";
import type { Outcome, SaveSettings, SettingsList } from "@armada/protocol";

const NO_MODEL = { state: "absent" } as const;

export function ThreadPane({ settings, onSave }: { settings: SettingsList | null; onSave: (changes: SaveSettings["changes"]) => Promise<Outcome> }) {
  const source = useThreadLook();
  // A window that holds no look still draws the form; what it sets is the pane's own until something reads it.
  const [own, setOwn] = useState<ThreadLook>(DEFAULT_LOOK);
  const density = settings?.settings.find((one) => one.key === "spacing.density")?.value;
  const spacing: ThreadSpacing = density === "compact" || density === "roomy" ? density : "default";
  return (
    <ThreadSettings
      look={source?.look ?? own}
      onChange={source?.setLook ?? setOwn}
      spacing={spacing}
      onSpacing={(next) => void onSave({ "spacing.density": next })}
      voice={{ whisper: source?.whisper ?? NO_MODEL, onInstall: source?.installWhisper ?? (() => undefined) }}
      keep="thread-look:new-line:system"
    />
  );
}
