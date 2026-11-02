// A Job's thread on a Mac that has not allowed the microphone: Talk ends at once and the box says what to turn on. The
// `livelier-thread` scenario with nothing for Talk to hear.

import type { Scenario } from "../moment";
import { s341LivelierThread } from "./livelier-thread";

export const s342TalkRefused: Scenario = {
  ...s341LivelierThread,
  name: "talk-refused",
  says: "Talk refused by macOS: the guide over the box names the microphone and opens its Settings pane",
  draft: { ...s341LivelierThread.draft, threadLook: { ...s341LivelierThread.draft!.threadLook!, heard: "" } },
};
