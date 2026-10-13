// Where a Session's pull request opens, off `links.openPullRequestsIn`. Apart from `App.tsx`, which is at its length.

import type { SettingsList } from "@armada/protocol";

export type PullRequestPlace = "armada" | "browser";

/** `browser` only where Fleet has said so; unread, absent or any other value is Armada's own panel. */
export function pullRequestPlaceOf(settings: SettingsList | null | undefined): PullRequestPlace {
  return settings?.settings.find((one) => one.key === "links.openPullRequestsIn")?.value === "browser" ? "browser" : "armada";
}
