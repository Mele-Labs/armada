import { Unplug } from "lucide-react";

import { ToolsMenu, type ToolsFix } from "../SessionFrame/SessionFrame";

/**
 * The agent came up without some of its Armada tools, as a band on the composer drawn as the Needs you band is. Its body is always up: the
 * facts the mark's tooltip says, and each missing piece with the act that puts it right. It has no dismissal; it goes when nothing is missing.
 * The acts come before the message box in the Tab order, and the first is not focused on draw, so the box keeps the cursor.
 */
export function ToolsBand({ said, fixes, onFix }: { said: string; fixes: readonly ToolsFix[]; onFix: (piece: string) => void }) {
  return (
    <section className="armada-needs-panel" aria-label={said} data-kind="tools" data-open={false}>
      <div className="armada-needs-panel__band">
        <div className="armada-needs-panel__trigger">
          <Unplug size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Tools missing</span>
        </div>
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
