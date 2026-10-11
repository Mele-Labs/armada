// A Session beside the page it showed: its thread live, the cards it is waiting on, and the box to
// answer it — `SessionMini`, as the Dashboard's pane draws it, over the same Sessions the window holds.
// `docs/practices/capture-window.md`, *The panel beside the page*.

import { WalkPanel } from "@armada/components";

import { SessionMini } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";

export function WalkSession({ sessionId }: { sessionId: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const served = useSessionsDraft() !== undefined;
  const openInBridge = () => void window.armada.walkPanel.openInBridge();
  return (
    <WalkPanel kind="session" title={session?.title ?? sessionId} onOpenInBridge={openInBridge}>
      {session !== undefined ? (
        // Open, from the thread or the box's Draw, is this Session's whole page on Bridge's window.
        <SessionMini sessionId={sessionId} fill onOpen={(id) => id === sessionId && openInBridge()} />
      ) : (
        <p className="text-fg-muted">{served ? "Fleet holds no Session by this id now." : "Reading the Session."}</p>
      )}
    </WalkPanel>
  );
}
