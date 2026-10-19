import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "../styles/walk-panel.css";
import { HapticsProvider } from "@armada/components";
import { Boundary } from "@armada/shell";
import { NOTHING_YET } from "../../../shared/bridge";
import { CATALOGUE } from "../catalogue";
import { createFleetThemes } from "../fleet-themes";
import { WiredSessions } from "../sessions-wired";
import { BUILT_IN_SWATCHES, modSwatch } from "../swatches";
import { Themed } from "../theme";
import { Panel } from "./Panel";
import { useWalkSubject } from "./subject";
import { withState } from "../state-feed";

// The panel beside a walk window's page — `docs/practices/capture-window.md`, *The panel beside the
// page*. A third renderer entry, beside Bridge's and the bar's: a Bridge document with Bridge's
// preload, in a view main places beside the page and never over it.
//
// **It reads what Bridge's window reads**, published by main to every Bridge document, and draws it
// with the components Bridge's window draws it with. Its subject is the Job of the walk on screen, which main pushes as the window moves between walks.

const THEMES = createFleetThemes(withState(window.armada), CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });
/** The Job of the walk on screen, followed as main moves the window between walks. */
function Followed() {
  const id = useWalkSubject();
  return <Panel subject={id === undefined ? null : { kind: "session", id }} />;
}

function Root() {
  const [bridge, setBridge] = useState(NOTHING_YET.bridge);
  useEffect(() => {
    void window.armada.state().then((state) => state !== undefined && setBridge(state.bridge));
  }, []);
  return (
    <Boundary region="the walk panel" usable={false} bridge={bridge}>
      <HapticsProvider perform={window.armada.tap}>
        <Themed source={THEMES}>
          <WiredSessions>
            <Followed />
          </WiredSessions>
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
