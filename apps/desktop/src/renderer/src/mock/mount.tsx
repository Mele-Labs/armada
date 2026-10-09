// The app on a scenario, mounted — for the mock's own page and for a browser test.
//
// **`App` itself, under the root boundary `main.tsx` puts it under**, so what a
// test asserts against is the window Bridge draws. `main.tsx` is not imported
// because it mounts itself on import.

import { StrictMode, useEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { Boundary, LayoutSourceProvider } from "@armada/shell";
import { HapticsProvider } from "@armada/components";

import "../styles/index.css";
import type { BridgeApi } from "../../../shared/api";
import { App } from "../App";
import { DraftedFrom } from "../drafted";
import { SessionsFrom } from "../sessions-draft";
import { WiredSessions } from "../sessions-wired";
import { Themed } from "../theme";
import { mountAnnotating } from "./annotating";
import { fakeBridge, heldSessions, liveDraft } from "./fake";
import type { FakeOptions, LiveDraft } from "./fake";
import { scenarioNamed } from "./scenario";
import { mockLayout } from "./layout";
import { mockThemes } from "./themes";
import type { Scenario } from "./scenario";

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
function OnScreen({ say }: { say: () => void }) {
  useEffect(() => say(), [say]);
  return null;
}

/**
 * Sessions from the scenario's fixtures where it carries them, and otherwise **from the fake's own
 * capabilities**, the way a real window has them. A fake whose Fleet serves none leaves them off.
 */
function SessionsHere({ held, children }: { held: ReturnType<typeof heldSessions>; children: ReactNode }) {
  return held === undefined ? <WiredSessions>{children}</WiredSessions> : <SessionsFrom held={held}>{children}</SessionsFrom>;
}

/** The fixed draft a fake that holds none stands in with. */
const HELD = (draft: Scenario["draft"]): LiveDraft => ({ current: () => draft, subscribe: () => () => undefined });

/**
 * The app over the draft its fake holds as it stands — **live**, because a
 * plan edit the fake accepts changes the groups an arc board draws.
 */
function Drafted({ draft }: { draft: LiveDraft }) {
  const held = useSyncExternalStore(draft.subscribe, draft.current);
  return (
    <DraftedFrom held={held ?? {}}>
      <App {...(held === undefined ? {} : { draft: held })} />
    </DraftedFrom>
  );
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
  if (shared === undefined) mockLayout.reset();
  window.armada = api;
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
          {/* What this moment holds that Fleet cannot serve yet. The app's own
              mount provides none, so every field is absent there. The context
              is what a composer reads before a Job exists; the prop is what a
              Job's own boards read. */}
          <Themed source={mockThemes}>
            <LayoutSourceProvider value={mockLayout}>
              <SessionsHere held={heldSessions(api)}>
                <Drafted draft={liveDraft(api) ?? HELD(chosen.draft)} />
              </SessionsHere>
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
