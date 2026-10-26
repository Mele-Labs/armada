// The app on a scenario, mounted — for the mock's own page and for a browser test.
//
// **`App` itself, under the root boundary `main.tsx` puts it under**, so what a
// test asserts against is the window Bridge draws. `main.tsx` is not imported
// because it mounts itself on import.

import { StrictMode, useEffect } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { Boundary, LayoutSourceProvider } from "@armada/shell";
import { HapticsProvider, forgetAllKept, writeKept } from "@armada/components";

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
import { RunsFrom } from "../cockpit/runs/source";
import { createMockRuns } from "./runs";
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
import { PhoneSourceProvider, TaughtSourceProvider } from "@armada/settings";
import { mockPhone, type PhoneMoment } from "./phone";
import { TerminalWindows } from "./TerminalWindows";
import { WalkJobPanel } from "./WalkJobPanel";
import { SleepSourceProvider } from "../sleep";
import { mockSleep, overnight } from "./sleep";
import { PlanUsageSourceProvider } from "../plan-usage";
import { AgentSignInSourceProvider } from "../agent-sign-in";
import { DownshiftSourceProvider } from "../downshift";
import { createMockAgentSignIn } from "./agent-sign-in";
import { createMockDownshift } from "./downshift";
import { createFleetPlanUsage } from "../fleet-plan-usage";
import { mockThemes, promoteThemesWith } from "./themes";
import { installTrackerMods, triable } from "./trackers";
import { installSystemMods, installTravelMods } from "./system-mods";
import { provideKitModels } from "../kit-models";
import { createMockKitModels } from "./kit-models";
import { provideTeaching } from "../teach";
import { createFleetTaught, NO_TAUGHT } from "../fleet-taught";
import { NO_TWEAKS, createFleetTweaks } from "../fleet-tweaks";
import { TweakSourceProvider, TweaksApplied } from "../tweaks";
import { seedTaught } from "./teach";
import { repository } from "@armada/screens/src/fixtures/repository";
import { AccountFrom } from "./account";
import { SharedHomeFrom } from "./shared-home";
import { PrOwnersFrom } from "./pr-owners";
import { KEPT_LISTS, ReviewsFrom, createReviewStore } from "../pr-review/store";
import { keptReading } from "./review-forge";
import { SharedPanelFrom } from "../shared-panel/reads";
import { SharedPanelHost } from "../shared-panel/host";
import { SharedPanelOver } from "../shared-panel/wired";
import { reviewSourceOf } from "../pr-review/source";
import { IssuesFrom, createIssuesStore } from "../issues-tab/store";
import { issuesTabSource } from "./slices/issues";
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
const PHONE_MOMENTS: Record<string, PhoneMoment> = {
  "phone/off": "off",
  "phone/port-taken": "port_taken",
  "phone/tailscale": "tailscale",
  "phone/set-up": "set_up",
  "phone/named": "named",
};

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
  // A Promote opens a pull request only where the scenario's `behaves` says so, which building the fake runs.
  if (shared === undefined) promoteThemesWith(undefined);
  // `shared` is a second window on the same main: both hear what either one's Fleet publishes.
  const api = shared ?? fakeBridge(chosen, options);
  // A window starts on Dark with the mods a machine starts with; a second window on the same main shares them.
  if (shared === undefined) mockThemes.reset();
  mockSleep.reset(chosen.name === "sleep-cockpit" ? overnight() : undefined);
  if (shared === undefined) mockLayout.reset();
  if (shared === undefined && chosen.name === "system-sidebar") installSystemMods();
  if (shared === undefined && chosen.name === "mods-travel") installTravelMods();
  if (shared === undefined && chosen.name === "mods-trackers") installTrackerMods();
  if (shared === undefined) provideKitModels(createMockKitModels());
  // Teach is read only in the scenario that plays it, which starts with nothing taught.
  if (shared === undefined) {
    provideTeaching(chosen.name === "teach-anything" || chosen.name === "teach-mods");
  }
  // `teach-mods` starts with the Jira key and the notice already taught.
  if (shared === undefined && chosen.name === "teach-mods") seedTaught(repository().root);
  // Every window starts with the deck as it begins.
  if (shared === undefined) forgetDeck();
  // And with no repository having shown its first Job's card.
  if (shared === undefined) forgetFirsts();
  mockPhone.reset(PHONE_MOMENTS[chosen.name] ?? "paired");
  window.armada = api;
  const planUsage = createFleetPlanUsage(withState(api));
  // The mods the lessons made, over the fake Fleet's mod list as a real window reads them; safe mode reads none.
  const taught = skipsMods() ? NO_TAUGHT : chosen.name === "mods-trackers" ? triable(createFleetTaught(withState(api))) : createFleetTaught(withState(api));
  const tweaks = skipsMods() ? NO_TWEAKS : createFleetTweaks(withState(api));
  // Only the scenario that plays the offer has one; every other window draws the limits as they were.
  const downshift = chosen.name === "plan-downshift" ? createMockDownshift(planUsage, api) : null;
  // Likewise the Fleet-wide sign-in alert: only the scenario that plays it starts signed out.
  const agentSignIn = chosen.name === "agent-signed-out" ? createMockAgentSignIn() : null;
  // The side panel reads from fixtures where the scenario carries them, built for this window so a write is gone with it, and
  // from the fake Fleet as a real window does everywhere else.
  const panelReads = chosen.draft?.sharedPanel?.();
  // The review surface starts as a window does: nothing fetched, nothing open, the Cockpit on its work.
  // And with nothing kept from an earlier pass, so a replay meets the same page the walk was written against: no saved Ask
  // text. Cleared before the store is made, since the store draws what is kept at once; a scenario that plays a relaunch keeps a reading of the lists.
  const seed = chosen.draft?.prReview;
  if (seed !== undefined) {
    forgetAllKept();
    const last = keptReading(seed);
    if (last !== undefined) writeKept(KEPT_LISTS, last);
  }
  const reviews = seed === undefined ? undefined : createReviewStore(reviewSourceOf(api));
  // The Issues tab starts as a window does, with no query chosen and no draft kept from an earlier pass.
  const tracked = chosen.draft?.issuesTab;
  if (tracked !== undefined && seed === undefined) forgetAllKept();
  const issues = tracked === undefined ? undefined : createIssuesStore(issuesTabSource(tracked));
  // A scenario that adds to its timeline as time passes holds it here, and moves it when the fake's moment does.
  const timeline = chosen.draft?.eventsLater === undefined ? undefined : eventsLater(chosen.draft.events ?? [], chosen.draft.eventsLater);
  if (timeline !== undefined && shared === undefined) alsoOnTimePassing(timeline.advance);
  // Each window starts with its repositories' runs as the scenario seeds them and nothing running.
  const runs = chosen.draft?.runs === undefined ? undefined : createMockRuns(chosen.draft.runs);
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
            <TaughtSourceProvider value={taught}>
            <TweakSourceProvider source={tweaks}>
            <LayoutSourceProvider value={skipsMods() ? NO_LAYOUT_MODS : mockLayout}>
              <MockFleetBuild scenario={chosen.name}>
                <PhoneSourceProvider value={mockPhone}>
                  <SleepSourceProvider value={mockSleep}>
                    <NumberedFrom held={mockNumbered}>
                      <PlanUsageSourceProvider value={planUsage}>
                        <AgentSignInSourceProvider value={agentSignIn}>
                        <DownshiftSourceProvider value={downshift}>
                          <EventsHere timeline={timeline} events={chosen.draft?.events}>
                            <SessionsHere held={heldSessions(api)}>
                              <ClashesFrom clashes={chosen.draft?.clashes}>
                                  <RunsFrom source={runs}>
                                  <AreasFrom source={chosen.draft?.areas}>
                                    <AccountFrom account={chosen.draft?.account}>
                                      <SharedHomeFrom home={chosen.draft?.sharedHome}>
                                        <PrOwnersFrom owners={chosen.draft?.prOwners}>
                                          <ReviewsFrom store={reviews}>
                                          <IssuesFrom store={issues}>
                                            {panelReads === undefined ? (
                                              <SharedPanelOver api={withState(api)} testsReaching={chosen.draft?.testsReaching}>
                                                <App />
                                                <WalkJobPanel />
                                                <TerminalWindows api={api} />
                                                <TweaksApplied />
                                              </SharedPanelOver>
                                            ) : (
                                              <SharedPanelFrom reads={panelReads}>
                                                <App />
                                                <WalkJobPanel />
                                                <TerminalWindows api={api} />
                                                <TweaksApplied />
                                                <SharedPanelHost />
                                              </SharedPanelFrom>
                                            )}
                                          </IssuesFrom>
                                          </ReviewsFrom>
                                        </PrOwnersFrom>
                                      </SharedHomeFrom>
                                    </AccountFrom>
                                  </AreasFrom>
                                  </RunsFrom>
                              </ClashesFrom>
                            </SessionsHere>
                          </EventsHere>
                        </DownshiftSourceProvider>
                        </AgentSignInSourceProvider>
                      </PlanUsageSourceProvider>
                    </NumberedFrom>
                  </SleepSourceProvider>
                </PhoneSourceProvider>
              </MockFleetBuild>
            </LayoutSourceProvider>
            </TweakSourceProvider>
            </TaughtSourceProvider>
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
