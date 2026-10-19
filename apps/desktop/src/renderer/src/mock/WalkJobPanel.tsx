// The Job view beside the page in the mock's walk window: **`SessionView`, the Cockpit's own, drawn into the element
// the window keeps for it** (`@armada/screens/src/fixtures/walk-window`). The window is a root of its own and has none of
// the app's providers, so this sits in the app's tree and portals into that element. It follows the walk on screen: the
// Job changes with the walk, and a Job that is working draws its Stop as the Cockpit does.

import { createPortal } from "react-dom";
import { Square } from "lucide-react";
import { useWalkQueue, walkOnScreen } from "@armada/screens/src/fixtures/walk-window";

import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";

const HELD = { held: { state: "none" }, onWant: () => {} } as const;

export function WalkJobPanel() {
  const queue = useWalkQueue();
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const id = walkOnScreen(queue)?.options.panelFor;
  const session = id === undefined ? undefined : sessions.find((one) => one.id === id);
  const host = queue?.panelHost ?? null;
  if (host === null || session === undefined || draft === undefined) return null;
  return createPortal(
    <section className="deck-pane" data-frame="" aria-label={`Job: ${session.title ?? session.id}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
      <SessionView
        key={session.id}
        session={session}
        goes={{ onGoTo: () => {}, onOpenLink: () => {}, onOpenCheck: () => {}, onOpenLoadout: () => {} }}
        onOpen={askToOpenSession}
        held={HELD}
        deck={{
          ledger: null,
          back: null,
          acts: draft.stop === undefined ? [] : [{ id: "stop", label: "Stop", icon: Square, run: () => draft.stop?.(session.id) }],
        }}
      />
    </section>,
    host,
  );
}
