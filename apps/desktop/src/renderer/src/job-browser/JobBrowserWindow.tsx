// A Browser tab's page in a window of its own: the page with the Job beside it, as the walk window draws its Job. Bridge's window is
// `../browser-window/main.tsx`; the mock draws the same inside its own frame (`../mock/JobBrowserWindows.tsx`).

import { Square } from "lucide-react";

import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { JobBrowserPane } from "./JobBrowserPane";
import { useJobBrowser } from "./store";

const HELD = {
  held: {
    state: "none",
  },
  onWant: () => {},
} as const;

/** The Job beside its browser: its thread and its composer, the Cockpit's own. */
function JobBeside({ sessionId, className }: { sessionId: string; className?: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const draft = useSessionsDraft();
  if (session === undefined || draft === undefined) return null;
  return (
    <section
      className={[
        "deck-pane",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      data-frame=""
      aria-label={`Job: ${session.title ?? session.id}`}
      {...{
        [OPEN_SESSION_ATTRIBUTE]: session.id,
      }}
    >
      <SessionView
        session={session}
        goes={{
          onGoTo: () => {},
          onOpenLink: () => {},
          onOpenCheck: () => {},
          onOpenLoadout: () => {},
        }}
        onOpen={askToOpenSession}
        held={HELD}
        deck={{
          ledger: null,
          back: null,
          acts:
            draft.stop === undefined
              ? []
              : [
                  {
                    id: "stop",
                    label: "Stop",
                    icon: Square,
                    run: () => draft.stop?.(session.id),
                  },
                ],
        }}
      />
    </section>
  );
}

/** `tabId` is left out by the mock, which plays one browser for the Job. */
export function JobBrowserWindow({ sessionId, tabId = "" }: { sessionId: string; tabId?: string }) {
  const browser = useJobBrowser(sessionId, tabId);
  if (browser === undefined) return null;
  return (
    <div className="job-browser-window">
      <div className="job-browser-window__page">
        <JobBrowserPane sessionId={sessionId} tabId={tabId} browser={browser} placed="window" />
      </div>
      <JobBeside sessionId={sessionId} className="job-browser-window__job" />
    </div>
  );
}
