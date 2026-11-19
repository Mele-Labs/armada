export type ThreadSpacing = "compact" | "default" | "roomy";

/** A line the quick bar offers: one he saved, or one Bridge learnt from what he sends. */
export type QuickPhrase = {
  id: string;
  text: string;
  kind:
    | "saved"
    | "learnt" /** Sits first in the bar, after the answers to the agent's question. Only a saved line is pinned. */;
  pinned?: boolean;
};

/** What he does to a quick reply, from its menu or from the settings list. */
export type QuickAct =
  | {
      act: "edit";
      id: string;
      text: string;
    }
  | {
      act: "delete";
      id: string;
    }
  | {
      act: "pin";
      id: string;
    }
  | {
      act: "unpin";
      id: string;
    }
  /** An answer to the agent's question, kept as a saved line. */
  | {
      act: "save";
      text: string;
    };

/** The lines as the bar draws them: pinned first, each group in the order it is kept. */
export function pinnedFirst<
  T extends {
    pinned?: boolean;
  },
>(lines: readonly T[]): T[] {
  return [
    ...lines.filter((one) => one.pinned === true),
    ...lines.filter((one) => one.pinned !== true),
  ];
}

/** The body sizes a thread can be read at, each a type token's (`typography.css`). `base` is what a message is drawn at today. */
export const THREAD_TEXT_SIZES = [
  {
    size: "xs",
    said: "13 px",
  },
  {
    size: "sm",
    said: "14 px",
  },
  {
    size: "base",
    said: "15 px",
  },
  {
    size: "lg",
    said: "18 px",
  },
] as const;

export type ThreadTextSize = (typeof THREAD_TEXT_SIZES)[number]["size"];

/** Why Talk ended early: the same names as `VoiceWhy` in the desktop app's `shared/api/voice.ts`. */
export type TalkWhy =
  | "helper_missing"
  | "helper_failed"
  | "microphone_denied"
  | "microphone_unavailable"
  | "speech_denied"
  | "on_device_unavailable"
  | "recognition_failed"
  | "whisper_absent"
  | "whisper_failed"
  | "download_failed"
  | "digest_mismatch"
  | "unpack_failed";

/** The System Settings panes Talk's guide can open. */
export type TalkPane = "microphone" | "speech";
