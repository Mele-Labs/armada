// What the panel beside a walk window's page holds, read off the query main loaded it with —
// `main/capture/layout.ts`'s `panelQuery`. Pure, so the rule is read without a window.

export type WalkSubject = { kind: "session" | "job"; id: string };

/** The Session or the Job this panel was opened on. **Exactly one**: a query naming both, or neither, holds nothing. */
export function subjectOf(search: string): WalkSubject | null {
  const query = new URLSearchParams(search);
  const session = query.get("session");
  const job = query.get("job");
  if ((session === null) === (job === null)) return null;
  const id = session ?? job ?? "";
  if (id === "") return null;
  return { kind: session !== null ? "session" : "job", id };
}
