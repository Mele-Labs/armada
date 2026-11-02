// Talk's preload members against a fake `ipcRenderer`: what each one may send main, and that a hearing's words reach
// only the listener that started it.

import { describe, expect, it } from "vitest";

import type { Heard, WhisperState } from "../shared/api/voice";
import { CHANNELS } from "../shared/bridge";
import { voicePreload } from "./voice";

type Listener = (event: unknown, value: never) => void;

function fakeIpc(answer: (channel: string) => unknown = () => true) {
  const invoked: unknown[][] = [];
  const listeners = new Map<string, Set<Listener>>();
  const ipc = {
    invoke: async (channel: string, ...args: unknown[]) => {
      invoked.push([
        channel,
        ...args,
      ]);
      return answer(channel);
    },
    on: (channel: string, listener: Listener) => {
      listeners.set(channel, (listeners.get(channel) ?? new Set()).add(listener));
      return ipc;
    },
    removeListener: (channel: string, listener: Listener) => {
      listeners.get(channel)?.delete(listener);
      return ipc;
    },
  };
  const send = (channel: string, value: unknown): void =>
    listeners.get(channel)?.forEach((listener) => listener({}, value as never));
  const count = (channel: string): number => listeners.get(channel)?.size ?? 0;
  return {
    ipc: ipc as unknown as Parameters<typeof voicePreload>[0],
    invoked,
    send,
    count,
  };
}

describe("Talk's preload surface", () => {
  it("is the four members and nothing else", () => {
    const { voice } = voicePreload(fakeIpc().ipc);
    expect(Object.keys(voice).sort()).toEqual([
      "installWhisper",
      "listen",
      "onWhisper",
      "whisper",
    ]);
  });

  it("names Talk's own channels only, with an engine and a hearing's number", async () => {
    const fake = fakeIpc((channel) =>
      channel === CHANNELS.voiceWhisper
        ? {
            state: "absent",
          }
        : true,
    );
    const { voice } = voicePreload(fake.ipc);
    const stop = voice.listen("apple", () => {});
    stop();
    expect(await voice.whisper()).toEqual({
      state: "absent",
    });
    await voice.installWhisper();
    expect(fake.invoked).toEqual([
      [
        CHANNELS.voiceListen,
        1,
        "apple",
      ],
      [
        CHANNELS.voiceStop,
        1,
      ],
      [
        CHANNELS.voiceWhisper,
      ],
      [
        CHANNELS.voiceInstallWhisper,
      ],
    ]);
  });

  it("gives each hearing its own words and end, and lets go of it once ended", () => {
    const fake = fakeIpc();
    const { voice } = voicePreload(fake.ipc);
    const first: string[] = [];
    const second: string[] = [];
    const ends: unknown[] = [];
    voice.listen(
      "apple",
      (word) => first.push(word),
      (why) => ends.push(why),
    );
    voice.listen("whisper", (word) => second.push(word));
    const heard = (value: Heard): void => fake.send(CHANNELS.voiceHeard, value);
    heard({
      id: 1,
      word: "run",
    });
    heard({
      id: 2,
      word: "ship",
    });
    heard({
      id: 1,
      end: true,
      why: "speech_denied",
    });
    heard({
      id: 1,
      word: "late",
    });
    expect(first).toEqual([
      "run",
    ]);
    expect(second).toEqual([
      "ship",
    ]);
    expect(ends).toEqual([
      "speech_denied",
    ]);
    expect(fake.count(CHANNELS.voiceHeard)).toBe(1);
  });

  it("lets go of a hearing main refused", async () => {
    const fake = fakeIpc(() => false);
    voicePreload(fake.ipc).voice.listen("apple", () => {});
    await Promise.resolve();
    await Promise.resolve();
    expect(fake.count(CHANNELS.voiceHeard)).toBe(0);
  });

  it("tells a listener each change to Whisper until it lets go", () => {
    const fake = fakeIpc();
    const told: WhisperState[] = [];
    const off = voicePreload(fake.ipc).voice.onWhisper((state) => told.push(state));
    fake.send(CHANNELS.voiceWhisperChanged, {
      state: "installing",
      progress: 0.5,
    });
    off();
    fake.send(CHANNELS.voiceWhisperChanged, {
      state: "installed",
    });
    expect(told).toEqual([
      {
        state: "installing",
        progress: 0.5,
      },
    ]);
  });
});
