import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "../styles/browser-window.css";
import { HapticsProvider } from "@armada/components";
import { Boundary } from "@armada/shell";
import { NOTHING_YET } from "../../../shared/bridge";
import { CATALOGUE } from "../catalogue";
import { createFleetThemes } from "../fleet-themes";
import { JobBrowserWindow } from "../job-browser/JobBrowserWindow";
import { wireJobBrowsers } from "../job-browser/store";
import { WiredSessions } from "../sessions-wired";
import { withState } from "../state-feed";
import { BUILT_IN_SWATCHES, modSwatch } from "../swatches";
import { Themed } from "../theme";

// A Job's browser in a window of its own, a fifth renderer entry: a Bridge document with Bridge's preload, which main loads on
// `?session=` and closes when the page moves back to the side panel (`main/job-browsers.ts`). The page is main's view, placed over
// the pane's frame here as it is over the side panel's.

const THEMES = createFleetThemes(withState(window.armada), CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });
const SESSION = new URLSearchParams(window.location.search).get("session") ?? "";
if (window.armada.jobBrowser !== undefined) wireJobBrowsers(window.armada.jobBrowser);

function Root() {
  const [bridge, setBridge] = useState(NOTHING_YET.bridge);
  useEffect(() => {
    void window.armada.state().then((state) => state !== undefined && setBridge(state.bridge));
  }, []);
  return (
    <Boundary region="the Job browser's window" usable={false} bridge={bridge}>
      <HapticsProvider perform={window.armada.tap}>
        <Themed source={THEMES}>
          <WiredSessions>{SESSION === "" ? null : <JobBrowserWindow sessionId={SESSION} />}</WiredSessions>
        </Themed>
      </HapticsProvider>
    </Boundary>
  );
}

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}
