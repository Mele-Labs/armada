import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "./styles/index.css";
import "@armada/components/src/look";
import { watchStill } from "./still";

watchStill();

// Dimmed behind a walk window that has focus — `styles/index.css`, `data-walking`.
window.armada.onWalkFocus((focused) => {
  document.documentElement.toggleAttribute("data-walking", focused);
});
import type { BridgeIdentity } from "@armada/protocol";
import { createPhoneGatewaySource, PhoneSourceProvider } from "@armada/settings";
import { App, WAITING } from "./App";
import { TaughtSourceProvider } from "@armada/settings";
import { Boundary, LayoutSourceProvider, createLayoutSource } from "@armada/shell";
import { HapticsProvider } from "@armada/components";
import { WiredNumbered } from "./numbered-fleet";
import { WiredSessions } from "./sessions-wired";
import { SleepSourceProvider } from "./sleep";
import { createFleetSleep } from "./fleet-sleep";
import { createFleetPlanUsage } from "./fleet-plan-usage";
import { createFleetTimeline } from "./fleet-timeline";
import { canReport } from "./report";
import { withState } from "./state-feed";
import { FleetEventsOver } from "./cockpit/fleet-events";
import { PlanUsageSourceProvider } from "./plan-usage";
import { createFleetDownshift } from "./fleet-downshift";
import { DownshiftSourceProvider } from "./downshift";
import { createFleetClashes } from "./fleet-clashes";
import { ReviewsFrom, createReviewStore } from "./pr-review/store";
import { IssuesFrom, createIssuesStore } from "./issues-tab/store";
import { issuesSourceOf } from "./issues-tab/source";
import { WiredSharedPanel } from "./shared-panel/wired";
import { reviewSourceOf } from "./pr-review/source";
import { createFleetTerritory } from "./fleet-territory";
import { createFleetAccount } from "./fleet-account";
import { createFleetSharedHome } from "./fleet-shared-home";
import { AccountFromFleet } from "./account-fleet";
import { ClashesFromFleet } from "./clashes";
import { TerritoryFromFleet } from "./territory";
import { CATALOGUE } from "./catalogue";
import { Themed } from "./theme";
import { createFleetThemes } from "./fleet-themes";
import { createFleetLayout } from "./fleet-layout";
import { createFleetTaught, NO_TAUGHT } from "./fleet-taught";
import { NO_TWEAKS, createFleetTweaks } from "./fleet-tweaks";
import { provideTeaching } from "./teach";
import { TweakSourceProvider, TweaksApplied } from "./tweaks";
import { wireFleetKeymap } from "./fleet-keymap";
import { wireFleetTimeFormat } from "./fleet-time-format";
import { skipsMods } from "./theme-loader";
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
const THEMES = createFleetThemes(withState(window.armada), CATALOGUE, { builtIn: BUILT_IN_SWATCHES, ofMod: modSwatch });

/** The layout this window draws: Fleet's layout mods and the owner's saved choices, or the shipped layout in safe mode. */
const LAYOUT = skipsMods() ? createLayoutSource() : createFleetLayout(withState(window.armada));
/** The theme tweaks laid over whichever theme is chosen, and the words, Fleet and tracker mods the Mods list shows, over Fleet. Safe mode reads neither. */
const TWEAKS = skipsMods() ? NO_TWEAKS : createFleetTweaks(withState(window.armada));
const TAUGHT = skipsMods() ? NO_TAUGHT : createFleetTaught(withState(window.armada));
/** Teach is offered in every window: Fleet reads what is said and keeps it (`readTeaching`, `saveTeaching`). */
provideTeaching(true);
/** The owner's key bindings, over Fleet. Not a mod, so safe mode keeps them. */
wireFleetKeymap(withState(window.armada));
/** How the owner has a time drawn, over Fleet. Not a mod, so safe mode keeps it. */
wireFleetTimeFormat(withState(window.armada));
/** Settings → Phone, over the Gateway through the main process. Reads nothing until Settings draws it. */
const PHONE = createPhoneGatewaySource((request) => window.armada.phone(request));
/** The moon in the title row and the night's decisions as Cockpit calls, over Fleet. */
const SLEEP = createFleetSleep(withState(window.armada));
/** The plan's windows in the title row, over Fleet. */
const PLAN_USAGE = createFleetPlanUsage(withState(window.armada));
/** The default model's offered step down on the weekly meter, over Fleet. */
const DOWNSHIFT = createFleetDownshift(withState(window.armada));
const TIMELINE = createFleetTimeline(withState(window.armada));
/** The mark on a row whose changes would conflict with another agent's, over Fleet. */
const CLASHES = createFleetClashes(withState(window.armada));
/** The directories the deck's map is drawn in and what each slot's holder has changed, over Fleet. */
const TERRITORY = createFleetTerritory(withState(window.armada));
/** The pull requests waiting on him, read from the forge by Fleet when the Reviews tab opens, and what an agent says of them, as runs. Natural Search and the Walk are runs too. */
const REVIEWS = createReviewStore(reviewSourceOf(window.armada));
/** A repository's issues and the queries saved for its tracker, read from the tracker by Fleet when the Issues tab opens. */
const ISSUES = createIssuesStore(issuesSourceOf(window.armada));
/** The title menu's account entry: the profiles, the sign-in and What syncs, over Fleet. A kind switched reads the home again. */
const ACCOUNT = createFleetAccount(withState(window.armada), undefined, { kindSwitched: (login) => HOME.readAgain(login) });
/** Kit and the Theme rows against the chosen profile's shared home, over Fleet. */
const HOME = createFleetSharedHome(window.armada, ACCOUNT);

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
          <LayoutSourceProvider value={LAYOUT}>
            <TaughtSourceProvider value={TAUGHT}>
            <TweakSourceProvider source={TWEAKS}>
            <PhoneSourceProvider value={PHONE}>
              <SleepSourceProvider value={SLEEP}>
                <WiredNumbered>
                  <PlanUsageSourceProvider value={PLAN_USAGE}>
                    <DownshiftSourceProvider value={DOWNSHIFT}>
                      <WiredSessions events={TIMELINE}>
                        <ClashesFromFleet source={CLASHES}>
                          <TerritoryFromFleet source={TERRITORY}>
                            <AccountFromFleet source={ACCOUNT} home={HOME}>
                              <FleetEventsOver source={TIMELINE}>
                                <ReviewsFrom store={REVIEWS}>
                                  <IssuesFrom store={ISSUES}>
                                    <WiredSharedPanel>
                                      <App />
                                      <TweaksApplied />
                                    </WiredSharedPanel>
                                  </IssuesFrom>
                                </ReviewsFrom>
                              </FleetEventsOver>
                            </AccountFromFleet>
                          </TerritoryFromFleet>
                        </ClashesFromFleet>
                      </WiredSessions>
                    </DownshiftSourceProvider>
                  </PlanUsageSourceProvider>
                </WiredNumbered>
              </SleepSourceProvider>
            </PhoneSourceProvider>
            </TweakSourceProvider>
            </TaughtSourceProvider>
          </LayoutSourceProvider>
        </Themed>
      </HapticsProvider>
    </Boundary>
  );
}

// Dev only, #1226: the annotation layer, when main exposed its save path or a vite dev server serves this page.
if (canReport()) {
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
