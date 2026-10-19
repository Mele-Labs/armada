import { RotateCw, X } from "lucide-react";
import type { KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";

/**
 * A hand off that did not start, as a band on the composer in the failed hue, drawn as the Needs you band is. Its body is always up: the
 * reason Fleet recorded, and the two ways on, Retry (r) and Dispatch job to fix (d), the failure toast's own act. A press on the band puts it away.
 * The keys work while focus is inside the band.
 */
export function HandoffBand({ reason, onRetry, onDispatch, onDismiss }: { reason?: string; onRetry?: () => void; onDispatch?: () => void; onDismiss: () => void }) {
  const keys = (event: KeyboardEvent) => {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "r" && onRetry !== undefined) {
      event.preventDefault();
      onRetry();
    } else if (event.key === "d" && onDispatch !== undefined) {
      event.preventDefault();
      onDispatch();
    }
  };
  return (
    <section className="armada-needs-panel" aria-label="Failed to hand off to new job" data-kind="failed" data-open={false} onKeyDown={keys}>
      <div className="armada-needs-panel__band">
        <button type="button" className="armada-needs-panel__trigger" aria-label="Failed to hand off to new job" onClick={onDismiss}>
          <X size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Failed to hand off to new job</span>
        </button>
      </div>
      <div className="armada-needs-panel__reveal" data-open>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open>
            {reason === undefined ? null : <p className="armada-handoff-band__reason">{reason}</p>}
            <div className="armada-session-done__acts">
              {onRetry === undefined ? null : (
                <Button size="sm" variant="secondary" aria-keyshortcuts="r" onClick={onRetry}>
                  <RotateCw size={12} strokeWidth={2} aria-hidden />
                  Retry
                  <Kbd aria-hidden>R</Kbd>
                </Button>
              )}
              {onDispatch === undefined ? null : (
                <Button size="sm" variant="secondary" aria-keyshortcuts="d" onClick={onDispatch}>
                  Dispatch job to fix
                  <Kbd aria-hidden>D</Kbd>
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
