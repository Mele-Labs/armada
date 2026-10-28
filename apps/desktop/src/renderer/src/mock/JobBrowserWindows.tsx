// A Job browser's window in the mock, drawn over the page because a browser has no second window, in the frame the terminal's
// window is drawn in (`TerminalWindows.tsx`). **The page with the Job beside it**, as the walk window draws its Job: the pane is
// the app's own, and so is the Job beside it (`JobBeside`). The OS close is Close window, which puts it away.

import { Button } from "@armada/components";

import { useSessions } from "../sessions-draft";
import { JobBeside } from "../job-browser/BrowserSide";
import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { closeBrowserWindow, useBrowserWindows, useJobBrowser } from "../job-browser/store";
import "./mock.css";

function Frame({ sessionId }: { sessionId: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const browser = useJobBrowser(sessionId);
  if (browser === undefined || session === undefined) return null;
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
        <JobBeside sessionId={sessionId} className="armada-mock-browser-window__job" />
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
