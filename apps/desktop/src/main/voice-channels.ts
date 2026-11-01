// The channels Talk travels over. The window names an engine and a number of its own for each hearing, and gets that
// number's words back; a window that closes stops what it was hearing.

import type { IpcMainInvokeEvent } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { Heard, VoiceEngine } from "../shared/api/voice";
import { hear } from "./voice";
import type { VoiceHosts } from "./voice";
import type { WhisperInstall } from "./whisper-install";

const ENGINES: readonly unknown[] = ["apple", "whisper", "dictation"] satisfies VoiceEngine[];

type Sender = IpcMainInvokeEvent["sender"];
type Handle = (channel: string, listener: (event: { sender: Sender }, ...args: unknown[]) => unknown) => void;

type Hosts = { ipc: { handle: Handle }; voice: Omit<VoiceHosts, "whisper">; install: WhisperInstall };

export function handleVoice({ ipc, voice, install }: Hosts): void {
  const hearing = new Map<Sender, Map<number, () => void>>();
  const heldBy = (sender: Sender): Map<number, () => void> => {
    let held = hearing.get(sender);
    if (held === undefined) {
      const fresh = new Map<number, () => void>();
      hearing.set(sender, fresh);
      sender.once("destroyed", () => {
        hearing.delete(sender);
        fresh.forEach((stop) => stop());
      });
      held = fresh;
    }
    return held;
  };

  ipc.handle(CHANNELS.voiceListen, ({ sender }, id, engine) => {
    if (typeof id !== "number" || !Number.isInteger(id) || !ENGINES.includes(engine)) return false;
    const held = heldBy(sender);
    held.get(id)?.();
    const send = (heard: Heard): void => {
      if (!sender.isDestroyed()) sender.send(CHANNELS.voiceHeard, heard);
    };
    let ended = false;
    const stop = hear(engine as VoiceEngine, { ...voice, whisper: () => install.paths() }, (word) => send({ id, word }), (why) => {
      ended = true;
      held.delete(id);
      send(why === undefined ? { id, end: true } : { id, end: true, why });
    });
    if (!ended) held.set(id, stop);
    return true;
  });
  ipc.handle(CHANNELS.voiceStop, ({ sender }, id) => {
    if (typeof id === "number") hearing.get(sender)?.get(id)?.();
  });
  ipc.handle(CHANNELS.voiceWhisper, () => install.current);
  ipc.handle(CHANNELS.voiceInstallWhisper, () => install.install());
}
