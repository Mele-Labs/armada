import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "./styles/index.css";

// Dimmed behind a walk window that has focus — `styles/index.css`, `data-walking`.
window.armada.onWalkFocus((focused) => {
  document.documentElement.toggleAttribute("data-walking", focused);
});
import type { BridgeIdentity } from "@armada/protocol";
import { createPhoneGatewaySource, PhoneSourceProvider } from "@armada/settings";
import { App, WAITING } from "./App";
import { Boundary } from "@armada/shell";
import { HapticsProvider } from "@armada/components";
import { WiredSessions } from "./sessions-wired";
import { CATALOGUE } from "./catalogue";
import { Themed } from "./theme";
import { createFleetThemes } from "./fleet-themes";
import { BUILT_IN_SWATCHES, modSwatch } from "./swatches";

// Bridge's renderer entry point. No Node, no `require`, no socket — everything
// it draws arrives through the preload from the one connection in the main
// process.
//
// **The root boundary is outside `App` on purpose.** The failure that started
// this was a title bar over an empty window, which means the whole tree went —
// a boundary inside `App` cannot catch what `App` itself throws. `Root` holds
// nothing but the path Bridge's log is at, so the fallback can still name it
// when everything under it has gone.

/** The themes this window offers: Fleet's mods and the saved preference, and the catalogue Bridge ships. */
const THEMES = createFleetThemes(window.armada, CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });

/** Settings → Phone, over the Gateway through the main process. Reads nothing until Settings draws it. */
const PHONE = createPhoneGatewaySource((request) => window.armada.phone(request));

/**
 * Who Bridge is, read once. The only state above the boundary, and the least
 * that can be: everything this component can throw on, the fallback needs.
 */
function Root() {
  const [bridge, setBridge] = useState<BridgeIdentity>(WAITING.bridge);

  useEffect(() => {
    void window.armada.state().then((state) => setBridge(state.bridge));
  }, []);

  return (
    <Boundary
      region="the window"
      usable={false}
      bridge={bridge}
    >
      {/* Here rather than in `App`, which the mock mounts: only a real Bridge reaches a trackpad. */}
      <HapticsProvider perform={window.armada.tap}>
        <Themed source={THEMES}>
          <PhoneSourceProvider value={PHONE}>
            <WiredSessions>
              <App />
            </WiredSessions>
          </PhoneSourceProvider>
        </Themed>
      </HapticsProvider>
    </Boundary>
  );
}

// Dev only, #1226: the annotation layer, when main exposed its save path or a vite dev server serves this page.
if (window.armadaDev !== undefined || import.meta.env.DEV) {
  void import("./annotate/mount").then(({ mount }) => mount());
}

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}
