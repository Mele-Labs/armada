// A confirmation drawn as a card beside the thing it asks about, with a pointer to it and everything else dimmed. Cancel holds focus, so
// Enter keeps the item; Esc and a press outside both cancel.

import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Button } from "@armada/components";

type Props = {
  /** The item it asks about, in the window. */
  target: { left: number; top: number; width: number; height: number };
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
};

export function ConfirmCard({ target, title, children, confirmLabel, onConfirm, onCancel }: Props) {
  const card = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    cancel.current?.focus();
    function onKey(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      onCancel();
    }
    function onDown(event: MouseEvent) {
      if (!card.current?.contains(event.target as Node)) onCancel();
    }
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [onCancel]);
  return createPortal(
    <>
      <div className="projects-confirm__hole" aria-hidden style={{ left: target.left, top: target.top, width: target.width, height: target.height }} />
      <div ref={card} className="projects-confirm" role="alertdialog" aria-label={title} style={{ left: target.left + target.width + 12, top: target.top }}>
        <span className="projects-confirm__pointer" aria-hidden style={{ top: target.height / 2 }} />
        <p className="projects-confirm__title">{title}</p>
        <p className="projects-confirm__body">{children}</p>
        <div className="projects-confirm__acts">
          <Button ref={cancel} variant="secondary" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button variant="destructive" size="sm" onClick={onConfirm}>
            {confirmLabel}
          </Button>
        </div>
      </div>
    </>,
    document.body,
  );
}
