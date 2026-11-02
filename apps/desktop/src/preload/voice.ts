// Talk's members of the preload surface (`main/voice-channels.ts`): hear with an engine and get words back, and read
// or install Whisper. The renderer names an engine, never a path, a process or a download.

import type { IpcRenderer } from "electron";

import type { Heard, VoiceApi, WhisperState } from "../shared/api/voice";
import { CHANNELS } from "../shared/bridge";

export function voicePreload(
  ipc: Pick<IpcRenderer, "invoke" | "on" | "removeListener">,
): Required<VoiceApi> {
  let hearings = 0;
  return {
    voice: {
      listen: (engine, onWord, onEnd) => {
        hearings += 1;
        const id = hearings;
        const handler = (_event: unknown, heard: Heard): void => {
          if (heard.id !== id) return;
          if ("word" in heard) return onWord(heard.word);
          ipc.removeListener(CHANNELS.voiceHeard, handler);
          onEnd?.(heard.why);
        };
        ipc.on(CHANNELS.voiceHeard, handler);
        void ipc.invoke(CHANNELS.voiceListen, id, engine).then((started: boolean) => {
          if (!started) ipc.removeListener(CHANNELS.voiceHeard, handler);
        });
        return () => void ipc.invoke(CHANNELS.voiceStop, id);
      },
      whisper: () => ipc.invoke(CHANNELS.voiceWhisper),
      onWhisper: (listener) => {
        const handler = (_event: unknown, state: WhisperState): void => listener(state);
        ipc.on(CHANNELS.voiceWhisperChanged, handler);
        return () => void ipc.removeListener(CHANNELS.voiceWhisperChanged, handler);
      },
      installWhisper: () => ipc.invoke(CHANNELS.voiceInstallWhisper),
    },
  };
}
