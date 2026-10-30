// The video script Job after he approved revision 1 and the Job wrote revision 2: the first card reads Approved, and the newest
// asks again, back in Needs you and in the spine's Now.

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";

export const documentAsksAgain: Scenario = videoScriptScenario("document-asks-again", "The video script Job with revision 1 approved and revision 2 waiting on his Approve", 2);
