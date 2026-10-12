/**
 * One quiet row above the thread of a Session another Session's agent started, naming that Session.
 * **A press opens it.** Unlike a fork, nothing was copied, so there is no conversation to fold out.
 */
export function StartedBy({ title, onOpen }: { title: string; onOpen: () => void }) {
  return (
    <button type="button" className="armada-started-by" onClick={onOpen}>
      Started by {title}
    </button>
  );
}
