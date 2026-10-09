// The theme this viewer chose, kept across launches in the window's own storage. A per-viewer
// convenience: unreadable or absent storage is Dark, and a stored id the source no longer offers
// is Dark too, which `setActive` already does.

import type { ModsSource } from "@armada/settings";

const KEY = "armada.theme";

export function remembered<T extends ModsSource>(source: T, storage: () => Storage = () => window.localStorage): T {
  try {
    const kept = storage().getItem(KEY);
    if (kept !== null) source.setActive(kept);
  } catch {
    // Storage is blocked or cleared: Dark.
  }
  source.subscribe(() => {
    try {
      storage().setItem(KEY, source.get().active);
    } catch {
      // As above; the choice holds until the window closes.
    }
  });
  return source;
}
