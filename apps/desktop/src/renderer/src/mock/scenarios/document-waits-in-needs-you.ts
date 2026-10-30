// A Job that wrote a video script as its document, revision 1 not yet approved. The Job's row needs him, the script is a card in
// the thread with Approve and Send back, and it stands in the spine's Now until he approves it, then under Done. The walk
// `document-waits-in-needs-you` plays it.

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";

export const documentWaitsInNeedsYou: Scenario = videoScriptScenario("document-waits-in-needs-you", "A Job whose video script, revision 1, waits on his Approve", 1);
