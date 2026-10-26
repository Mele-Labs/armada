// The window's own effects, out of `App.tsx`, which is at the length the gate refuses: where a pressed notification lands and the
// call an alert asked the Cockpit for. Each is App's, and each says why it is the way it is.

import { useEffect, useRef, useState } from "react";
import type { RepositorySummary, SettingsList } from "@armada/protocol";
import type { Summons } from "../../shared/bridge";
import type { PullLink } from "../../shared/pull-link";
import { askToOpenSession } from "./open-session";
import { openLinked } from "./pr-review/link";
import { useReviewStore } from "./pr-review/store";
import { installUiSounds, setUiSounds } from "./ui-sounds";

/**
 * **Where a pressed notification says to go, and it always goes somewhere.** A press that raised
 * the window and left it on whatever it was last showing is a press that did nothing, which is
 * the one outcome that teaches somebody to stop pressing them.
 *
 * A Session opens on the Cockpit. Anything else lands on the Cockpit itself, the overlays down
 * first: the press asked for the work, not for the composer that happened to be up.
 */
export function useSummoned(go: () => void, then: (act: NonNullable<Summons["then"]>, sessionId: string) => void): void {
  // The latest, because the listener is installed once.
  const latest = useRef({ go, then });
  latest.current = { go, then };
  useEffect(
    () =>
      window.armada.onSummoned((to) => {
        if (to.sessionId === undefined) return latest.current.go();
        // A walk window's panel asked for more than the Session: a Check or its Loadout opens over it, a surface or a link replaces it.
        if (to.then === undefined || to.then.kind === "check" || to.then.kind === "loadout") askToOpenSession(to.sessionId);
        if (to.then !== undefined) latest.current.then(to.then, to.sessionId);
      }),
    [],
  );
}

/**
 * Where a press asked the Cockpit to land: one call (an in-app alert's). **A token rather than a call**: the press may have
 * arrived over another page, so the Cockpit is not mounted yet, and it acts once it is. `at` is what makes a second press of the same
 * call land, and the taker clears the token so it is not acted on twice.
 */
export function useLandings() {
  const [bringing, setBringing] = useState<{ key: string; at: number } | null>(null);
  return {
    bringing,
    bring: (key: string) => setBringing({ key, at: Date.now() }),
    brought: () => setBringing(null),
  };
}

/** The quiet sounds, installed once for the window and on unless `features.uiSounds` is false. Absent counts as on. */
export function useUiSounds(settings: SettingsList | null): void {
  useEffect(() => installUiSounds(), []);
  const on = settings?.settings.find((one) => one.key === "features.uiSounds")?.value !== false;
  useEffect(() => setUiSounds(on), [on]);
}

/**
 * A link to a pull request opened from outside (`armada://pull/...`, behind `features.deepLinks`): Reviews opens on its page, `go` having taken the window there.
 * **Held until a repository is drawn**, since a link that launched the app arrives before the window has read which ones Fleet serves. A refusal is `say`'s, never a dead press.
 */
export function usePullLinks(go: () => void, repositories: readonly RepositorySummary[], say: (words: string) => void): (link: PullLink) => void {
  const store = useReviewStore();
  const [waiting, setWaiting] = useState<PullLink | undefined>(undefined);
  const latest = useRef({ go, say });
  latest.current = { go, say };
  useEffect(() => (store === undefined ? undefined : window.armada.onPullLink(setWaiting)), [store]);
  useEffect(() => {
    if (waiting === undefined || store === undefined || repositories.length === 0) return;
    setWaiting(undefined);
    latest.current.go();
    void openLinked(store, repositories, waiting).then((refused) => refused === undefined || latest.current.say(refused));
  }, [waiting, store, repositories]);
  return setWaiting;
}
