// The app on a scenario, mounted — for the mock's own page and for a browser test.
//
// **`App` itself, under the root boundary `main.tsx` puts it under**, so what a
// test asserts against is the window Bridge draws. `main.tsx` is not imported
// because it mounts itself on import.

import { StrictMode, useEffect } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { Boundary, LayoutSourceProvider } from "@armada/shell";
import { HapticsProvider, forgetAllKept } from "@armada/components";

import "../styles/index.css";
import "@armada/components/src/look";
import type { BridgeApi } from "../../../shared/api";
import { App } from "../App";
import { ClashesFrom } from "../clashes";
import { FleetEventsFrom, FleetEventsOver } from "../cockpit/fleet-events";
import { NumberedFrom } from "../numbered";
import { forgetFirsts } from "../run-book";
import { SessionsFrom } from "../sessions-draft";
import { WiredSessions } from "../sessions-wired";
import { AreasFrom } from "../cockpit/deck/areas-source";
import { forgetDeck } from "../cockpit/deck/store";
import { skipsMods } from "../theme-loader";
import { Themed } from "../theme";
import { mountAnnotating } from "./annotating";
import { eventsLater } from "./events-later";
import { fakeBridge, heldSessions } from "./fake";
import { alsoOnTimePassing } from "./time-passes";
import { MockFleetBuild } from "./fleet-build";
import type { FakeOptions } from "./fake";
import { scenarioNamed } from "./scenario";
import { mockLayout, NO_LAYOUT_MODS } from "./layout";
import { mockNumbered } from "./numbered";
import { PhoneSourceProvider } from "@armada/settings";
import { mockPhone, type PhoneMoment } from "./phone";
import { WalkJobPanel } from "./WalkJobPanel";
import { SleepSourceProvider } from "../sleep";
import { mockSleep, overnight } from "./sleep";
import { PlanUsageSourceProvider } from "../plan-usage";
import { DownshiftSourceProvider } from "../downshift";
import { createMockDownshift } from "./downshift";
import { createFleetPlanUsage } from "../fleet-plan-usage";
import { mockThemes } from "./themes";
import { installSystemMods } from "./system-mods";
import { provideKitModels } from "../kit-models";
import { createMockKitModels } from "./kit-models";
import { AccountFrom } from "./account";
import { SharedHomeFrom } from "./shared-home";
import { PrOwnersFrom } from "./pr-owners";
import { ReviewsFrom, createReviewStore } from "../pr-review/store";
import { SharedPanelFrom } from "../shared-panel/reads";
import { SharedPanelHost } from "../shared-panel/host";
import { SharedPanelOver } from "../shared-panel/wired";
import { reviewSourceOf } from "../pr-review/source";
import type { Scenario } from "./scenario";
import { withState } from "../state-feed";

/** What was mounted: the fake the window is talking to, and how to take it down. */
export type Mounted = {
  api: BridgeApi;
  scenario: Scenario;
  /**
   * Resolved once React has drawn the window and run its effects — which is
   * when the app's own key bindings exist.
   *
   * **`render` schedules the work rather than doing it**, so the line after
   * `mountApp` runs against an empty host. A press with a locator waits that
   * out on its own; a keystroke has none and nothing retries it, so it is
   * simply lost. Measured 23 Sep 2026 on #1592: under load `⌘J` reached
   * `window` with `#root` still empty, and Helm's dock never came up.
   */
  onScreen: Promise<void>;
  unmount: () => void;
};

/**
 * Says the window is up. **A sibling after `App` rather than a hook inside
 * it**: effects run in tree order, so this one runs after everything the app
 * mounted, this window's bindings included.
 */
/** The Phone card's moment for each `phone/…` scenario; any other scenario has access on and two phones paired. */
const PHONE_MOMENTS: Record<string, PhoneMoment> = { "phone/off": "off", "phone/port-taken": "port_taken", "phone/tailscale": "tailscale" };

function OnScreen({ say }: { say: () => void }) {
  useEffect(() => say(), [say]);
  return null;
}

/** The timeline the window reads: the scenario's fixed entries, or those it adds to as time passes. */
function EventsHere({ timeline, events, children }: { timeline: ReturnType<typeof eventsLater> | undefined; events: Parameters<typeof FleetEventsFrom>[0]["events"]; children: ReactNode }) {
  return timeline === undefined ? <FleetEventsFrom events={events}>{children}</FleetEventsFrom> : <FleetEventsOver source={timeline.source}>{children}</FleetEventsOver>;
}

/**
 * Sessions from the scenario's fixtures where it carries them, and otherwise **from the fake's own
 * capabilities**, the way a real window has them. A fake whose Fleet serves none leaves them off.
 */
function SessionsHere({ held, children }: { held: ReturnType<typeof heldSessions>; children: ReactNode }) {
  return held === undefined ? <WiredSessions>{children}</WiredSessions> : <SessionsFrom held={held}>{children}</SessionsFrom>;
}

/**
 * Install a fake `window.armada` on `scenario` and mount the app into `host`.
 * **Throws on a name no scenario has**, so a test never passes against a default.
 * `options.slices` mounts core and only those surfaces' fakes; the default is every one.
 */
export function mountApp(
  scenario: string | Scenario,
  host: HTMLElement,
  shared?: BridgeApi,
  options?: FakeOptions,
): Mounted {
  const chosen = typeof scenario === "string" ? scenarioNamed(scenario) : scenario;
  if (chosen === undefined) throw new Error(`no mock scenario named ${String(scenario)}`);
  // `shared` is a second window on the same main: both hear what either one's Fleet publishes.
  const api = shared ?? fakeBridge(chosen, options);
  // A window starts on Dark with the mods a machine starts with; a second window on the same main shares them.
  if (shared === undefined) mockThemes.reset();
  mockSleep.reset(chosen.name === "sleep-cockpit" ? overnight() : undefined);
  if (shared === undefined) mockLayout.reset();
  if (shared === undefined && chosen.name === "system-sidebar") installSystemMods();
  if (shared === undefined) provideKitModels(createMockKitModels());
  // Every window starts with the deck as it begins.
  if (shared === undefined) forgetDeck();
  // And with no repository having shown its first Job's card.
  if (shared === undefined) forgetFirsts();
  mockPhone.reset(PHONE_MOMENTS[chosen.name] ?? "paired");
  window.armada = api;
  const planUsage = createFleetPlanUsage(withState(api));
  // Only the scenario that plays the offer has one; every other window draws the limits as they were.
  const downshift = chosen.name === "plan-downshift" ? createMockDownshift(planUsage, api) : null;
  // The side panel reads from fixtures where the scenario carries them, built for this window so a write is gone with it, and
  // from the fake Fleet as a real window does everywhere else.
  const panelReads = chosen.draft?.sharedPanel?.();
  // The review surface starts as a window does: nothing fetched, nothing open, the Cockpit on its work.
  const reviews = chosen.draft?.prReview === undefined ? undefined : createReviewStore(reviewSourceOf(api));
  // And with nothing kept from an earlier pass: no saved Ask text, so a replay meets the same page the walk was written against.
  if (reviews !== undefined) forgetAllKept();
  // A scenario that adds to its timeline as time passes holds it here, and moves it when the fake's moment does.
  const timeline = chosen.draft?.eventsLater === undefined ? undefined : eventsLater(chosen.draft.events ?? [], chosen.draft.eventsLater);
  if (timeline !== undefined && shared === undefined) alsoOnTimePassing(timeline.advance);
  const root = createRoot(host);
  let say = (): void => undefined;
  const onScreen = new Promise<void>((resolve) => {
    say = resolve;
  });
  root.render(
    <StrictMode>
      <Boundary region="the window" usable={false} bridge={chosen.state.bridge}>
        {/* As `main.tsx` mounts it, so a test can read what a press asked the trackpad to play. */}
        <HapticsProvider perform={(pattern) => api.tap(pattern)}>
          <Themed source={mockThemes}>
            <LayoutSourceProvider value={skipsMods() ? NO_LAYOUT_MODS : mockLayout}>
              <MockFleetBuild scenario={chosen.name}>
                <PhoneSourceProvider value={mockPhone}>
                  <SleepSourceProvider value={mockSleep}>
                    <NumberedFrom held={mockNumbered}>
                      <PlanUsageSourceProvider value={planUsage}>
                        <DownshiftSourceProvider value={downshift}>
                          <EventsHere timeline={timeline} events={chosen.draft?.events}>
                            <SessionsHere held={heldSessions(api)}>
                              <ClashesFrom clashes={chosen.draft?.clashes}>
                                  <AreasFrom source={chosen.draft?.areas}>
                                    <AccountFrom account={chosen.draft?.account}>
                                      <SharedHomeFrom home={chosen.draft?.sharedHome}>
                                        <PrOwnersFrom owners={chosen.draft?.prOwners}>
                                          <ReviewsFrom store={reviews}>
                                            {panelReads === undefined ? (
                                              <SharedPanelOver api={withState(api)}>
                                                <App />
                                                <WalkJobPanel />
                                              </SharedPanelOver>
                                            ) : (
                                              <SharedPanelFrom reads={panelReads}>
                                                <App />
                                                <WalkJobPanel />
                                                <SharedPanelHost />
                                              </SharedPanelFrom>
                                            )}
                                          </ReviewsFrom>
                                        </PrOwnersFrom>
                                      </SharedHomeFrom>
                                    </AccountFrom>
                                  </AreasFrom>
                              </ClashesFrom>
                            </SessionsHere>
                          </EventsHere>
                        </DownshiftSourceProvider>
                      </PlanUsageSourceProvider>
                    </NumberedFrom>
                  </SleepSourceProvider>
                </PhoneSourceProvider>
              </MockFleetBuild>
            </LayoutSourceProvider>
          </Themed>
          <OnScreen say={say} />
        </HapticsProvider>
      </Boundary>
    </StrictMode>,
  );
  const unlayer = mountAnnotating(chosen.name);
  return {
    api,
    scenario: chosen,
    onScreen,
    unmount: () => {
      unlayer();
      root.unmount();
    },
  };
}
