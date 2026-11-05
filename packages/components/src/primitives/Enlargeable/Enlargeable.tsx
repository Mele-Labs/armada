import {
  useEffect,
  useRef,
  useState,
  type ReactEventHandler,
  type ReactNode,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";

import { ViewerClose, ZoomablePicture } from "./ZoomablePicture";

const STOPS = "button, a[href], input, textarea, select, [tabindex]:not([tabindex='-1'])";

/** Tab stays among the dialog's own buttons: the page behind it is out of reach while it is open. */
function keepTabInside(layer: RefObject<HTMLElement | null>, event: KeyboardEvent) {
  const box = layer.current;
  if (box === null) return;
  const stops = [
    ...box.querySelectorAll<HTMLElement>(STOPS),
  ].filter((one) => one.offsetParent !== null);
  event.preventDefault();
  const at = stops.indexOf(document.activeElement as HTMLElement);
  const last = stops.length - 1;
  stops[event.shiftKey ? (at <= 0 ? last : at - 1) : at === last ? 0 : at + 1]?.focus();
}

/**
 * A picture that opens over the screen. The thumbnail is the button and the large view is the same `src`, so nothing is
 * fetched twice. The large view zooms (`ZoomablePicture`); Escape, Close, or a press beside the picture at fit closes it,
 * and focus goes back to the thumbnail. **No text is drawn**: the picture's own name is the only label, on the button and
 * on the dialog.
 *
 * `large` draws in the picture's place in the large view, with no gestures: a `NotableImage`, whose press draws a region
 * rather than closing. Esc goes first to a note or a region open in it. `opened` starts it large, for a press that asked
 * for the picture itself. `fills` sizes the thumbnail's button to its parent, for a frame that has already worked out the
 * picture's size; `onLoad` and `onError` are the thumbnail's.
 */
export function Enlargeable({
  src,
  name,
  className,
  large,
  opened = false,
  fills = false,
  onLoad,
  onError,
}: {
  src: string;
  name: string;
  className?: string;
  large?: ReactNode;
  opened?: boolean;
  fills?: boolean;
  onLoad?: ReactEventHandler<HTMLImageElement>;
  onError?: ReactEventHandler<HTMLImageElement>;
}) {
  const [open, setOpen] = useState(opened);
  const thumb = useRef<HTMLButtonElement>(null);
  const layer = useRef<HTMLDivElement>(null);
  const zooms = large === undefined;

  const close = () => {
    setOpen(false);
    thumb.current?.focus();
  };

  useEffect(() => {
    if (!open) return;
    layer.current?.focus();
    const keys = (event: KeyboardEvent) => {
      // A note open on the picture is portalled out of the dialog and keeps its own keys.
      if (event.key === "Tab" && layer.current?.querySelector("[data-viewer-notes-open]") == null)
        return keepTabInside(layer, event);
      if (event.key !== "Escape") return;
      if (
        layer.current?.querySelector(
          "[data-viewer-notes-open], .armada-notable-image__region[data-drawing]",
        ) != null
      )
        return;
      event.stopPropagation();
      event.preventDefault();
      setOpen(false);
      thumb.current?.focus();
    };
    window.addEventListener("keydown", keys, true);
    return () => window.removeEventListener("keydown", keys, true);
  }, [
    open,
  ]);

  return (
    <>
      <button
        ref={thumb}
        type="button"
        className={fills ? "armada-enlargeable armada-enlargeable--fills" : "armada-enlargeable"}
        aria-label={name}
        onClick={() => setOpen(true)}
      >
        <img
          className={className}
          src={src}
          alt=""
          {...(onLoad === undefined
            ? {}
            : {
                onLoad,
              })}
          {...(onError === undefined
            ? {}
            : {
                onError,
              })}
        />
      </button>
      {open
        ? createPortal(
            zooms ? (
              <ZoomablePicture
                rootRef={layer}
                className="armada-enlargeable__scrim"
                src={src}
                alt={name}
                role="dialog"
                aria-modal="true"
                aria-label={name}
                tabIndex={-1}
                onPressOutside={close}
                bar={<ViewerClose onClose={close} />}
              />
            ) : (
              <div
                ref={layer}
                className="armada-enlargeable__scrim"
                role="dialog"
                aria-modal="true"
                aria-label={name}
                tabIndex={-1}
                onClick={close}
              >
                {large}
                <div className="armada-zoomable__bar">
                  <ViewerClose onClose={close} />
                </div>
              </div>
            ),
            document.body,
          )
        : null}
    </>
  );
}

/**
 * The large view of `Enlargeable`, opened by something else: a row's press. A picture zooms as it does there, and a video
 * plays with its own controls. Escape, Close, or a press beside it closes it, and `onClose` unmounts it.
 */
export function EnlargedMedia({
  src,
  name,
  kind,
  onClose,
}: {
  src: string;
  name: string;
  kind: "picture" | "video";
  onClose: () => void;
}) {
  const layer = useRef<HTMLDivElement>(null);
  const closing = useRef(onClose);
  closing.current = onClose;

  useEffect(() => {
    layer.current?.focus();
    const keys = (event: KeyboardEvent) => {
      if (event.key === "Tab") return keepTabInside(layer, event);
      if (event.key !== "Escape") return;
      event.stopPropagation();
      event.preventDefault();
      closing.current();
    };
    window.addEventListener("keydown", keys, true);
    return () => window.removeEventListener("keydown", keys, true);
  }, []);

  return createPortal(
    kind === "picture" ? (
      <ZoomablePicture
        rootRef={layer}
        className="armada-enlargeable__scrim"
        src={src}
        alt={name}
        role="dialog"
        aria-modal="true"
        aria-label={name}
        tabIndex={-1}
        onPressOutside={onClose}
        bar={<ViewerClose onClose={onClose} />}
      />
    ) : (
      <div
        ref={layer}
        className="armada-enlargeable__scrim"
        role="dialog"
        aria-modal="true"
        aria-label={name}
        tabIndex={-1}
        onClick={onClose}
      >
        <video
          className="armada-enlargeable__video"
          controls
          playsInline
          src={src}
          aria-label={name}
          onClick={(event) => event.stopPropagation()}
        />
        <div className="armada-zoomable__bar">
          <ViewerClose onClose={onClose} />
        </div>
      </div>
    ),
    document.body,
  );
}
