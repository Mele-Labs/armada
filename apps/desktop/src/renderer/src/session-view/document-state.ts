// Where a Job's document opens: at the place a sent note named. A press on that note's heading in the thread asks for it, and the layer,
// once drawn, reads it. `at` alone tells one ask from the next: the same note pressed twice is two visits.

import { useSyncExternalStore } from "react";
import type { DocumentPlace, DocumentTarget } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { showLayer } from "./layers";

export type DocumentVisit = { session: string; place: DocumentPlace; target: DocumentTarget; at: number };

let visit: DocumentVisit | undefined;
const hear = new Set<() => void>();

/** Opens the Job's document over the Job at the place a sent note named. */
export function openDocumentAt(session: string, place: DocumentPlace, target: DocumentTarget): void {
  visit = { session, place, target, at: Date.now() };
  for (const one of hear) one();
  showLayer(session, "document");
}

/** The layer has taken the visit, so the next time the document opens it opens as it was left. */
export function clearVisit(session: string): void {
  if (visit?.session !== session) return;
  visit = undefined;
  for (const one of hear) one();
}

export function useDocumentVisit(session: string): DocumentVisit | undefined {
  const now = useSyncExternalStore(
    (listener) => {
      hear.add(listener);
      return () => void hear.delete(listener);
    },
    () => visit,
  );
  return now?.session === session ? now : undefined;
}

/** The page a Job wrote as its document: the artifact that names its revisions. */
export function documentOf(session: Session): { id: string; title: string; revisions: readonly string[] } | undefined {
  const found = attachmentsOf(session, "artifact").find((one) => one.revisions !== undefined && one.revisions.length > 0);
  return found?.revisions === undefined ? undefined : { id: found.id, title: found.title, revisions: found.revisions };
}
