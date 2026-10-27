// Settings, Bridge, Spacing (`spacing.density`): `data-spacing` on the root picks Compact's or Roomy's token set
// (`packages/tokens/src/spacing-sets.css`), and Default is none. Apart from `App.tsx`, which is at its length.

import { useEffect } from "react";
import type { SettingsList } from "@armada/protocol";

export type Spacing = "compact" | "default" | "roomy";

/** Compact or Roomy only where Fleet has said so; unread, absent or any other value is Default. */
export function spacingOf(settings: SettingsList | null | undefined): Spacing {
  const value = settings?.settings.find((one) => one.key === "spacing.density")?.value;
  return value === "compact" || value === "roomy" ? value : "default";
}

/** Keeps the root's `data-spacing` on the spacing in force. */
export function useSpacing(settings: SettingsList | null | undefined): void {
  const spacing = spacingOf(settings);
  useEffect(() => {
    const root = document.documentElement.dataset;
    if (spacing === "default") delete root["spacing"];
    else root["spacing"] = spacing;
  }, [spacing]);
}
