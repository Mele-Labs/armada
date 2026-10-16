// The ideas saved to start later, as the deck's list draws and walks them: at its head, above the bands, narrowed to the picked
// repository as the rest of the list is. Each row's key is its own, so the cursor and the keys can rest on it.

import type { RepositorySummary } from "@armada/protocol";
import type { QueuedIdea } from "@armada/screens/src/draft/sessions";

export const queuedKey = (id: string) => `queued:${id}`;

export const queuedIn = (ideas: readonly QueuedIdea[], picked: RepositorySummary | null): readonly QueuedIdea[] =>
  picked === null ? ideas : ideas.filter((one) => one.root === picked.root);
