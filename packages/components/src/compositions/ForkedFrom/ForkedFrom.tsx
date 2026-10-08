import type { ReactNode } from "react";

/**
 * The conversation a forked Session began as a copy of, one quiet row at the head of its thread.
 * **Closed until pressed.** Its rows are the old Session's own, read-only, and new messages follow
 * below it. `onOpen` is called as it opens, so the host can read a thread it does not hold yet.
 */
export function ForkedFrom({ title, onOpen, children }: { title: string; onOpen?: () => void; children: ReactNode }) {
  return (
    <details className="armada-forked-from" aria-label={`Forked from ${title}`} onToggle={(event) => event.currentTarget.open && onOpen?.()}>
      <summary className="armada-forked-from__head" role="button">Forked from {title}</summary>
      <div className="armada-forked-from__rows">{children}</div>
    </details>
  );
}
