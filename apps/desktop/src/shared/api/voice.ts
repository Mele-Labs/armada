// Talk: the words the owner says into a Job's message box, heard on this Mac (`main/voice.ts`), and Whisper's install
// in Bridge's own data folder (`main/whisper-install.ts`). Nothing here reaches Fleet. A slice imports protocol, never
// another slice; `../api.ts` and `../bridge.ts` compose them.

/** Where Talk's words come from. The same three as `VoiceEngine` in `@armada/components`' `thread-look.ts`. */
export type VoiceEngine = "apple" | "whisper" | "dictation";

/** Whisper on this Mac, shaped as `WhisperModel` in `thread-look.ts`: not here, coming down (`progress` 0 to 1), or here. */
export type WhisperState = { state: "absent" } | { state: "installing"; progress: number } | { state: "installed" };

/**
 * Why hearing ended early, or why an install did not finish. `helper_missing`: Armada Voice was not built into this
 * Bridge. `microphone_denied`, `speech_denied`: the owner said no in macOS's prompt, or in System Settings since.
 * `on_device_unavailable`: this Mac has no on-device recogniser for its language. `whisper_absent`: Install was never
 * pressed. `download_failed`, `digest_mismatch`, `unpack_failed`: Install stopped and Whisper is absent again.
 * `helper_failed`: anything else the helper said, or its dying without saying.
 */
export type VoiceWhy =
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

/** The System Settings panes Talk's guide opens. The same as `TalkPane` in `@armada/components`' `thread-look.ts`. */
export type VoicePane = "microphone" | "speech" | "sound";

export type VoiceApi = {
  /** **Absent in the mock**, whose look source plays Talk from a script. */
  voice?: {
    /**
     * Hears the owner with `engine` until the returned stop is called, and calls `onWord` with each word as it is
     * settled. Apple's words come while he talks; Whisper's all come after stop, once transcribed. Dictation calls
     * neither: macOS types into the box that has focus, so the box must have it before this is called. `onEnd` is
     * called once, when nothing more will come, with why where it ended early.
     */
    listen: (engine: VoiceEngine, onWord: (word: string) => void, onEnd?: (why?: VoiceWhy) => void) => () => void;
    /** Opens the System Settings pane that holds what Talk was refused: Microphone, Speech Recognition, or Sound's input list. */
    openSettings: (pane: VoicePane) => void;
    whisper: () => Promise<WhisperState>;
    onWhisper: (listener: (state: WhisperState) => void) => () => void;
    /** Downloads whisper.cpp and its model into Bridge's data folder. Resolves when it is installed or has given up. */
    installWhisper: () => Promise<{ ok: true } | { ok: false; why: VoiceWhy }>;
  };
};

/** One message on `voiceHeard`: a word for the hearing `id`, or its end. */
export type Heard = { id: number; word: string } | { id: number; end: true; why?: VoiceWhy };

export const VOICE_CHANNELS = {
  voiceListen: "bridge:voice-listen",
  voiceStop: "bridge:voice-stop",
  voiceHeard: "bridge:voice-heard",
  voiceWhisper: "bridge:voice-whisper",
  voiceWhisperChanged: "bridge:voice-whisper-changed",
  voiceInstallWhisper: "bridge:voice-install-whisper",
  voiceOpenSettings: "bridge:voice-open-settings",
} as const;
