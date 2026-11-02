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
  const installs: number[] = [];
  const timers: {
    ms: number;
    run: () => void;
    cancelled: boolean;
  }[] = [];
  const hosts: VoiceHosts = {
    helper: () => "/Armada Voice.app/Contents/MacOS/armada-voice",
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
    installWhisper: () => installs.push(1),
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
    installs,
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

const LISTEN = [
  "listen",
];
const WHISPER = [
  "whisper",
  "/w/whisper",
  "/w/model.bin",
];

describe("a voice that cannot hear", () => {
  it("Whisper chosen and not installed is fetched unasked, and Apple hears meanwhile with no card", () => {
    const { started, installs, ends, start } = setUp({
      whisper: () => null,
    });
    start("whisper");
    expect(installs).toHaveLength(1);
    expect(started.map((one) => one.args)).toEqual([
      LISTEN,
    ]);
    expect(ends).toEqual([]);
  });

  it("Whisper is never fetched because it is next in line behind Apple", () => {
    const { started, dictated, installs, ends, start } = setUp({
      whisper: () => null,
    });
    start("apple");
    started[0]!.exit(null);
    expect(installs).toEqual([]);
    expect(dictated).toEqual([
      "startDictation:",
    ]);
    expect(ends).toEqual([]);
  });

  it("with no helper, Apple and Whisper both go to Dictation with no card", () => {
    for (const engine of [
      "apple",
      "whisper",
    ] as const) {
      const { started, dictated, ends, start } = setUp({
        helper: () => null,
      });
      start(engine);
      expect(started).toEqual([]);
      expect(dictated).toEqual([
        "startDictation:",
      ]);
      expect(ends).toEqual([]);
    }
  });

  it("a helper built since the last press is found at the next", () => {
    let built = false;
    const { started, dictated, start } = setUp({
      helper: () => (built ? "/built/armada-voice" : null),
    });
    start("apple")();
    expect(started).toEqual([]);
    expect(dictated).toEqual([
      "startDictation:",
      "stopDictation:",
    ]);
    built = true;
    start("apple");
    expect(started.map((one) => one.args)).toEqual([
      LISTEN,
    ]);
  });

  it("moves through the voices in order, for what each chosen engine can fall to", () => {
    const apple = setUp();
    apple.start("apple");
    apple.started[0]!.print(
      line({
        kind: "error",
        code: "on_device_unavailable",
      }),
    );
    expect(apple.started.map((one) => one.args)).toEqual([
      LISTEN,
      WHISPER,
    ]);
    apple.started[1]!.print(
      line({
        kind: "error",
        code: "whisper_model_unloadable",
      }),
    );
    expect(apple.dictated).toEqual([
      "startDictation:",
    ]);
    expect(apple.ends).toEqual([]);

    const whisper = setUp();
    whisper.start("whisper");
    whisper.started[0]!.exit(null);
    expect(whisper.started.map((one) => one.args)).toEqual([
      WHISPER,
      LISTEN,
    ]);
    expect(whisper.ends).toEqual([]);
  });

  it("the release stops whichever voice took over, and ends the hearing once", () => {
    const { started, dictated, ends, start } = setUp({
      whisper: () => null,
    });
    const stop = start("apple");
    started[0]!.exit(null);
    stop();
    expect(dictated).toEqual([
      "startDictation:",
      "stopDictation:",
    ]);
    expect(ends).toEqual([
      undefined,
    ]);
  });

  it("a refused permission or no microphone ends it with the reason, and starts nothing else", () => {
    for (const code of [
      "microphone_denied",
      "speech_denied",
      "microphone_unavailable",
    ] as const) {
      const { started, dictated, ends, start } = setUp();
      start("apple");
      started[0]!.print(
        line({
          kind: "error",
          code,
        }),
      );
      expect(started).toHaveLength(1);
      expect(dictated).toEqual([]);
      expect(ends).toEqual([
        code,
      ]);
    }
  });

  it("failing after a word ends quietly: the next voice would miss what was said", () => {
    const { started, dictated, words, ends, start } = setUp();
    start("apple");
    started[0]!.print(
      line({
        kind: "partial",
        text: "one two",
      }) +
        line({
          kind: "partial",
          text: "one two three",
        }),
    );
    started[0]!.print(
      line({
        kind: "error",
        code: "recognition_failed",
      }),
    );
    expect(words).toEqual([
      "one",
      "two",
    ]);
    expect(started).toHaveLength(1);
    expect(dictated).toEqual([]);
    expect(ends).toEqual([
      undefined,
    ]);
  });

  it("failing after the release ends quietly: nobody is talking for the next voice to hear", () => {
    const { started, dictated, ends, start } = setUp();
    const stop = start("whisper");
    stop();
    started[0]!.print(
      line({
        kind: "error",
        code: "whisper_model_unloadable",
      }),
    );
    expect(started).toHaveLength(1);
    expect(dictated).toEqual([]);
    expect(ends).toEqual([
      undefined,
    ]);
  });
});

describe("where the helper is found", () => {
  const DEV = "/repo/apps/desktop/native/voice/build/Armada Voice.app/Contents/MacOS/armada-voice";

  it("beside its sources on Electron's own binary", () => {
    expect(helperPath(false, true, "/repo/apps/desktop", "/unused")).toBe(DEV);
  });

  it("beside its sources on the dev copy named Armada, which Electron reads as packaged", () => {
    expect(helperPath(true, true, "/repo/apps/desktop", "/unused")).toBe(DEV);
  });

  it("in Contents/Helpers in a real Armada", () => {
    expect(
      helperPath(true, undefined, "/unused", "/Applications/Armada.app/Contents/Resources"),
    ).toBe(
      "/Applications/Armada.app/Contents/Helpers/Armada Voice.app/Contents/MacOS/armada-voice",
    );
  });
});
