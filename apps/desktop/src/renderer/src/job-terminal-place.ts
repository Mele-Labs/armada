// Where a Job's own shell opens, off `terminal.openJobTerminalIn`: docked under the thread, or in the side panel.

import type { SettingsList } from "@armada/protocol";

export type JobTerminalPlace = "docked" | "side panel";

/** `side panel` only where Fleet has said so; unread, absent or any other value is docked. */
export function jobTerminalPlaceOf(settings: SettingsList | null | undefined): JobTerminalPlace {
  return settings?.settings.find((one) => one.key === "terminal.openJobTerminalIn")?.value === "side panel" ? "side panel" : "docked";
}
