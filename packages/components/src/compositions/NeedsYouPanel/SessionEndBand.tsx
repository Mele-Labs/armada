import { useRef } from "react";
import type { KeyboardEvent } from "react";
import { Check, ChevronUp } from "lucide-react";

import { DoneBody, type ThreadDone } from "../SessionThread/SessionThread";
import { useEscapeFold } from "./useEscapeFold";
import { useScrimFold } from "./useScrimFold";

/**
 * The agent's last word as a band on the composer, in the done hue, drawn as the Needs you band is: pressed, it rides up as the header of a
 * panel holding what was made and the two ways on, and the thread behind it dims. Esc and a press on the dimmed thread fold it. The host opens
 * it on coming into the Session. It shares the panel's styles, so the band and the body move the same way.
 */
export function SessionEndBand({ done, open, onOpen }: { done: ThreadDone; open: boolean; onOpen: (next: boolean) => void }) {
  const section = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useScrimFold(section, open, () => onOpen(false));
  useEscapeFold(section, open, () => onOpen(false));
  const close = (event: KeyboardEvent) => {
    if (!open || event.key !== "Escape" || event.defaultPrevented) return;
    event.preventDefault();
    event.stopPropagation();
    onOpen(false);
    trigger.current?.focus();
  };
  return (
    <section ref={section} className="armada-needs-panel" aria-label="Nothing left to do" data-kind="end" data-open={open} onKeyDown={close} data-keeps-escape={open ? "" : undefined}>
      <div className="armada-needs-panel__band">
        <button ref={trigger} type="button" className="armada-needs-panel__trigger" aria-label="Nothing left to do" aria-expanded={open} onClick={() => onOpen(!open)}>
          <Check size={12} strokeWidth={2} aria-hidden />
          <span className="armada-needs-panel__label">Nothing left to do</span>
          <ChevronUp className="armada-needs-panel__fold" size={12} strokeWidth={2} aria-hidden />
        </button>
      </div>
      <div className="armada-needs-panel__reveal" data-open={open}>
        <div className="armada-needs-panel__shell">
          <div className="armada-needs-panel__body" data-open={open}>
            <DoneBody done={done} />
          </div>
        </div>
      </div>
    </section>
  );
}
