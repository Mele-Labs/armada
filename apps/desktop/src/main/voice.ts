// Talk's three engines, heard in main. Apple's and Whisper's run Armada Voice (`native/voice/`), which prints one JSON
// object a line; Dictation is macOS's own, started in whatever box has focus. The renderer names an engine and gets
// words back, and never a path or a process.

import { join } from "node:path";

import type { VoiceEngine, VoiceWhy } from "../shared/api/voice";

/** One line Armada Voice prints. */
export type HelperLine =
  | { kind: "ready" }
  | { kind: "transcribing" }
  | { kind: "partial"; text: string }
  | { kind: "final"; text: string }
  | { kind: "error"; code: string };

/** A line read, or undefined for anything that is not one: a log line, a half-written object. */
export function readHelperLine(line: string): HelperLine | undefined {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch {
    return undefined;
  }
  if (typeof value !== "object" || value === null) return undefined;
  const { kind, text, code } = value as Record<string, unknown>;
  if (kind === "ready" || kind === "transcribing") return { kind };
  if ((kind === "partial" || kind === "final") && typeof text === "string") return { kind, text };
  if (kind === "error" && typeof code === "string") return { kind, code };
  return undefined;
}

const HELPER_WHYS: readonly VoiceWhy[] = ["microphone_denied", "microphone_unavailable", "speech_denied", "on_device_unavailable", "recognition_failed", "whisper_failed"];

/** The helper's error code as the renderer reads it; whisper.cpp's own failures are all `whisper_failed`. */
export function whyOf(code: string): VoiceWhy {
  if (code.startsWith("whisper_")) return "whisper_failed";
  return HELPER_WHYS.find((one) => one === code) ?? "helper_failed";
}

const wordsIn = (text: string): string[] => text.split(/\s+/).filter((one) => one !== "");

/**
 * The recogniser's running transcripts as words, each given once. It rewrites its guess as he talks, so a word is
 * given when two transcripts in a row agree on it and on everything before it, and the final transcript gives the
 * rest. A word given and rewritten after stays as given: the box has no way to take one back.
 */
export class Settled {
  private given = 0;
  private last: readonly string[] = [];

  partial(text: string): string[] {
    const words = wordsIn(text);
    let agreed = 0;
    while (agreed < words.length && agreed < this.last.length && words[agreed] === this.last[agreed]) agreed += 1;
    this.last = words;
    return this.upTo(words, agreed);
  }

  final(text: string): string[] {
    const words = wordsIn(text);
    return this.upTo(words, words.length);
  }

  private upTo(words: readonly string[], end: number): string[] {
    if (end <= this.given) return [];
    const out = words.slice(this.given, end);
    this.given = end;
    return out;
  }
}

/** What hearing needs of a child process, so a test can hand it a fake. */
export type VoiceChild = {
  stdout: { on: (event: "data", listener: (chunk: Buffer | string) => void) => unknown } | null;
  stdin: { end: () => unknown; on: (event: "error", listener: (error: Error) => void) => unknown } | null;
  on(event: "exit", listener: (code: number | null) => void): unknown;
  on(event: "error", listener: (error: Error) => void): unknown;
  kill: () => unknown;
};

export type VoiceHosts = {
  /** Armada Voice's executable, or null where this Bridge was built without it. */
  helper: string | null;
  /** Where Install put whisper.cpp and its model, or null before it has. */
  whisper: () => { library: string; model: string } | null;
  spawn: (file: string, args: readonly string[]) => VoiceChild;
  /** macOS Dictation, sent to the first responder: `Menu.sendActionToFirstResponder`. */
  dictate: (action: "startDictation:" | "stopDictation:") => void;
  /** Calls `run` after `ms`; the returned function cancels it. */
  after: (ms: number, run: () => void) => () => void;
};

/**
 * How long a released helper has to say its last word before it is killed. Apple's final transcript follows in well
 * under a second; Whisper transcribes everything after the release, up to five minutes of speech.
 */
export const RELEASE_GRACE_MS: Record<"apple" | "whisper", number> = { apple: 6_000, whisper: 120_000 };

/**
 * Starts hearing with `engine`. `onWord` gets each word once; `onEnd` is called once, when nothing more will come,
 * with why where it ended early. The returned stop is the release: the helper finishes what it heard and ends.
 */
export function hear(engine: VoiceEngine, hosts: VoiceHosts, onWord: (word: string) => void, onEnd: (why?: VoiceWhy) => void): () => void {
  let ended = false;
  const end = (why?: VoiceWhy): void => {
    if (ended) return;
    ended = true;
    onEnd(why);
  };

  if (engine === "dictation") {
    hosts.dictate("startDictation:");
    return () => {
      if (ended) return;
      hosts.dictate("stopDictation:");
      end();
    };
  }

  if (hosts.helper === null) {
    end("helper_missing");
    return () => {};
  }
  let args: string[] = ["listen"];
  if (engine === "whisper") {
    const installed = hosts.whisper();
    if (installed === null) {
      end("whisper_absent");
      return () => {};
    }
    args = ["whisper", installed.library, installed.model];
  }

  const child = hosts.spawn(hosts.helper, args);
  const settled = new Settled();
  let released = false;
  let cancelKill = (): void => {};
  const finish = (why?: VoiceWhy): void => {
    cancelKill();
    end(why);
  };
  let pending = "";
  child.stdout?.on("data", (chunk) => {
    pending += chunk.toString();
    const lines = pending.split("\n");
    pending = lines.pop() ?? "";
    for (const line of lines) {
      const read = readHelperLine(line);
      if (read === undefined || ended) continue;
      if (read.kind === "partial") settled.partial(read.text).forEach(onWord);
      if (read.kind === "final") {
        settled.final(read.text).forEach(onWord);
        finish();
      }
      if (read.kind === "error") finish(whyOf(read.code));
    }
  });
  child.stdin?.on("error", () => {});
  child.on("error", () => finish("helper_missing"));
  // Released, a helper that ends without a final line heard nothing; unreleased, it died.
  child.on("exit", () => finish(released ? undefined : "helper_failed"));
  return () => {
    if (released || ended) return;
    released = true;
    child.stdin?.end();
    cancelKill = hosts.after(RELEASE_GRACE_MS[engine], () => {
      child.kill();
      finish();
    });
  };
}

/** Armada Voice's executable: built beside the sources in development, in `Contents/Helpers` packaged. */
export function helperPath(packaged: boolean, appPath: string, resourcesPath: string): string {
  const app = packaged ? join(resourcesPath, "..", "Helpers", "Armada Voice.app") : join(appPath, "native", "voice", "build", "Armada Voice.app");
  return join(app, "Contents", "MacOS", "armada-voice");
}
