// The Job browser in the side panel, and the Job's own thread and composer floating on the left over the app, as a plate off the
// sheet's leading edge (`Sheet`'s `aside`), so the page and the Job are read and answered together. Hide Job folds the thread to a
// plate that brings it back, the walk window's own toggle. The thread is `SessionView`, the Cockpit's, as the walk window draws its Job.

import { Square } from "lucide-react";
import { Button, Sheet } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { useBack } from "../shared-panel/panels";
import { closePanel, type PanelItem } from "../shared-panel/store";
import { JobBrowserPane } from "./JobBrowserPane";
import { showThread, useJobBrowser } from "./store";

const HELD = { held: { state: "none" }, onWant: () => {} } as const;

/** The Job beside its browser: its thread and its composer, the Cockpit's own. */
export function JobBeside({ sessionId, className }: { sessionId: string; className?: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const draft = useSessionsDraft();
  if (session === undefined || draft === undefined) return null;
  return (
    <section className={["deck-pane", className].filter(Boolean).join(" ")} data-frame="" aria-label={`Job: ${session.title ?? session.id}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
      <SessionView
        session={session}
        goes={{ onGoTo: () => {}, onOpenLink: () => {}, onOpenCheck: () => {}, onOpenLoadout: () => {} }}
        onOpen={askToOpenSession}
        held={HELD}
        deck={{ ledger: null, back: null, acts: draft.stop === undefined ? [] : [{ id: "stop", label: "Stop", icon: Square, run: () => draft.stop?.(session.id) }] }}
      />
    </section>
  );
}

export function BrowserSide({ item }: { item: Extract<PanelItem, { kind: "browser" }> }) {
  const browser = useJobBrowser(item.sessionId);
  const title = useSessions().find((one) => one.id === item.sessionId)?.title;
  const floor = useAtFloor();
  const back = useBack();
  if (browser === undefined) return null;
  const shown = !browser.threadHidden;
  const aside = (
    <div className="job-browser-beside" data-shown={shown ? "" : undefined}>
      <Button variant="ghost" size="sm" aria-expanded={shown} onClick={() => showThread(item.sessionId, !shown)}>
        {`${shown ? "Hide" : "Show"} Job`}
      </Button>
      {shown ? <JobBeside sessionId={item.sessionId} className="job-browser-beside__job" /> : null}
    </div>
  );
  return (
    <Sheet
      kind="job-browser"
      open
      floating
      floor={floor}
      title="Job browser"
      {...(title === undefined ? {} : { subtitle: title })}
      size="widest"
      bleed
      {...(back === undefined ? {} : { back })}
      closeLabel="Close"
      closeBinding="Esc"
      aside={aside}
      onClose={closePanel}
    >
      <JobBrowserPane sessionId={item.sessionId} browser={browser} placed="panel" />
    </Sheet>
  );
}
