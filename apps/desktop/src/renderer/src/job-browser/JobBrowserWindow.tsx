// A Job's browser in a window of its own: the page with the Job beside it, as the walk window draws its Job. Bridge's window is
// `../browser-window/main.tsx`; the mock draws the same inside its own frame (`../mock/JobBrowserWindows.tsx`).

import { JobBeside } from "./BrowserSide";
import { JobBrowserPane } from "./JobBrowserPane";
import { useJobBrowser } from "./store";

export function JobBrowserWindow({ sessionId }: { sessionId: string }) {
  const browser = useJobBrowser(sessionId);
  if (browser === undefined) return null;
  return (
    <div className="job-browser-window">
      <div className="job-browser-window__page">
        <JobBrowserPane sessionId={sessionId} browser={browser} placed="window" />
      </div>
      <JobBeside sessionId={sessionId} className="job-browser-window__job" />
    </div>
  );
}
