// What the panel beside a walk window's page holds, read off the query main loaded it with —
// `main/capture/layout.ts`'s `panelQuery`. Pure, so the rule is read without a window.

export type WalkSubject = { kind: "session"; id: string };

/** The Session this panel was opened on, or nothing where the query names none. */
export function subjectOf(search: string): WalkSubject | null {
  const id = new URLSearchParams(search).get("session");
  return id === null || id === "" ? null : { kind: "session", id };
}
