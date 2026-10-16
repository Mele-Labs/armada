// The ideas a person saved to start later, kept on this machine across restarts: words, repository, model and effort, oldest first.
//
// **`localStorage`, not a Fleet preference**, for `remembered-views.ts`' reason: Fleet's preference set is closed and an idea is
// how this person works, not a fact Fleet holds. Every window of Bridge shares the one store, and the browser tells the others
// when one writes (`storage`), so a queue in one window is in the next.

import type { QueuedIdea } from "@armada/screens/src/draft/sessions";

export const QUEUED_KEY = "armada.bridge.queued-ideas";

const text = (value: unknown): value is string => typeof value === "string" && value !== "";

/** What was stored, an idea at a time: one that is not whole is left out rather than failing the rest. */
export function readQueuedIdeas(): readonly QueuedIdea[] {
  try {
    const raw = JSON.parse(window.localStorage.getItem(QUEUED_KEY) ?? "[]") as unknown;
    if (!Array.isArray(raw)) return [];
    return raw.flatMap((one: Record<string, unknown> | null): QueuedIdea[] =>
      one !== null && typeof one === "object" && text(one.id) && text(one.text) && text(one.root) && text(one.repository)
        ? [{ id: one.id, text: one.text, root: one.root, repository: one.repository, ...(text(one.model) ? { model: one.model } : {}), ...(text(one.effort) ? { effort: one.effort } : {}) }]
        : [],
    );
  } catch {
    return [];
  }
}

/** A blocked store leaves the queue to this window's memory until it closes. */
export function writeQueuedIdeas(ideas: readonly QueuedIdea[]): void {
  try {
    window.localStorage.setItem(QUEUED_KEY, JSON.stringify(ideas));
  } catch {
    // Private window or blocked storage: the queue is forgotten between launches.
  }
}

/** Calls `on` when another window changed the queue. Returns the way to stop. */
export function hearQueuedIdeas(on: () => void): () => void {
  const heard = (event: StorageEvent) => event.key === QUEUED_KEY && on();
  window.addEventListener("storage", heard);
  return () => window.removeEventListener("storage", heard);
}

let made = 0;

/** A key for a new idea: unique across windows and launches. */
export const newIdeaId = (): string => `q-${Date.now().toString(36)}-${(made += 1).toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
