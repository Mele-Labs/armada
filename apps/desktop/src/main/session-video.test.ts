// The scheme serves a video the Session's ledger names and nothing else, and a seek gets the bytes it asked for.

import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import type { SessionRecord } from "@armada/protocol";
import { answerVideo, answerVideoRequest, rangeWithin, type VideoReads } from "./session-video";
import { videoAddressOf } from "../shared/video-address";

const shown = (target: string, form = "file") => ({
  kind: "artifact",
  target,
  state: "standing",
  detail: {
    form,
  },
});
const record = {
  id: "S1",
  attachments: [
    shown("/work/run.mp4"),
    shown("/work/run.MOV"),
    shown("/work/run.webm"),
    shown("/work/notes.md"),
    shown("/work/gone.mp4"),
    shown("/work/empty.mp4"),
    shown("/work/page.mp4", "page"),
  ],
} as unknown as SessionRecord;

const sliced = vi.fn(
  (
    _path: string,
    slice: {
      start: number;
      end: number;
    },
  ) => slice,
);
const reads: VideoReads<{
  start: number;
  end: number;
}> = {
  size: async (path) => {
    if (path === "/work/gone.mp4") throw new Error("ENOENT");
    return path === "/work/empty.mp4" ? 0 : 100;
  },
  slice: sliced,
};
const answer = (path: string, range: string | null = null) =>
  answerVideo(record, path, range, reads);

const BARE = {
  status: 404,
  headers: {},
  body: null,
};

describe("what the scheme refuses", () => {
  it("a path the ledger does not name is a bare 404, and nothing is read", async () => {
    sliced.mockClear();
    expect(await answer("/etc/secret.mp4")).toEqual(BARE);
    expect(sliced).not.toHaveBeenCalled();
  });

  it("a session with no record names nothing", async () => {
    expect(await answerVideo(undefined, "/work/run.mp4", null, reads)).toEqual(BARE);
  });

  it("a target the ledger holds as something other than a file is a bare 404", async () => {
    expect(await answer("/work/page.mp4")).toEqual(BARE);
  });

  it("a file the ledger names that is not a video is a bare 404", async () => {
    sliced.mockClear();
    expect(await answer("/work/notes.md")).toEqual(BARE);
    expect(sliced).not.toHaveBeenCalled();
  });

  it("a video that has gone from the disk is a bare 404", async () => {
    expect(await answer("/work/gone.mp4")).toEqual(BARE);
  });
});

describe("what it answers", () => {
  it("no Range is 200 and the whole file, seekable and never cached", async () => {
    expect(await answer("/work/run.mp4")).toEqual({
      status: 200,
      headers: {
        "Accept-Ranges": "bytes",
        "Cache-Control": "no-store",
        "Content-Length": "100",
        "Content-Type": "video/mp4",
      },
      body: {
        start: 0,
        end: 99,
      },
    });
  });

  it.each([
    [
      "/work/run.mp4",
      "video/mp4",
    ],
    [
      "/work/run.MOV",
      "video/quicktime",
    ],
    [
      "/work/run.webm",
      "video/webm",
    ],
  ])("%s is %s", async (path, type) => {
    expect((await answer(path)).headers["Content-Type"]).toBe(type);
  });

  it.each([
    [
      "closed",
      "bytes=10-19",
      10,
      19,
    ],
    [
      "open ended",
      "bytes=90-",
      90,
      99,
    ],
    [
      "suffix",
      "bytes=-5",
      95,
      99,
    ],
    [
      "past the end, cut to it",
      "bytes=90-500",
      90,
      99,
    ],
    [
      "suffix longer than the file",
      "bytes=-500",
      0,
      99,
    ],
    [
      "the last byte",
      "bytes=99-99",
      99,
      99,
    ],
  ])("range %s is 206 with exactly its bytes", async (_name, range, start, end) => {
    const got = await answer("/work/run.mp4", range);
    expect(got.status).toBe(206);
    expect(got.body).toEqual({
      start,
      end,
    });
    expect(got.headers).toMatchObject({
      "Accept-Ranges": "bytes",
      "Cache-Control": "no-store",
      "Content-Type": "video/mp4",
      "Content-Length": String(end - start + 1),
      "Content-Range": `bytes ${start}-${end}/100`,
    });
  });

  it.each([
    [
      "starting at the size",
      "bytes=100-",
    ],
    [
      "starting past it",
      "bytes=500-600",
    ],
    [
      "an empty suffix",
      "bytes=-0",
    ],
  ])("range %s is 416, naming the size", async (_name, range) => {
    sliced.mockClear();
    const got = await answer("/work/run.mp4", range);
    expect(got.status).toBe(416);
    expect(got.body).toBeNull();
    expect(got.headers["Content-Range"]).toBe("bytes */100");
    expect(got.headers["Accept-Ranges"]).toBe("bytes");
    expect(sliced).not.toHaveBeenCalled();
  });

  it.each([
    [
      "a unit it does not count",
      "items=0-5",
    ],
    [
      "nothing to read",
      "bytes=-",
    ],
    [
      "an end before its start",
      "bytes=9-5",
    ],
    [
      "several at once",
      "bytes=0-1,3-4",
    ],
  ])("a header with %s is the whole file", async (_name, range) => {
    expect((await answer("/work/run.mp4", range)).status).toBe(200);
  });

  it("an empty file is 200 with nothing to read, and any range of it is 416", async () => {
    sliced.mockClear();
    expect(await answer("/work/empty.mp4")).toMatchObject({
      status: 200,
      body: null,
      headers: {
        "Content-Length": "0",
      },
    });
    expect(sliced).not.toHaveBeenCalled();
    expect((await answer("/work/empty.mp4", "bytes=0-")).status).toBe(416);
    expect((await answer("/work/empty.mp4", "bytes=-5")).status).toBe(416);
  });

  it("rangeWithin takes the header's spacing and case as HTTP does", () => {
    expect(rangeWithin(" BYTES=1-2 ", 10)).toEqual({
      start: 1,
      end: 2,
    });
  });
});

describe("over a request, from the disk", () => {
  let dir: string;
  let path: string;
  let onDisk: SessionRecord;

  beforeAll(async () => {
    dir = await mkdtemp(join(tmpdir(), "armada-video-"));
    path = join(dir, "a clip #1 & more.mp4");
    await writeFile(
      path,
      Uint8Array.from(
        {
          length: 256,
        },
        (_, at) => at,
      ),
    );
    onDisk = {
      id: "S1",
      attachments: [
        shown(path),
      ],
    } as unknown as SessionRecord;
  });
  afterAll(() =>
    rm(dir, {
      recursive: true,
    }),
  );

  const ask = (at: string, range?: string, method = "GET") =>
    answerVideoRequest(
      new Request(at, {
        method,
        ...(range === undefined
          ? {}
          : {
              headers: {
                Range: range,
              },
            }),
      }),
      (id) => (id === "S1" ? onDisk : undefined),
    );

  it("a range reads exactly those bytes, through a path that needs escaping", async () => {
    const response = await ask(videoAddressOf("S1", path), "bytes=250-");
    expect(response.status).toBe(206);
    expect(response.headers.get("Content-Range")).toBe("bytes 250-255/256");
    expect([
      ...new Uint8Array(await response.arrayBuffer()),
    ]).toEqual([
      250,
      251,
      252,
      253,
      254,
      255,
    ]);
  });

  it("no range reads the whole file", async () => {
    const response = await ask(videoAddressOf("S1", path));
    expect(response.status).toBe(200);
    expect((await response.arrayBuffer()).byteLength).toBe(256);
  });

  it("the fragment a player adds changes nothing", async () => {
    const response = await ask(`${videoAddressOf("S1", path)}#t=0.001`, "bytes=0-0");
    expect(response.status).toBe(206);
    expect([
      ...new Uint8Array(await response.arrayBuffer()),
    ]).toEqual([
      0,
    ]);
  });

  it("another Session's name for the file is a 404 with no body", async () => {
    const response = await ask(videoAddressOf("S2", path));
    expect(response.status).toBe(404);
    expect(response.body).toBeNull();
  });

  it("an address missing its session or its path is a 404", async () => {
    expect((await ask("armada-video://shown/?session=S1")).status).toBe(404);
    expect((await ask("armada-video://shown/?path=%2Fwork%2Frun.mp4")).status).toBe(404);
  });

  it("anything but a read is a 404", async () => {
    expect((await ask(videoAddressOf("S1", path), undefined, "POST")).status).toBe(404);
  });

  it("a video deleted after the ledger named it is a 404", async () => {
    const lost = join(dir, "lost.mp4");
    const response = await answerVideoRequest(
      new Request(videoAddressOf("S1", lost)),
      () =>
        ({
          id: "S1",
          attachments: [
            shown(lost),
          ],
        }) as unknown as SessionRecord,
    );
    expect(response.status).toBe(404);
  });
});
