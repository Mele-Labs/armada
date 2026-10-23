// Where notes on a sketch, a file or a picture go: a Job's thread, as a message from the person with the notes' pictures
// along. **Sent the way the Session composer sends**, through the Sessions draft, so the mock's Fleet and a real one each
// take it as they take any message, and a picture reaches the agent as an attachment. Mounted with the Sessions.

import { useMemo, useSyncExternalStore, type ReactNode } from "react";
import { ViewerNotesFrom } from "@armada/components";
import type { NotePicture, ViewerNotesHost } from "@armada/components";
import type { SentFile, Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

const NONE = () => () => undefined;
const NO_SESSIONS: readonly Session[] = [];
const EMPTY = () => NO_SESSIONS;

/** A picture's `data:` address as a file the send uploads, and the address the thread draws it from. */
function fileOf(picture: NotePicture, at: number): SentFile | undefined {
  const comma = picture.png.indexOf(",");
  if (comma < 0) return undefined;
  const bytes = Uint8Array.from(atob(picture.png.slice(comma + 1)), (one) => one.charCodeAt(0));
  return { id: `note-${String(Date.now())}-${String(at)}`, name: picture.name, src: picture.png, file: new File([bytes], picture.name, { type: "image/png" }) };
}

export function ViewerNotesOver({ draft, children }: { draft: SessionsDraft | undefined; children: ReactNode }) {
  const sessions = useSyncExternalStore(draft?.subscribe ?? NONE, draft?.get ?? EMPTY);
  const host = useMemo((): ViewerNotesHost | undefined => {
    if (draft === undefined) return undefined;
    return {
      // The Jobs that can still be told something.
      jobs: sessions.filter((one) => one.dead === undefined).map((one) => ({ id: one.id, title: one.title ?? one.address ?? one.id })),
      send: async (job, message) => {
        const files = message.pictures.flatMap((one, at) => fileOf(one, at) ?? []);
        return (await draft.send(job, { text: message.text, files, sketches: [], tags: [] })) !== false;
      },
    };
  }, [draft, sessions]);
  return <ViewerNotesFrom host={host}>{children}</ViewerNotesFrom>;
}
