// A Job browser's window in the mock, drawn over the page because a browser has no second window, in the frame the terminal's
// window is drawn in (`TerminalWindows.tsx`). The pane is the app's own; the OS close is the Close window button, which puts it away.

import { Button } from "@armada/components";

import { useSessions } from "../sessions-draft";
import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { closeBrowserWindow, useBrowserWindows, useJobBrowser } from "../job-browser/store";
import "./mock.css";

function Frame({ sessionId }: { sessionId: string }) {
  const title = `Job browser: ${useSessions().find((one) => one.id === sessionId)?.title ?? sessionId}`;
  const browser = useJobBrowser(sessionId);
  if (browser === undefined) return null;
  return (
    <section role="dialog" aria-label={title} className="armada-mock-terminal-window" data-front="">
      <div className="armada-mock-terminal-window__strip">
        <span>{title}</span>
        <Button variant="secondary" size="sm" onClick={() => closeBrowserWindow(sessionId)}>
          Close window
        </Button>
      </div>
      <div className="armada-mock-terminal-window__document">
        <JobBrowserPane sessionId={sessionId} browser={browser} placed="window" />
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
