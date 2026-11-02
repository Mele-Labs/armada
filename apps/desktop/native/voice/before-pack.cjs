// electron-builder's `beforePack`: builds Armada Voice, and refuses to pack an Armada that would ship without it.
// `build.mjs` says why it is an app of its own.

const { execFileSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { join } = require("node:path");

exports.default = async function beforePack() {
  execFileSync(
    process.execPath,
    [
      join(__dirname, "build.mjs"),
    ],
    {
      stdio: "inherit",
    },
  );
  const binary = join(__dirname, "build", "Armada Voice.app", "Contents", "MacOS", "armada-voice");
  if (!existsSync(binary))
    throw new Error(
      `Armada Voice was not built, so Talk would have no Apple or Whisper engine: ${binary}`,
    );
};
