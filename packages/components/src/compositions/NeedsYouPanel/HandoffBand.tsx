import { RotateCw, X } from "lucide-react";
import type { KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { keyFor } from "../../actions";
import { isPressed, useKeyBindings } from "../../keymap";
import { ThreadKbd, useThreadCard } from "../SessionThread/thread-keys";

/** A bare letter is drawn as the cap reads it. */
const cap = (key: string): string => (key.length === 1 ? key.toUpperCase() : key);

/**
 * A hand off that did not start, as a band on the composer in the failed hue, drawn as the Needs you band is. Its body is always up: the
 * reason Fleet recorded, and the two ways on, Retry (`handoff_retry`) and Dispatch job to fix (`handoff_dispatch`), the failure toast's own act.
 * A press on the band puts it away. The two keys work while focus is inside the band; as a waiting card of the Job, `⌥↵` reaches Retry too.
 */
export function HandoffBand({ reason, onRetry, onDispatch, onDismiss }: { reason?: string; onRetry?: () => void; onDispatch?: () => void; onDismiss: () => void }) {
  useKeyBindings();
  const { target, bind } = useThreadCard(onRetry === undefined ? undefined : { id: "handoff", kind: "Hand off", title: reason ?? "Failed to hand off to new job", acts: { yes: { verb: "Retry", run: onRetry } } });
  const retryKey = keyFor("handoff_retry");
  const dispatchKey = keyFor("handoff_dispatch");
  const keys = (event: KeyboardEvent) => {
    if (event.defaultPrevented) return;
    if (isPressed("handoff_retry", event) && onRetry !== undefined) {
      event.preventDefault();
      onRetry();
    } else if (isPressed("handoff_dispatch", event) && onDispatch !== undefined) {
      event.preventDefault();
      onDispatch();
    }
  };
  return (
    <section className="armada-needs-panel" aria-label="Failed to hand off to new job" data-kind="failed" data-open={false} onKeyDown={keys} {...bind}>
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
                <Button size="sm" variant="secondary" {...(retryKey === "" ? {} : { "aria-keyshortcuts": retryKey })} onClick={onRetry}>
                  <RotateCw size={12} strokeWidth={2} aria-hidden />
                  Retry
                  {retryKey === "" ? null : <Kbd aria-hidden>{cap(retryKey)}</Kbd>}
                  <ThreadKbd act="yes" on={target} />
                </Button>
              )}
              {onDispatch === undefined ? null : (
                <Button size="sm" variant="secondary" {...(dispatchKey === "" ? {} : { "aria-keyshortcuts": dispatchKey })} onClick={onDispatch}>
                  Dispatch job to fix
                  {dispatchKey === "" ? null : <Kbd aria-hidden>{cap(dispatchKey)}</Kbd>}
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
