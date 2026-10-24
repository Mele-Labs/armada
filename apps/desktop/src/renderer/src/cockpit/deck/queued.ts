// The ideas saved to start later, as the deck's list draws and walks them: in their repository's group, above its Sessions,
// or at the head of the flat list where there are no groups. Each row's key is its own, so the cursor and the keys can rest on it.

import type { RepositorySummary } from "@armada/protocol";
import type { QueuedIdea } from "@armada/screens/src/draft/sessions";

import { bandsOf, type Banding } from "./bands";
import type { Subject } from "./ship";

export const queuedKey = (id: string) => `queued:${id}`;

export const queuedIn = (ideas: readonly QueuedIdea[], picked: RepositorySummary | null): readonly QueuedIdea[] =>
  picked === null ? ideas : ideas.filter((one) => one.root === picked.root);

/** The keys of the list in the order it draws them: each group's ideas before its subjects, or the ideas and then the subjects where there are no groups. */
export function walkOrder(listed: readonly Subject[], banding: readonly Banding[] | undefined, ideas: readonly QueuedIdea[], picked: RepositorySummary | null): string[] {
  if (banding === undefined) return [...queuedIn(ideas, picked).map((one) => queuedKey(one.id)), ...listed.map((one) => one.key)];
  return bandsOf(listed, banding, queuedIn(ideas, picked)).flatMap((band) => [...band.ideas.map((one) => queuedKey(one.id)), ...band.subjects.map((one) => one.key)]);
}
