// Talk's main side against a fake Armada Voice: the lines it prints read into words, and each engine started the way it
// is heard. The microphone, the recogniser and macOS's prompts are the helper's, and no test here reaches them.

import { describe, expect, it } from "vitest";

import type { VoiceWhy } from "../shared/api/voice";
import { hear, helperPath, readHelperLine, RELEASE_GRACE_MS, Settled, whyOf } from "./voice";
import type { VoiceChild, VoiceHosts } from "./voice";

type Fake = VoiceChild & {
  args: readonly string[];
  print: (text: string) => void;
  exit: (code: number | null) => void;
  stdinEnded: boolean;
  killed: boolean;
};

function setUp(over: Partial<VoiceHosts> = {}) {
  const started: Fake[] = [];
  const dictated: string[] = [];
  const timers: {
    ms: number;
    run: () => void;
    cancelled: boolean;
  }[] = [];
  const hosts: VoiceHosts = {
    helper: "/Armada Voice.app/Contents/MacOS/armada-voice",
    whisper: () => ({
      library: "/w/whisper",
      model: "/w/model.bin",
    }),
    spawn: (_file, args) => {
      const data: ((chunk: string) => void)[] = [];
      const exits: ((code: number | null) => void)[] = [];
      const fake: Fake = {
        args,
        stdinEnded: false,
        killed: false,
        stdout: {
          on: (_event, listener) => data.push(listener),
        },
        stdin: {
          end: () => (fake.stdinEnded = true),
          on: () => undefined,
        },
        on: (event: "exit" | "error", listener: never) =>
          event === "exit" ? exits.push(listener) : undefined,
        kill: () => (fake.killed = true),
        print: (text) => data.forEach((listener) => listener(text)),
        exit: (code) => exits.forEach((listener) => listener(code)),
      };
      started.push(fake);
      return fake;
    },
    dictate: (action) => dictated.push(action),
    after: (ms, run) => {
      const timer = {
        ms,
        run,
        cancelled: false,
      };
      timers.push(timer);
      return () => (timer.cancelled = true);
    },
    ...over,
  };
  const words: string[] = [];
  const ends: (VoiceWhy | undefined)[] = [];
  const start = (engine: "apple" | "whisper" | "dictation") =>
    hear(
      engine,
      hosts,
      (word) => words.push(word),
      (why) => ends.push(why),
    );
  return {
    started,
    dictated,
    timers,
    words,
    ends,
    start,
  };
}

const line = (value: object): string => `${JSON.stringify(value)}\n`;

describe("a line from Armada Voice", () => {
  it("reads each kind it prints, and nothing else", () => {
    expect(readHelperLine('{"kind":"partial","text":"run the"}')).toEqual({
      kind: "partial",
      text: "run the",
    });
    expect(readHelperLine('{"kind":"final","text":"run the tests"}')).toEqual({
      kind: "final",
      text: "run the tests",
    });
    expect(readHelperLine('{"kind":"error","code":"speech_denied","message":"denied"}')).toEqual({
      kind: "error",
      code: "speech_denied",
    });
    expect(readHelperLine('{"kind":"ready"}')).toEqual({
      kind: "ready",
    });
    expect(readHelperLine("ggml_metal_init: found device")).toBeUndefined();
    expect(readHelperLine('{"kind":"partial"}')).toBeUndefined();
    expect(readHelperLine('{"kind":"partial","text":')).toBeUndefined();
  });

  it("names the helper's own failures as the renderer reads them", () => {
    expect(whyOf("microphone_denied")).toBe("microphone_denied");
    expect(whyOf("whisper_model_unloadable")).toBe("whisper_failed");
    expect(whyOf("respawn_failed")).toBe("helper_failed");
  });
});

describe("words settled out of the recogniser's running guesses", () => {
  it("gives a word once two guesses in a row agree on it, and the final gives the rest", () => {
    const settled = new Settled();
    expect(settled.partial("Run")).toEqual([]);
    expect(settled.partial("Run the")).toEqual([
      "Run",
    ]);
    expect(settled.partial("Run the test")).toEqual([
      "the",
    ]);
    expect(settled.partial("Run the tests again")).toEqual([]);
    expect(settled.partial("Run the tests again please")).toEqual([
      "tests",
      "again",
    ]);
    expect(settled.final("Run the tests again please.")).toEqual([
      "please.",
    ]);
  });

  it("never gives a word twice when the guess is rewritten behind it", () => {
    const settled = new Settled();
    settled.partial("I scream");
    expect(settled.partial("I scream for")).toEqual([
      "I",
      "scream",
    ]);
    expect(settled.partial("ice cream for you")).toEqual([]);
    expect(settled.final("ice cream for you")).toEqual([
      "for",
      "you",
    ]);
  });
});

describe("hearing, by engine", () => {
  it("Apple on-device runs the helper's recogniser and gives words while he talks", () => {
    const { started, words, ends, start } = setUp();
    start("apple");
    expect(started[0]?.args).toEqual([
      "listen",
    ]);
    started[0]!.print(
      line({
        kind: "ready",
      }) +
        line({
          kind: "partial",
          text: "please",
        }),
    );
    // A line split across two reads is read whole.
    started[0]!.print('{"kind":"partial","te');
    started[0]!.print('xt":"please look"}\n');
    expect(words).toEqual([
      "please",
    ]);
    started[0]!.print(
      line({
        kind: "final",
        text: "please look again",
      }),
    );
    expect(words).toEqual([
      "please",
      "look",
      "again",
    ]);
    expect(ends).toEqual([
      undefined,
    ]);
  });

  it("Whisper records until the release and gives every word after it", () => {
    const { started, words, ends, timers, start } = setUp();
    const stop = start("whisper");
    expect(started[0]?.args).toEqual([
      "whisper",
      "/w/whisper",
      "/w/model.bin",
    ]);
    stop();
    expect(started[0]?.stdinEnded).toBe(true);
    expect(timers[0]?.ms).toBe(RELEASE_GRACE_MS.whisper);
    started[0]!.print(
      line({
        kind: "transcribing",
      }) +
        line({
          kind: "final",
          text: "ship it",
        }),
    );
    expect(words).toEqual([
      "ship",
      "it",
    ]);
    expect(ends).toEqual([
      undefined,
    ]);
    expect(timers[0]?.cancelled).toBe(true);
  });

  it("Whisper not installed starts nothing and says so", () => {
    const { started, ends, start } = setUp({
      whisper: () => null,
    });
    start("whisper");
    expect(started).toEqual([]);
    expect(ends).toEqual([
      "whisper_absent",
    ]);
  });

  it("macOS Dictation starts and stops the system's own, and gives no words itself", () => {
    const { started, dictated, words, ends, start } = setUp();
    const stop = start("dictation");
    expect(dictated).toEqual([
      "startDictation:",
    ]);
    stop();
    stop();
    expect(dictated).toEqual([
      "startDictation:",
      "stopDictation:",
    ]);
    expect(started).toEqual([]);
    expect(words).toEqual([]);
    expect(ends).toEqual([
      undefined,
    ]);
  });

  it("a Bridge built without the helper says so rather than hearing nothing", () => {
    const { ends, start } = setUp({
      helper: null,
    });
    start("apple");
    expect(ends).toEqual([
      "helper_missing",
    ]);
  });

  it("ends early on the helper's error, and on its dying unreleased", () => {
    const denied = setUp();
    denied.start("apple");
    denied.started[0]!.print(
      line({
        kind: "error",
        code: "speech_denied",
        message: "",
      }),
    );
    expect(denied.ends).toEqual([
      "speech_denied",
    ]);

    const died = setUp();
    died.start("apple");
    died.started[0]!.exit(null);
    expect(died.ends).toEqual([
      "helper_failed",
    ]);
  });

  it("a released helper that never answers is killed, and what was heard stands", () => {
    const { started, words, ends, timers, start } = setUp();
    const stop = start("apple");
    started[0]!.print(
      line({
        kind: "partial",
        text: "one",
      }) +
        line({
          kind: "partial",
          text: "one two",
        }),
    );
    stop();
    timers[0]!.run();
    expect(started[0]?.killed).toBe(true);
    expect(words).toEqual([
      "one",
    ]);
    expect(ends).toEqual([
      undefined,
    ]);
  });
});

it("finds the helper beside its sources in development and in Contents/Helpers packaged", () => {
  expect(helperPath(false, "/repo/apps/desktop", "/unused")).toBe(
    "/repo/apps/desktop/native/voice/build/Armada Voice.app/Contents/MacOS/armada-voice",
  );
  expect(helperPath(true, "/unused", "/Applications/Armada.app/Contents/Resources")).toBe(
    "/Applications/Armada.app/Contents/Helpers/Armada Voice.app/Contents/MacOS/armada-voice",
  );
});
