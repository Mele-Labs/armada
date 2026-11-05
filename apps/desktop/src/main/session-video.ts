// A video a Session showed, answered over `armada-video:` because one is too large to cross the preload.
//
// **Only a video the Session's own ledger names**, the guard `readSessionArtifact` and `openSessionFile` use, so the scheme
// reads no other file. Ranges are answered by hand: `net.fetch` of a `file://` does not reliably honour them, and a seek needs them.

import type { SessionRecord } from "@armada/protocol";
import { createReadStream } from "node:fs";
import { extname } from "node:path";
import { Readable } from "node:stream";

import { namesFile } from "./session-file";

const VIDEOS: Record<string, string> = {
  ".mp4": "video/mp4",
  ".mov": "video/quicktime",
  ".webm": "video/webm",
};

/** Both ends included, as `Content-Range` counts them. */
type Slice = {
  start: number;
  end: number;
};

/**
 * What a `Range` header asks of a file this size. **A header this cannot read asks for the whole file**, which HTTP lets a
 * server answer to any range: that covers several ranges at once, which no media element sends.
 */
export function rangeWithin(header: string | null, size: number): Slice | "whole" | "beyond" {
  const read = header === null ? null : /^bytes=(\d*)-(\d*)$/i.exec(header.trim());
  if (read === null) return "whole";
  const [, first = "", last = ""] = read;
  if (first === "" && last === "") return "whole";
  if (first === "") {
    const tail = Number(last);
    return tail === 0 || size === 0
      ? "beyond"
      : {
          start: Math.max(0, size - tail),
          end: size - 1,
        };
  }
  const start = Number(first);
  const end = last === "" ? size - 1 : Number(last);
  if (last !== "" && end < start) return "whole";
  return start >= size
    ? "beyond"
    : {
        start,
        end: Math.min(end, size - 1),
      };
}

/** What reads a video: its size, then the slice asked for. */
export type VideoReads<Body> = {
  size: (path: string) => Promise<number>;
  slice: (path: string, slice: Slice) => Body;
};

export type VideoAnswer<Body> = {
  status: 200 | 206 | 404 | 416;
  headers: Record<string, string>;
  body: Body | null;
};

const REFUSED = {
  status: 404,
  headers: {},
  body: null,
} as const;

/** The answer to one request. Refused and gone are the same bare 404, so the scheme says nothing about what is on the disk. */
export async function answerVideo<Body>(
  record: SessionRecord | undefined,
  path: string,
  range: string | null,
  reads: VideoReads<Body>,
): Promise<VideoAnswer<Body>> {
  const type = VIDEOS[extname(path).toLowerCase()];
  if (type === undefined || !namesFile(record, path)) return REFUSED;
  let size: number;
  try {
    size = await reads.size(path);
  } catch {
    return REFUSED;
  }
  const headers = {
    "Accept-Ranges": "bytes",
    "Cache-Control": "no-store",
    "Content-Type": type,
  };
  const asked = rangeWithin(range, size);
  if (asked === "beyond")
    return {
      status: 416,
      headers: {
        ...headers,
        "Content-Range": `bytes */${size}`,
      },
      body: null,
    };
  if (asked === "whole")
    return {
      status: 200,
      headers: {
        ...headers,
        "Content-Length": String(size),
      },
      // An end of -1 is what a read stream refuses, and an empty file has no slice to read.
      body:
        size === 0
          ? null
          : reads.slice(path, {
              start: 0,
              end: size - 1,
            }),
    };
  return {
    status: 206,
    headers: {
      ...headers,
      "Content-Length": String(asked.end - asked.start + 1),
      "Content-Range": `bytes ${asked.start}-${asked.end}/${size}`,
    },
    body: reads.slice(path, asked),
  };
}

const onDisk: VideoReads<ReadableStream<Uint8Array>> = {
  size: async (path) => {
    const held = await (await import("node:fs/promises")).stat(path);
    if (!held.isFile()) throw new Error("not a file");
    return held.size;
  },
  slice: (path, { start, end }) =>
    Readable.toWeb(
      createReadStream(path, {
        start,
        end,
      }),
    ) as ReadableStream<Uint8Array>,
};

/** The scheme's handler. `recordOf` is the record main holds for a session, as every other read of a Session's file takes it. */
export async function answerVideoRequest(
  request: Request,
  recordOf: (sessionId: string) => SessionRecord | undefined,
  reads: VideoReads<ReadableStream<Uint8Array>> = onDisk,
): Promise<Response> {
  const asked = new URL(request.url).searchParams;
  const sessionId = asked.get("session");
  const path = asked.get("path");
  if (request.method !== "GET" || sessionId === null || path === null)
    return new Response(null, {
      status: 404,
    });
  const answer = await answerVideo(recordOf(sessionId), path, request.headers.get("range"), reads);
  return new Response(answer.body, {
    status: answer.status,
    headers: answer.headers,
  });
}
