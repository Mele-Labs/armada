import { Minus, Plus, X } from "lucide-react";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type HTMLAttributes,
  type PointerEvent,
  type ReactNode,
  type RefObject,
} from "react";

import { Button } from "../Button/Button";
import { Tooltip } from "../Tooltip/Tooltip";

/** Fit is scale 1: the picture as the box lays it out, which is the floor. */
type View = {
  scale: number;
  x: number;
  y: number;
};

const FIT: View = {
  scale: 1,
  x: 0,
  y: 0,
};
const MOST = 8;
const DOUBLE_TAP_SCALE = 2.5;
const KEY_STEP = 1.5;
const KEY_PAN = 64;
/** A press that travels further than this is a drag, and is not a tap. */
const TAP_SLOP = 8;
const DOUBLE_TAP_MS = 300;
const DOUBLE_TAP_SLOP = 24;
/** A trackpad pinch sends small ctrl+wheel deltas and a mouse wheel large plain ones, so each has its own speed. */
const PINCH_SPEED = 0.01;
const WHEEL_SPEED = 0.002;
const LINE_HEIGHT = 16;

const limited = (scale: number) => Math.min(MOST, Math.max(FIT.scale, scale));

/** How far one axis may be panned: an edge of a picture larger than the box never comes inside it, and a smaller one stays centred. */
function panned(pan: number, size: number, scale: number, centre: number, whole: number) {
  const half = (size * scale) / 2;
  if (half * 2 <= whole) return 0;
  return Math.min(half - centre, Math.max(whole - centre - half, pan));
}

/**
 * A picture that fills its box and zooms: two pointers pinch about their midpoint, one drags when zoomed, a double tap
 * toggles fit and 2.5x at that point, and a wheel zooms about the cursor. `+`, `-`, `0` and the arrows work while it is
 * mounted. The box takes `role`, labels and `tabIndex` from its caller, so the viewer's dialog can be this very box.
 *
 * `onPressOutside` hears a tap beside the picture at fit, where nothing is lost by closing. `bar` is what sits after the
 * zoom buttons, top right.
 */
export function ZoomablePicture({
  src,
  alt,
  className,
  rootRef,
  onPressOutside,
  bar,
  ...rest
}: Omit<
  HTMLAttributes<HTMLDivElement>,
  "onPointerDown" | "onPointerMove" | "onPointerUp" | "onPointerCancel" | "onClick" | "children"
> & {
  src: string;
  alt: string;
  rootRef?: RefObject<HTMLDivElement | null>;
  onPressOutside?: () => void;
  bar?: ReactNode;
}) {
  const own = useRef<HTMLDivElement>(null);
  const root = rootRef ?? own;
  const picture = useRef<HTMLImageElement>(null);
  const view = useRef(FIT);
  const [zoomed, setZoomed] = useState(false);
  const pointers = useRef(
    new Map<
      number,
      {
        x: number;
        y: number;
      }
    >(),
  );
  const press = useRef({
    x: 0,
    y: 0,
    moved: false,
    many: false,
    outside: false,
  });
  const lastTap = useRef<
    | {
        at: number;
        x: number;
        y: number;
      }
    | undefined
  >(undefined);

  // Written to the element, not to state: a pinch moves it every frame and nothing else on the page depends on it.
  const show = useCallback(
    (next: View, glide = false) => {
      const box = root.current;
      const held = picture.current;
      if (box === null || held === null) return;
      const scale = limited(next.scale);
      const at = {
        scale,
        x: panned(
          next.x,
          held.offsetWidth,
          scale,
          held.offsetLeft + held.offsetWidth / 2,
          box.clientWidth,
        ),
        y: panned(
          next.y,
          held.offsetHeight,
          scale,
          held.offsetTop + held.offsetHeight / 2,
          box.clientHeight,
        ),
      };
      view.current = at;
      held.style.transition = glide ? "" : "none";
      held.style.transform = `translate(${at.x}px, ${at.y}px) scale(${at.scale})`;
      setZoomed(at.scale > FIT.scale);
    },
    [
      root,
    ],
  );

  const zoomAbout = useCallback(
    (clientX: number, clientY: number, scale: number, glide = false) => {
      const box = root.current;
      const held = picture.current;
      if (box === null || held === null) return;
      const edge = box.getBoundingClientRect();
      const x = clientX - edge.left - (held.offsetLeft + held.offsetWidth / 2);
      const y = clientY - edge.top - (held.offsetTop + held.offsetHeight / 2);
      const was = view.current;
      const by = limited(scale) / was.scale;
      show(
        {
          scale,
          x: x - by * (x - was.x),
          y: y - by * (y - was.y),
        },
        glide,
      );
    },
    [
      root,
      show,
    ],
  );

  const zoomBy = useCallback(
    (factor: number) => {
      const box = root.current;
      if (box === null) return;
      const edge = box.getBoundingClientRect();
      zoomAbout(
        edge.left + edge.width / 2,
        edge.top + edge.height / 2,
        view.current.scale * factor,
        true,
      );
    },
    [
      root,
      zoomAbout,
    ],
  );

  useEffect(() => {
    show(FIT);
  }, [
    src,
    show,
  ]);

  // React's wheel listener is passive, and a pinch left to the browser zooms the whole page.
  useEffect(() => {
    const box = root.current;
    if (box === null) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const delta =
        event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * LINE_HEIGHT : event.deltaY;
      const speed = event.ctrlKey ? PINCH_SPEED : WHEEL_SPEED;
      zoomAbout(event.clientX, event.clientY, view.current.scale * Math.exp(-delta * speed));
    };
    box.addEventListener("wheel", wheel, {
      passive: false,
    });
    const refit = new ResizeObserver(() => show(view.current));
    refit.observe(box);
    return () => {
      box.removeEventListener("wheel", wheel);
      refit.disconnect();
    };
  }, [
    root,
    show,
    zoomAbout,
  ]);

  // On the window and first, as Esc is: an arrow left to bubble would also move the thread behind the picture.
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const was = view.current;
      if (event.key === "+" || event.key === "=") zoomBy(KEY_STEP);
      else if (event.key === "-") zoomBy(1 / KEY_STEP);
      else if (event.key === "0") show(FIT, true);
      else if (was.scale > FIT.scale && event.key === "ArrowLeft")
        show(
          {
            ...was,
            x: was.x + KEY_PAN,
          },
          true,
        );
      else if (was.scale > FIT.scale && event.key === "ArrowRight")
        show(
          {
            ...was,
            x: was.x - KEY_PAN,
          },
          true,
        );
      else if (was.scale > FIT.scale && event.key === "ArrowUp")
        show(
          {
            ...was,
            y: was.y + KEY_PAN,
          },
          true,
        );
      else if (was.scale > FIT.scale && event.key === "ArrowDown")
        show(
          {
            ...was,
            y: was.y - KEY_PAN,
          },
          true,
        );
      else return;
      event.preventDefault();
      event.stopPropagation();
    };
    window.addEventListener("keydown", keys, true);
    return () => window.removeEventListener("keydown", keys, true);
  }, [
    show,
    zoomBy,
  ]);

  const tapped = (x: number, y: number) => {
    const held = picture.current?.getBoundingClientRect();
    const on =
      held !== undefined && x >= held.left && x <= held.right && y >= held.top && y <= held.bottom;
    if (view.current.scale === FIT.scale && !on) {
      press.current.outside = true;
      return;
    }
    const last = lastTap.current;
    if (
      last !== undefined &&
      performance.now() - last.at < DOUBLE_TAP_MS &&
      Math.hypot(x - last.x, y - last.y) < DOUBLE_TAP_SLOP
    ) {
      lastTap.current = undefined;
      if (view.current.scale > FIT.scale) show(FIT, true);
      else zoomAbout(x, y, DOUBLE_TAP_SCALE, true);
      return;
    }
    lastTap.current = {
      at: performance.now(),
      x,
      y,
    };
  };

  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target as Element).closest("button") !== null) return;
    if (pointers.current.size === 0)
      press.current = {
        x: event.clientX,
        y: event.clientY,
        moved: false,
        many: false,
        outside: false,
      };
    else press.current.many = true;
    pointers.current.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
    });
  };

  const move = (event: PointerEvent<HTMLDivElement>) => {
    const was = pointers.current.get(event.pointerId);
    if (was === undefined) return;
    const now = {
      x: event.clientX,
      y: event.clientY,
    };
    const other = [
      ...pointers.current,
    ].find(([id]) => id !== event.pointerId)?.[1];
    if (other !== undefined) {
      const apart = Math.hypot(was.x - other.x, was.y - other.y);
      const middle = {
        x: (was.x + other.x) / 2,
        y: (was.y + other.y) / 2,
      };
      zoomAbout(
        middle.x,
        middle.y,
        view.current.scale * (Math.hypot(now.x - other.x, now.y - other.y) / (apart || 1)),
      );
      show({
        ...view.current,
        x: view.current.x + (now.x - was.x) / 2,
        y: view.current.y + (now.y - was.y) / 2,
      });
    } else {
      if (Math.hypot(now.x - press.current.x, now.y - press.current.y) > TAP_SLOP) {
        // A mouse leaves the box mid drag; a finger is already held to it by the browser.
        if (!press.current.moved) root.current?.setPointerCapture(event.pointerId);
        press.current.moved = true;
      }
      if (press.current.moved && view.current.scale > FIT.scale)
        show({
          ...view.current,
          x: view.current.x + now.x - was.x,
          y: view.current.y + now.y - was.y,
        });
    }
    pointers.current.set(event.pointerId, now);
  };

  const up = (event: PointerEvent<HTMLDivElement>) => {
    if (!pointers.current.delete(event.pointerId)) return;
    if (pointers.current.size === 0 && !press.current.many && !press.current.moved)
      tapped(event.clientX, event.clientY);
  };

  return (
    <div
      {...rest}
      ref={root}
      className={className === undefined ? "armada-zoomable" : `armada-zoomable ${className}`}
      data-zoomed={zoomed ? "" : undefined}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={(event) => void pointers.current.delete(event.pointerId)}
      // On the click and not the pointer's lift: a finger's click comes after its lift, and with this box gone it would land on what is behind.
      onClick={() => {
        if (press.current.outside) onPressOutside?.();
      }}
    >
      <img
        ref={picture}
        className="armada-zoomable__picture"
        src={src}
        alt={alt}
        draggable={false}
      />
      <div className="armada-zoomable__bar">
        <div className="armada-zoomable__group armada-zoomable__group--zoom">
          <Tooltip label="Zoom out" shortcut="-">
            <Button
              variant="ghost"
              iconOnly
              aria-label="Zoom out"
              onClick={() => zoomBy(1 / KEY_STEP)}
            >
              <Minus size={14} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          <Tooltip label="Zoom in" shortcut="+">
            <Button variant="ghost" iconOnly aria-label="Zoom in" onClick={() => zoomBy(KEY_STEP)}>
              <Plus size={14} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </div>
        {bar}
      </div>
    </div>
  );
}

/** The viewer's close, top right of its bar. */
export function ViewerClose({ onClose }: { onClose: () => void }) {
  return (
    <div className="armada-zoomable__group">
      <Tooltip label="Close" shortcut="Esc">
        <Button variant="ghost" iconOnly aria-label="Close" onClick={onClose}>
          <X size={14} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}
