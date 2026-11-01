// Whisper for Talk, installed on the owner's press and never before, into Bridge's own data folder.
//
// **What comes down, and why these two.** whisper.cpp publishes no macOS command-line build, so the library is the
// Apple build ggml-org attaches to its release, whose macOS framework the helper loads (`native/voice/voice-native.c`);
// the release is pinned because the helper is compiled against that release's structs (`whisper-abi.h`). The model is
// `base.en` quantised to 5 bits: English only, as the box is, 60 MB against `small.en`'s 190 MB, and close enough to it
// for dictated sentences. Both are checked against the digests below before anything is kept.

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createWriteStream, readFileSync } from "node:fs";
import { mkdir, rename, rm, writeFile } from "node:fs/promises";
import { once } from "node:events";
import { join } from "node:path";
import { promisify } from "node:util";

import type { VoiceWhy, WhisperState } from "../shared/api/voice";

/** One file Install fetches: where from, how big, and the SHA-256 it must have. */
export type WhisperPart = { name: "library" | "model"; url: string; bytes: number; sha256: string };

export const WHISPER_RELEASE = "b5454";
export const WHISPER_MODEL = "ggml-base.en-q5_1.bin";

export const WHISPER_PARTS: readonly WhisperPart[] = [
  {
    name: "library",
    url: `https://github.com/ggml-org/whisper.cpp/releases/download/${WHISPER_RELEASE}/whisper-${WHISPER_RELEASE}-xcframework.zip`,
    bytes: 65_816_082,
    sha256: "e57f8c48933000acabc13bb913fbe82805d483a2deff692cc6738836bd92393b",
  },
  {
    name: "model",
    url: `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/${WHISPER_MODEL}`,
    bytes: 59_721_011,
    sha256: "4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f",
  },
];

/** Inside the release's zip: the macOS framework, the only slice the helper loads. */
export const FRAMEWORK_IN_ZIP = "build-apple/whisper.xcframework/macos-arm64_x86_64/whisper.framework";

/** What Install does to the disk, so a test can hand it a fake. Paths are inside `dir`. */
export type WhisperDisk = {
  /** Fetches `url` into `file`, calling `onChunk` with each chunk as it lands. Rejects on a failed request. */
  download: (url: string, file: string, onChunk: (chunk: Uint8Array) => void) => Promise<void>;
  /** Unpacks `inner` from the zip at `zip` to `to`. */
  unpack: (zip: string, inner: string, to: string) => Promise<void>;
  /** Whether the marker Install writes last is there, and says this release and model. */
  marked: () => boolean;
  mark: () => Promise<void>;
  /** Removes whatever a stopped install left. */
  clear: () => Promise<void>;
};

/** How much progress has to move before the window is told again: a hundred updates a download. */
const STEP = 0.01;

export class WhisperInstall {
  private state: WhisperState;
  private running: Promise<{ ok: true } | { ok: false; why: VoiceWhy }> | undefined;

  constructor(
    private readonly dir: string,
    private readonly disk: WhisperDisk,
    private readonly onChange: (state: WhisperState) => void,
    private readonly parts: readonly WhisperPart[] = WHISPER_PARTS,
  ) {
    this.state = disk.marked() ? { state: "installed" } : { state: "absent" };
  }

  get current(): WhisperState {
    return this.state;
  }

  /** The library and model the helper loads, once installed. */
  paths(): { library: string; model: string } | null {
    if (this.state.state !== "installed") return null;
    return { library: `${this.dir}/whisper.framework/Versions/A/whisper`, model: `${this.dir}/${WHISPER_MODEL}` };
  }

  /** Starts the install, or joins the one under way; installed already, it does nothing. */
  install(): Promise<{ ok: true } | { ok: false; why: VoiceWhy }> {
    if (this.state.state === "installed") return Promise.resolve({ ok: true });
    this.running ??= this.run().finally(() => {
      this.running = undefined;
    });
    return this.running;
  }

  private set(state: WhisperState): void {
    this.state = state;
    this.onChange(state);
  }

  private async run(): Promise<{ ok: true } | { ok: false; why: VoiceWhy }> {
    const total = this.parts.reduce((sum, part) => sum + part.bytes, 0);
    let done = 0;
    let told = 0;
    this.set({ state: "installing", progress: 0 });
    const files: Record<WhisperPart["name"], string> = { library: `${this.dir}/whisper.zip`, model: `${this.dir}/${WHISPER_MODEL}` };
    for (const part of this.parts) {
      const hash = createHash("sha256");
      try {
        await this.disk.download(part.url, files[part.name], (chunk) => {
          hash.update(chunk);
          done += chunk.byteLength;
          // Held under 1 until unpacked and marked: a full bar is installed.
          const progress = Math.min(done / total, 0.99);
          if (progress - told < STEP) return;
          told = progress;
          this.set({ state: "installing", progress });
        });
      } catch {
        return this.stop("download_failed");
      }
      if (hash.digest("hex") !== part.sha256) return this.stop("digest_mismatch");
    }
    try {
      await this.disk.unpack(files.library, FRAMEWORK_IN_ZIP, `${this.dir}/whisper.framework`);
      await this.disk.mark();
    } catch {
      return this.stop("unpack_failed");
    }
    this.set({ state: "installed" });
    return { ok: true };
  }

  private async stop(why: VoiceWhy): Promise<{ ok: false; why: VoiceWhy }> {
    await this.disk.clear().catch(() => {});
    this.set({ state: "absent" });
    return { ok: false, why };
  }
}

const MARK = JSON.stringify({ release: WHISPER_RELEASE, model: WHISPER_MODEL });

/** The disk Install writes: `dir` holds nothing but Whisper, so a stopped install clears the whole of it. */
export function whisperDisk(dir: string, fetcher: (url: string) => Promise<Response>): WhisperDisk {
  const marker = join(dir, "installed.json");
  return {
    download: async (url, file, onChunk) => {
      await mkdir(dir, { recursive: true });
      const response = await fetcher(url);
      if (!response.ok || response.body === null) throw new Error(`${url}: ${response.status}`);
      const out = createWriteStream(file);
      const reader = response.body.getReader();
      for (let read = await reader.read(); !read.done; read = await reader.read()) {
        onChunk(read.value);
        if (!out.write(read.value)) await once(out, "drain");
      }
      out.end();
      await once(out, "finish");
    },
    unpack: async (zip, inner, to) => {
      const staging = join(dir, "unpacking");
      await rm(staging, { recursive: true, force: true });
      // `unzip` keeps the framework's symlinks, and takes one slice of a zip that holds every Apple platform.
      await promisify(execFile)("/usr/bin/unzip", ["-q", zip, `${inner}/*`, "-d", staging]);
      await rm(to, { recursive: true, force: true });
      await rename(join(staging, inner), to);
      await rm(staging, { recursive: true, force: true });
      await rm(zip, { force: true });
    },
    marked: () => {
      try {
        return readFileSync(marker, "utf8") === MARK;
      } catch {
        return false;
      }
    },
    mark: () => writeFile(marker, MARK),
    clear: () => rm(dir, { recursive: true, force: true }),
  };
}
