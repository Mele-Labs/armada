// A Job that took another's place and only read its page. The page came with it at the revision he answered, so nothing asks: the card has no
// Approve or Send back and the Job's row does not need him. The walk `document-handed-on` plays it.

import type { Scenario } from "../moment";
import { videoScriptScenario } from "../video-script-job";

export const documentHandedOn: Scenario = videoScriptScenario("document-handed-on", "A Job that took another's place, holding the page he approved", "handed_on");
