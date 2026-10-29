import { Unplug, X } from "lucide-react";
import type { KeyboardEvent } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ToolsMenu, type ToolsFix } from "../SessionFrame/SessionFrame";

/**
 * The agent came up without some of its Armada tools, as a band on the composer drawn as the Needs you band is. Its body is always up: the
 * facts the mark's tooltip says, and each missing piece with the act that puts it right. A cross on the band puts it away, and so does Esc
 * while focus is inside it; the host keeps it away until what is missing changes, and the header's mark pulses in its place.
 * The acts come before the message box in the Tab order, and the first is not focused on draw, so the box keeps the cursor.
 */
export function ToolsBand({ said, fixes, onFix, onDismiss }: { said: string; fixes: readonly ToolsFix[]; onFix: (piece: string) => void; onDismiss: () => void }) {
  const keys = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onDismiss();
  };
  return (
    <section className="armada-needs-panel" aria-label={said} data-kind="tools" data-open={false} onKeyDown={keys}>
      <div className="armada-needs-panel__band">
        <div className="armada-needs-panel__trigger">
          <Unplug size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Tools missing</span>
        </div>
        <Tooltip label="Dismiss">
          <button type="button" className="armada-needs-panel__dismiss" aria-label="Dismiss" onClick={onDismiss}>
            <X size={12} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      </div>
      <div className="armada-needs-panel__reveal" data-open>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open>
            <ToolsMenu said={said} fixes={fixes} onFix={onFix} close={() => undefined} focusFirst={false} />
          </div>
        </div>
      </div>
    </section>
  );
}
