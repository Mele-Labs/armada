// The video script Job whose page carries no review mark: Fleet keeps revision 2 for reading, so its card has Open and no
// Approve, and nothing waits in Needs you.

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";

export const documentForReading: Scenario = videoScriptScenario(
  "document-for-reading",
  "The video script Job with revision 2 kept for reading: no Approve, nothing waiting on him",
  "reading",
);
