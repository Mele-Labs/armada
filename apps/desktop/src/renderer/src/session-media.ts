// A picture or a video a Session showed, read for its row in the thread: a picture's bytes become a blob, and a video plays
// from an address, since one is too large to read into memory.

import { mediaKindOf, type ShownMedia } from "@armada/components";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

const CANNOT_PLAY = "This video cannot play here.";

export async function readShownMedia(
  draft: Pick<SessionsDraft, "readArtifact" | "videoAddress">,
  id: string,
  path: string,
): Promise<ShownMedia> {
  if (mediaKindOf(path) === "video") {
    const asked = await draft.videoAddress?.(id, path);
    if (asked === undefined)
      return {
        kind: "refused",
        says: CANNOT_PLAY,
      };
    return asked.ok
      ? {
          kind: "video",
          url: asked.address,
          again: async () => {
            const next = await draft.videoAddress?.(id, path);
            if (next?.ok !== true) throw new Error(next?.says ?? CANNOT_PLAY);
            return next.address;
          },
        }
      : {
          kind: "refused",
          says: asked.says,
        };
  }
  const read = await draft.readArtifact?.(id, path);
  if (read?.ok !== true)
    return {
      kind: "refused",
      says:
        read?.why === "too_big"
          ? `This picture is larger than ${Math.round((read.limit ?? 0) / (1024 * 1024))} MB.`
          : "This picture could not be read.",
    };
  return {
    kind: "picture",
    url: URL.createObjectURL(
      new Blob(
        [
          read.bytes as BlobPart,
        ],
        {
          type: read.type,
        },
      ),
    ),
  };
}
