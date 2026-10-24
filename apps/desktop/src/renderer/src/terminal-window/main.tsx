import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "../styles/terminal-window.css";
import { Boundary } from "@armada/shell";
import { NOTHING_YET } from "../../../shared/bridge";
import { CATALOGUE } from "../catalogue";
import { createFleetThemes } from "../fleet-themes";
import { withState } from "../state-feed";
import { BUILT_IN_SWATCHES, modSwatch } from "../swatches";
import { Themed } from "../theme";
import { TerminalWindow } from "./TerminalWindow";

// A Job's shell in a window of its own, a fourth renderer entry beside Bridge's, the bar's and the walk panel's: a Bridge
// document with Bridge's preload, which main loads on `?session=` and closes when the shell moves back (`main/terminal-windows.ts`).

const THEMES = createFleetThemes(withState(window.armada), CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });
const SESSION = new URLSearchParams(window.location.search).get("session") ?? "";

function Root() {
  const [bridge, setBridge] = useState(NOTHING_YET.bridge);
  useEffect(() => {
    void window.armada.state().then((state) => state !== undefined && setBridge(state.bridge));
  }, []);
  return (
    <Boundary region="the terminal window" usable={false} bridge={bridge}>
      <Themed source={THEMES}>{SESSION === "" ? null : <TerminalWindow api={window.armada} sessionId={SESSION} />}</Themed>
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
