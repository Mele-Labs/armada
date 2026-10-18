// The Field large, as a layer over the Session's stage: the same frame the other layers have (`SessionLayers.tsx`), its name, Back, Esc as
// Back, and the thread and composer left mounted under it. It is its own file because it draws no rows: the map fills the body.

import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { FieldMap } from "./FieldPanel";
import { closeLayer } from "./layers";

export function FieldLayer({ session, repositories }: { session: Session; repositories: readonly RepositorySummary[] }) {
  const body = useRef<HTMLDivElement>(null);
  // Esc is Back, wherever focus is, unless a sheet is open over the layer: that one's Esc is its own.
  useEffect(() => {
    const listen = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      event.preventDefault();
      closeLayer();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);
  // Focus goes to the map, whose own keys pan and zoom it.
  useEffect(() => body.current?.querySelector<HTMLElement>(".bf")?.focus(), []);
  return (
    <section className="sv-layer" aria-label="Field" data-keeps-escape="">
      <header className="sv-layer__head">
        <h3 className="sv-layer__title">Field</h3>
        <Tooltip label="Back" shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={closeLayer}>
            <X size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      </header>
      <div ref={body} className="sv-layer__body sv-fieldlayer" role="group" aria-label="Expanded Field">
        <FieldMap session={session} repositories={repositories} large />
      </div>
    </section>
  );
}
