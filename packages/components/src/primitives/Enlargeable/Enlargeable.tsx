import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * A picture that opens larger over the screen. The thumbnail is the button and the large view is
 * the same `src`, so nothing is fetched twice. Escape, a press outside the picture, or a press on it
 * closes, and focus goes back to the thumbnail. **No text is drawn**: the picture's own name is the
 * only label, on the button and on the dialog.
 *
 * `large` draws in the picture's place in the large view: a `NotableImage`, whose press draws a region rather than
 * closing. Esc goes first to a note or a region open in it. `opened` starts it large, for a press that asked for the picture itself.
 */
export function Enlargeable({ src, name, className, large, opened = false }: { src: string; name: string; className?: string; large?: ReactNode; opened?: boolean }) {
  const [open, setOpen] = useState(opened);
  const thumb = useRef<HTMLButtonElement>(null);
  const layer = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    layer.current?.focus();
    const close = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (layer.current?.querySelector("[data-viewer-notes-open], .armada-notable-image__region[data-drawing]") != null) return;
      event.stopPropagation();
      event.preventDefault();
      setOpen(false);
      thumb.current?.focus();
    };
    window.addEventListener("keydown", close, true);
    return () => window.removeEventListener("keydown", close, true);
  }, [open]);

  return (
    <>
      <button ref={thumb} type="button" className="armada-enlargeable" aria-label={name} onClick={() => setOpen(true)}>
        <img className={className} src={src} alt="" />
      </button>
      {open
        ? createPortal(
            <div
              ref={layer}
              className="armada-enlargeable__scrim"
              role="dialog"
              aria-modal="true"
              aria-label={name}
              tabIndex={-1}
              onClick={() => {
                setOpen(false);
                thumb.current?.focus();
              }}
            >
              {large ?? <img className="armada-enlargeable__picture" src={src} alt={name} />}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
