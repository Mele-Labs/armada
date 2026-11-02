// Whisper's install against a fake disk: what the window is told as it comes down, that a file is kept only with the
// digest it was pinned to, and that a stopped install leaves Whisper absent. Nothing is downloaded.

import { createHash } from "node:crypto";

import { describe, expect, it } from "vitest";

import type { WhisperState } from "../shared/api/voice";
import { FRAMEWORK_IN_ZIP, WHISPER_MODEL, WHISPER_PARTS, WhisperInstall } from "./whisper-install";
import type { WhisperDisk, WhisperPart } from "./whisper-install";

/** Bytes standing in for each part, with the parts' pins rewritten to their digests. */
const BYTES: Record<WhisperPart["name"], Uint8Array> = {
  library: new Uint8Array(300).fill(1),
  model: new Uint8Array(100).fill(2),
};
const sha = (bytes: Uint8Array): string => createHash("sha256").update(bytes).digest("hex");

function setUp(
  options: { marked?: boolean; corrupt?: WhisperPart["name"]; failing?: WhisperPart["name"] } = {},
) {
  const told: WhisperState[] = [];
  const did: string[] = [];
  let marked = options.marked ?? false;
  const parts = WHISPER_PARTS.map((part) => ({
    ...part,
    bytes: BYTES[part.name].byteLength,
    sha256: sha(BYTES[part.name]),
  }));
  const disk: WhisperDisk = {
    download: async (url, file, onChunk) => {
      const part = parts.find((one) => one.url === url)!;
      did.push(`download ${part.name} to ${file}`);
      if (options.failing === part.name) throw new Error("offline");
      const bytes =
        options.corrupt === part.name ? new Uint8Array(BYTES[part.name]).fill(9) : BYTES[part.name];
      // Ten chunks a part, so progress moves in steps the window can see.
      const size = bytes.byteLength / 10;
      for (let at = 0; at < bytes.byteLength; at += size) onChunk(bytes.subarray(at, at + size));
    },
    unpack: async (zip, inner, to) => void did.push(`unpack ${inner} from ${zip} to ${to}`),
    marked: () => marked,
    mark: async () => {
      did.push("mark");
      marked = true;
    },
    clear: async () => void did.push("clear"),
  };
  const install = new WhisperInstall("/data/whisper", disk, (state) => told.push(state), parts);
  return {
    install,
    told,
    did,
  };
}

describe("Whisper's install", () => {
  it("is absent until Install is pressed, and installed where it was before", () => {
    expect(setUp().install.current).toEqual({
      state: "absent",
    });
    expect(setUp().install.paths()).toBeNull();
    const before = setUp({
      marked: true,
    }).install;
    expect(before.current).toEqual({
      state: "installed",
    });
    expect(before.paths()).toEqual({
      library: "/data/whisper/whisper.framework/Versions/A/whisper",
      model: `/data/whisper/${WHISPER_MODEL}`,
    });
  });

  it("comes down with its progress, unpacks the macOS framework, and is installed once marked", async () => {
    const { install, told, did } = setUp();
    expect(await install.install()).toEqual({
      ok: true,
    });
    expect(did).toEqual([
      "download library to /data/whisper/whisper.zip",
      `download model to /data/whisper/${WHISPER_MODEL}`,
      `unpack ${FRAMEWORK_IN_ZIP} from /data/whisper/whisper.zip to /data/whisper/whisper.framework`,
      "mark",
    ]);
    expect(told[0]).toEqual({
      state: "installing",
      progress: 0,
    });
    const progress = told.flatMap((one) =>
      one.state === "installing"
        ? [
            one.progress,
          ]
        : [],
    );
    expect(progress.length).toBeGreaterThan(5);
    expect(progress).toEqual(
      [
        ...progress,
      ].sort((a, b) => a - b),
    );
    expect(Math.max(...progress)).toBeLessThan(1);
    expect(told.at(-1)).toEqual({
      state: "installed",
    });
    expect(install.paths()).not.toBeNull();
  });

  it("a second press while it comes down joins the first", async () => {
    const { install, did } = setUp();
    const [first, second] = [
      install.install(),
      install.install(),
    ];
    expect(await first).toEqual({
      ok: true,
    });
    expect(await second).toEqual({
      ok: true,
    });
    expect(did.filter((one) => one.startsWith("download library"))).toHaveLength(1);
  });

  it("keeps nothing whose digest is not the pinned one", async () => {
    const { install, told, did } = setUp({
      corrupt: "model",
    });
    expect(await install.install()).toEqual({
      ok: false,
      why: "digest_mismatch",
    });
    expect(did).not.toContain("mark");
    expect(did.at(-1)).toBe("clear");
    expect(told.at(-1)).toEqual({
      state: "absent",
    });
    expect(install.paths()).toBeNull();
  });

  it("a failed download leaves Whisper absent, and Install can be pressed again", async () => {
    const { install, told } = setUp({
      failing: "library",
    });
    expect(await install.install()).toEqual({
      ok: false,
      why: "download_failed",
    });
    expect(told.at(-1)).toEqual({
      state: "absent",
    });
    expect(install.current).toEqual({
      state: "absent",
    });
  });
});
