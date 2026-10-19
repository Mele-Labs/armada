// What the panel beside a walk window's page holds: the Job of the walk on screen, which main pushes as
// the window moves between walks (`walkPanel.subject`), first read off the query main loaded it with —
// `main/capture/layout.ts`'s `panelQuery`. `subjectOf` is pure, so the rule is read without a window.

import { useEffect, useState } from "react";

export type WalkSubject = { kind: "session"; id: string };

/** The Session this panel was opened on, or nothing where the query names none. */
export function subjectOf(search: string): WalkSubject | null {
  const id = new URLSearchParams(search).get("session");
  return id === null || id === "" ? null : { kind: "session", id };
}

/** The Job of the walk on screen, live. `undefined`: a pull request's walk, which has no Job. */
export function useWalkSubject(): string | undefined {
  const [sessionId, setSessionId] = useState<string | undefined>(() => subjectOf(window.location.search)?.id);
  useEffect(() => window.armada.walkPanel.subject(setSessionId), []);
  return sessionId;
}
