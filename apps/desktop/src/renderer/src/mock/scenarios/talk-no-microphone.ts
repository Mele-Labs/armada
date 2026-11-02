// A Job's thread on a Mac with no microphone: Talk ends at once and the box says so. The `talk-refused` scenario with a
// different reason.

import type { Scenario } from "../moment";
import { s342TalkRefused } from "./talk-refused";

export const s342TalkNoMicrophone: Scenario = {
  ...s342TalkRefused,
  name: "talk-no-microphone",
  says: "Talk with no microphone: the guide over the box says none is available",
  draft: {
    ...s342TalkRefused.draft,
    threadLook: {
      ...s342TalkRefused.draft!.threadLook!,
      refusal: "microphone_unavailable",
    },
  },
};
