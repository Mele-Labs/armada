// What Talk's guide says for each reason hearing ended early: a title, the steps in order where the owner has any, and the Settings pane that holds the fix.

import type { TalkPane, TalkWhy } from "../ThreadSettings/thread-look";

/** `**bold**` marks what to find on screen. Only a refused permission has steps and a pane; the rest is a title. */
export type TalkGuide = {
  title: string;
  steps: readonly string[];
  pane?: TalkPane;
  action?: string;
};

const TRY_AGAIN = "Press Talk again.";

export function talkGuide(why: TalkWhy): TalkGuide {
  switch (why) {
    case "microphone_denied":
      return {
        title: "The microphone is off for Armada Voice",
        steps: [
          "Open **Privacy & Security**, then **Microphone**.",
          "Turn on **Armada Voice**.",
          TRY_AGAIN,
        ],
        pane: "microphone",
        action: "Open Microphone settings",
      };
    case "speech_denied":
      return {
        title: "Speech recognition is off for Armada Voice",
        steps: [
          "Open **Privacy & Security**, then **Speech Recognition**.",
          "Turn on **Armada Voice**.",
          TRY_AGAIN,
        ],
        pane: "speech",
        action: "Open Speech Recognition settings",
      };
    case "microphone_unavailable":
      return {
        title: "No microphone is available",
        steps: [],
      };
    default:
      return {
        title: "Talk stopped",
        steps: [],
      };
  }
}
