// A picture or a video a Session showed, read for its row: a picture's bytes become a blob, and a video plays from an address.
// Browser: `@armada/components` is what names which is which.

import type { ArtifactRead, VideoAddress } from "@armada/screens/src/draft/sessions";
import { afterEach, describe, expect, test, vi } from "vitest";

import { readShownMedia } from "./session-media";

const SAYS = "This video cannot play here.";
const VIDEO = "/work/run.mp4";
const PICTURE = "/work/shot.png";

const drafted = (draft: {
  address?: VideoAddress[];
  read?: ArtifactRead;
  openFile?: (id: string, path: string) => void;
}) => {
  const videoAddress = vi.fn(async () => draft.address?.shift() as VideoAddress);
  const readArtifact = vi.fn(async () => draft.read as ArtifactRead);
  return {
    videoAddress,
    readArtifact,
    ...(draft.openFile === undefined
      ? {}
      : {
          openFile: draft.openFile,
        }),
  };
};

afterEach(() => vi.restoreAllMocks());

describe("a picture", () => {
  test("its bytes become a blob of its type, and no address is asked for", async () => {
    const draft = drafted({
      read: {
        ok: true,
        bytes: new Uint8Array([
          1,
          2,
          3,
        ]),
        type: "image/png",
      },
    });
    const made = vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:made");
    await expect(readShownMedia(draft, "s1", PICTURE)).resolves.toEqual({
      kind: "picture",
      url: "blob:made",
    });
    expect(draft.readArtifact).toHaveBeenCalledWith("s1", PICTURE);
    expect(draft.videoAddress).not.toHaveBeenCalled();
    const blob = made.mock.calls[0]?.[0] as Blob;
    expect(blob.type).toBe("image/png");
    expect(blob.size).toBe(3);
  });

  test("too big says the limit, in megabytes", async () => {
    await expect(
      readShownMedia(
        drafted({
          read: {
            ok: false,
            why: "too_big",
            limit: 8 * 1024 * 1024,
          },
        }),
        "s1",
        PICTURE,
      ),
    ).resolves.toEqual({
      kind: "refused",
      says: "This picture is larger than 8 MB.",
    });
  });

  test("anything else refused says it could not be read", async () => {
    await expect(
      readShownMedia(
        drafted({
          read: {
            ok: false,
            why: "not_addressable",
          },
        }),
        "s1",
        PICTURE,
      ),
    ).resolves.toEqual({
      kind: "refused",
      says: "This picture could not be read.",
    });
  });
});

describe("a video", () => {
  test("plays from the address it asked for, and reads no bytes", async () => {
    const draft = drafted({
      address: [
        {
          ok: true,
          address: "armada-video://shown/?one",
        },
      ],
    });
    await expect(readShownMedia(draft, "s1", VIDEO)).resolves.toMatchObject({
      kind: "video",
      url: "armada-video://shown/?one",
    });
    expect(draft.videoAddress).toHaveBeenCalledWith("s1", VIDEO);
    expect(draft.readArtifact).not.toHaveBeenCalled();
  });

  test("`again` asks afresh", async () => {
    const draft = drafted({
      address: [
        {
          ok: true,
          address: "one",
        },
        {
          ok: true,
          address: "two",
        },
      ],
    });
    const media = await readShownMedia(draft, "s1", VIDEO);
    if (media.kind !== "video") throw new Error(media.kind);
    await expect(media.again?.()).resolves.toBe("two");
    expect(draft.videoAddress).toHaveBeenCalledTimes(2);
  });

  test("`again` refused rejects with the words it was refused in", async () => {
    const media = await readShownMedia(
      drafted({
        address: [
          {
            ok: true,
            address: "one",
          },
          {
            ok: false,
            says: "That Job did not show that file.",
          },
        ],
      }),
      "s1",
      VIDEO,
    );
    if (media.kind !== "video") throw new Error(media.kind);
    await expect(media.again?.()).rejects.toThrow("That Job did not show that file.");
  });

  test("`open` hands this Session's video to the Mac, when the host can", async () => {
    const openFile = vi.fn();
    const media = await readShownMedia(
      drafted({
        address: [
          {
            ok: true,
            address: "one",
          },
        ],
        openFile,
      }),
      "s1",
      VIDEO,
    );
    if (media.kind !== "video") throw new Error(media.kind);
    media.open?.();
    expect(openFile).toHaveBeenCalledExactlyOnceWith("s1", VIDEO);
  });

  test("with no `openFile`, as in the mock, there is no `open`", async () => {
    const media = await readShownMedia(
      drafted({
        address: [
          {
            ok: true,
            address: "one",
          },
        ],
      }),
      "s1",
      VIDEO,
    );
    expect(media).not.toHaveProperty("open");
  });

  test("a refused address keeps its words and its way out", async () => {
    const openFile = vi.fn();
    const media = await readShownMedia(
      drafted({
        address: [
          {
            ok: false,
            says: "That Job did not show that file.",
          },
        ],
        openFile,
      }),
      "s1",
      VIDEO,
    );
    expect(media).toMatchObject({
      kind: "refused",
      says: "That Job did not show that file.",
    });
    if (media.kind !== "refused") throw new Error(media.kind);
    media.open?.();
    expect(openFile).toHaveBeenCalledWith("s1", VIDEO);
  });

  test("a host that serves no video says it cannot play here", async () => {
    await expect(readShownMedia({}, "s1", VIDEO)).resolves.toEqual({
      kind: "refused",
      says: SAYS,
    });
  });
});
