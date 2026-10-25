// Create mod: an empty Mod Job, started at once and opened on the Cockpit with the caret in its box. The palette's row and the Mods page's button
// both call this. What a Mod Job can make is listed in its empty thread, so nothing is asked first.

import { startEmptyModJob } from "./mod-job";

export function askToCreateMod(): void {
  void startEmptyModJob();
}
