// A file the agent shows with `show_file` opens in the side panel for the Session that is on screen. **Told by `shown`**, the
// time Fleet took at each call: a second show of one path is a new `shown`. What the Session already held when it came on
// screen is not new, so reopening a Session, or a reload, opens nothing; its thread row opens the file instead.

import { useEffect, useRef } from "react";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "./shared-panel/store";

export function useShownFiles(session: Pick<Session, "id" | "attachments">): void {
  const seen = useRef<{ id: string; shown: Set<string> } | undefined>(undefined);
  useEffect(() => {
    const now = session.attachments.flatMap((one) => (one.kind === "artifact" && one.form === "file" && one.by === "agent" && one.shown !== undefined ? [{ path: one.id, key: `${one.id}@${one.shown}` }] : []));
    if (seen.current?.id !== session.id) {
      seen.current = { id: session.id, shown: new Set(now.map((one) => one.key)) };
      return;
    }
    for (const one of now) {
      if (seen.current.shown.has(one.key)) continue;
      seen.current.shown.add(one.key);
      openInPanel({ kind: "file", path: one.path, from: "session", sessionId: session.id });
    }
  }, [session.id, session.attachments]);
}
