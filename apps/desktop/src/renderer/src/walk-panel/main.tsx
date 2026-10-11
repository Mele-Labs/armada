import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "../styles/walk-panel.css";
import { HapticsProvider } from "@armada/components";
import { Boundary } from "@armada/shell";
import { NOTHING_YET, type BridgeState } from "../../../shared/bridge";
import { CATALOGUE } from "../catalogue";
import { createFleetThemes } from "../fleet-themes";
import { WiredSessions } from "../sessions-wired";
import { BUILT_IN_SWATCHES, modSwatch } from "../swatches";
import { Themed } from "../theme";
import { subjectOf } from "./subject";
import { WalkJob } from "./WalkJob";
import { WalkSession } from "./WalkSession";

// The panel beside a walk window's page — `docs/practices/capture-window.md`, *The panel beside the
// page*. A third renderer entry, beside Bridge's and the bar's: a Bridge document with Bridge's
// preload, in a view main places beside the page and never over it.
//
// **It reads what Bridge's window reads**, published by main to every Bridge document, and draws it
// with the components Bridge's window draws it with. Its subject is the query main loaded it with.

const THEMES = createFleetThemes(window.armada, CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });
const SUBJECT = subjectOf(window.location.search);

function Panel() {
  const [state, setState] = useState<BridgeState>(NOTHING_YET);
  useEffect(() => {
    let live = true;
    void window.armada.state().then((read) => live && read !== undefined && setState(read));
    const off = window.armada.subscribe(setState);
    return () => {
      live = false;
      off();
    };
  }, []);
  if (SUBJECT === null) return null;
  return SUBJECT.kind === "session" ? (
    <WalkSession sessionId={SUBJECT.id} />
  ) : (
    <WalkJob jobId={SUBJECT.id} state={state} onRead={setState} />
  );
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
            <Panel />
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
