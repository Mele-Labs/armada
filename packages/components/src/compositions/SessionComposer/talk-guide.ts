// What Talk's guide says for each reason hearing ended early: a title, the steps in order, and the Settings pane that holds the fix.

import type { TalkPane, TalkWhy } from "../ThreadSettings/thread-look";

export type TalkGuide = { title: string; steps: readonly string[]; pane?: TalkPane; action?: string };

const TRY_AGAIN = "Press Talk again.";
const OTHER_VOICE = "Or pick Whisper or macOS Dictation under System, Thread, Voice.";

export function talkGuide(why: TalkWhy): TalkGuide {
  switch (why) {
    case "microphone_denied":
      return { title: "The microphone is off for Armada Voice", steps: ["Open Privacy & Security, Microphone.", "Turn on Armada Voice.", TRY_AGAIN], pane: "microphone", action: "Open Microphone settings" };
    case "speech_denied":
      return { title: "Speech recognition is off for Armada Voice", steps: ["Open Privacy & Security, Speech Recognition.", "Turn on Armada Voice.", TRY_AGAIN], pane: "speech", action: "Open Speech Recognition settings" };
    case "microphone_unavailable":
      return { title: "No microphone is available", steps: ["Plug one in, or choose an input in Sound, Input.", TRY_AGAIN], pane: "sound", action: "Open Sound settings" };
    case "on_device_unavailable":
      return { title: "This Mac cannot recognise its language on the device", steps: [OTHER_VOICE] };
    case "helper_missing":
      return { title: "Armada Voice is not built into this Armada", steps: ["Run `xcode-select --install` if the Xcode tools are missing.", "Run `node native/voice/build.mjs` in apps/desktop.", "Restart Armada."] };
    case "whisper_absent":
      return { title: "Whisper is not installed", steps: ["Install it under System, Thread, Voice.", TRY_AGAIN] };
    case "whisper_failed":
      return { title: "Whisper could not read the recording", steps: [TRY_AGAIN, OTHER_VOICE] };
    case "recognition_failed":
      return { title: "Speech recognition stopped", steps: [TRY_AGAIN, OTHER_VOICE] };
    default:
      return { title: "Talk stopped", steps: [TRY_AGAIN, OTHER_VOICE] };
  }
}
