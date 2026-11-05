// A picture or a video of a Job's Ledger, opened over the window as the thread opens its own: read as the thread's row reads it.

import { useEffect, useState } from "react";
import { EnlargedMedia, type ShownMedia } from "@armada/components";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { readShownMedia } from "./session-media";

export function MediaOverlay({
  draft,
  sessionId,
  path,
  onClose,
}: {
  draft: SessionsDraft;
  sessionId: string;
  path: string;
  onClose: () => void;
}) {
  const [media, setMedia] = useState<ShownMedia | undefined>();
  useEffect(() => {
    let gone = false;
    let made: string | undefined;
    void readShownMedia(draft, sessionId, path).then((got) => {
      const url = got.kind === "picture" ? got.url : undefined;
      if (gone) {
        if (url !== undefined) URL.revokeObjectURL(url);
        return;
      }
      made = url;
      setMedia(got);
    });
    return () => {
      gone = true;
      if (made !== undefined) URL.revokeObjectURL(made);
    };
  }, [
    draft,
    sessionId,
    path,
  ]);
  if (media === undefined || media.kind === "refused") return null;
  return (
    <EnlargedMedia
      src={media.url}
      name={path.slice(path.lastIndexOf("/") + 1)}
      kind={media.kind}
      onClose={onClose}
    />
  );
}
