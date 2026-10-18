import { X } from "lucide-react";

/** A hand off that did not start, as a band on the composer in the failed hue, drawn as the Needs you band is and with no body: a press puts it away. */
export function HandoffBand({ onDismiss }: { onDismiss: () => void }) {
  return (
    <section className="armada-needs-panel" aria-label="Did not start" data-kind="failed" data-open={false}>
      <button type="button" className="armada-needs-panel__trigger" aria-label="Did not start" onClick={onDismiss}>
        <X size={12} strokeWidth={2} aria-hidden />
        <span className="armada-needs-panel__label">Did not start</span>
      </button>
    </section>
  );
}
