// Builds Armada Voice, the helper Talk hears through: `node native/voice/build.mjs` from `apps/desktop`, which `pnpm
// build` runs first and electron-builder's `beforePack` runs again (`before-pack.cjs`). Writes
// `native/voice/build/Armada Voice.app`, signed ad hoc; the packaged app carries it in `Contents/Helpers`.
//
// **An app of its own, not a bare binary**, because macOS asks for the microphone and speech recognition on behalf of
// the process it holds responsible, and reads the reason it shows from that process's Info.plist. The helper makes
// itself that process (`voice_run_disclaimed`), so the prompt is the same in development, where Bridge is started by
// `node`, and packaged.
//
// Off macOS, or with no Swift compiler, it says so and builds nothing: Talk's Apple and Whisper engines then report the
// helper missing. It builds nothing when the app is newer than every source.

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const build = join(here, "build");
const app = join(build, "Armada Voice.app");
const binary = join(app, "Contents", "MacOS", "armada-voice");
const sources = readdirSync(here).filter((name) => /\.(swift|c|h|plist|mjs)$/.test(name)).map((name) => join(here, name));

function has(tool) {
  try {
    execFileSync("xcrun", ["--find", tool], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

if (process.platform !== "darwin") {
  console.log("voice: not macOS, Armada Voice not built");
} else if (!has("swiftc")) {
  console.log("voice: no swiftc (xcode-select --install), Armada Voice not built");
} else if (existsSync(binary) && sources.every((file) => statSync(file).mtimeMs < statSync(binary).mtimeMs)) {
  console.log("voice: Armada Voice is up to date");
} else {
  const target = `${process.arch === "arm64" ? "arm64" : "x86_64"}-apple-macos13.0`;
  rmSync(app, { recursive: true, force: true });
  mkdirSync(dirname(binary), { recursive: true });
  const object = join(build, "voice-native.o");
  const run = (tool, args) => execFileSync("xcrun", [tool, ...args], { stdio: "inherit" });
  run("clang", ["-c", "-O2", "-target", target, join(here, "voice-native.c"), "-o", object]);
  run("swiftc", ["-O", "-target", target, "-import-objc-header", join(here, "voice-native.h"), join(here, "main.swift"), object, "-o", binary]);
  copyFileSync(join(here, "Info.plist"), join(app, "Contents", "Info.plist"));
  execFileSync("codesign", ["--force", "--sign", "-", app], { stdio: "inherit" });
  console.log(`voice: built ${app}`);
}
