import { X } from "lucide-react";

/** A hand off that did not start, as a band on the composer in the failed hue, drawn as the Needs you band is and with no body: a press puts it away. */
export function HandoffBand({ onDismiss }: { onDismiss: () => void }) {
  return (
    <section className="armada-needs-panel" aria-label="Failed to hand off to new job" data-kind="failed" data-open={false}>
      <div className="armada-needs-panel__band">
        <button type="button" className="armada-needs-panel__trigger" aria-label="Failed to hand off to new job" onClick={onDismiss}>
          <X size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Failed to hand off to new job</span>
        </button>
      </div>
    </section>
  );
}
