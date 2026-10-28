// A Job browser's window in the mock, drawn over the page because a browser has no second window, in the frame the terminal's
// window is drawn in (`TerminalWindows.tsx`). **The page with the Job beside it**, as the walk window draws its Job: the pane is
// the app's own, and the Job is `SessionView`, the Cockpit's, as `WalkJobPanel.tsx` draws it. The OS close is Close window, which puts it away.

import { Square } from "lucide-react";
import { Button } from "@armada/components";

import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { closeBrowserWindow, useBrowserWindows, useJobBrowser } from "../job-browser/store";
import "./mock.css";

const HELD = { held: { state: "none" }, onWant: () => {} } as const;

function Frame({ sessionId }: { sessionId: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const draft = useSessionsDraft();
  const browser = useJobBrowser(sessionId);
  if (browser === undefined || session === undefined || draft === undefined) return null;
  const title = `Job browser: ${session.title ?? sessionId}`;
  return (
    <section role="dialog" aria-label={title} className="armada-mock-terminal-window armada-mock-browser-window" data-front="">
      <div className="armada-mock-terminal-window__strip">
        <span>{title}</span>
        <Button variant="secondary" size="sm" onClick={() => closeBrowserWindow(sessionId)}>
          Close window
        </Button>
      </div>
      <div className="armada-mock-browser-window__body">
        <div className="armada-mock-terminal-window__document">
          <JobBrowserPane sessionId={sessionId} browser={browser} placed="window" />
        </div>
        <section className="deck-pane armada-mock-browser-window__job" data-frame="" aria-label={`Job: ${session.title ?? session.id}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
          <SessionView
            session={session}
            goes={{ onGoTo: () => {}, onOpenLink: () => {}, onOpenCheck: () => {}, onOpenLoadout: () => {} }}
            onOpen={askToOpenSession}
            held={HELD}
            deck={{ ledger: null, back: null, acts: draft.stop === undefined ? [] : [{ id: "stop", label: "Stop", icon: Square, run: () => draft.stop?.(session.id) }] }}
          />
        </section>
      </div>
    </section>
  );
}

export function JobBrowserWindows() {
  return (
    <>
      {useBrowserWindows().map((sessionId) => (
        <Frame key={sessionId} sessionId={sessionId} />
      ))}
    </>
  );
}
