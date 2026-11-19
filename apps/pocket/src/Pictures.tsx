// A picture opened over the whole screen: a drawing or a shot, or the takes of a vote one after another. It opens fitted to the
// scene's own bounds, so nothing is cut off. A pinch or a double tap zooms, a drag pans once zoomed, and a swipe or the
// named arrows go between takes. Pick chooses the take on view and closes back to the artifact, where it is sent.

import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { Check, ChevronLeft, ChevronRight, X } from "lucide-react";
import { Button } from "./kit/components/primitives/Button/Button";
import { SketchScene } from "./kit/components/compositions/SketchScene/SketchScene";
import type { Scene } from "./kit/components/compositions/SketchScene/scene";

import { IconButton } from "./ui";

/** A drawing, or a shot read from the Mac as an address the page can show. */
export type Picture = {
  id: string;
  title: string;
} & (
  | {
      scene: Scene;
    }
  | {
      src: string;
    }
);

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
const DOUBLE = 2.5;
/** A press that moves further than this is a drag or a swipe, never a tap. */
const TRAVEL = 8;
/** How far a swipe at fit must go across to move to the next take. */
const SWIPE = 56;
const DOUBLE_MS = 300;

const limited = (scale: number) => Math.min(MOST, Math.max(1, scale));

/** How far one axis may move: an edge of the zoomed picture never comes inside the screen. */
function panned(pan: number, length: number, scale: number): number {
  const spare = (length * (scale - 1)) / 2;
  return Math.min(spare, Math.max(-spare, pan));
}

export function Pictures({
  pictures,
  at,
  onAt,
  picked,
  onPick,
  onClose,
}: {
  pictures: readonly Picture[];
  at: number;
  onAt: (at: number) => void;
  /** The take chosen, where the pictures are a vote's takes. */
  picked?: string;
  /** Present while the vote asks: Pick chooses the take on view. */
  onPick?: (id: string) => void;
  onClose: () => void;
}) {
  const stage = useRef<HTMLDivElement>(null);
  const [view, setView] = useState<View>(FIT);
  const pointers = useRef(
    new Map<
      number,
      {
        x: number;
        y: number;
      }
    >(),
  );
  const gesture = useRef<
    | {
        kind: "one";
        from: {
          x: number;
          y: number;
        };
        start: View;
        moved: boolean;
      }
    | {
        kind: "two";
        distance: number;
        middle: {
          x: number;
          y: number;
        };
        start: View;
      }
    | undefined
  >(undefined);
  const lastTap = useRef(0);
  const picture = pictures[at];
  const before = at > 0 ? pictures[at - 1] : undefined;
  const after = at < pictures.length - 1 ? pictures[at + 1] : undefined;

  // Each take opens at fit.
  useEffect(
    () => setView(FIT),
    [
      at,
    ],
  );

  const size = () => {
    const box = stage.current?.getBoundingClientRect();
    return {
      width: box?.width ?? 1,
      height: box?.height ?? 1,
      left: box?.left ?? 0,
      top: box?.top ?? 0,
    };
  };

  /** `next` zoomed about a point on the screen, so that point stays under the fingers. */
  const zoomedAbout = (
    from: View,
    scale: number,
    point: {
      x: number;
      y: number;
    },
  ): View => {
    const box = size();
    const next = limited(scale);
    const cx = point.x - box.left - box.width / 2;
    const cy = point.y - box.top - box.height / 2;
    const x = cx - ((cx - from.x) * next) / from.scale;
    const y = cy - ((cy - from.y) * next) / from.scale;
    return {
      scale: next,
      x: panned(x, box.width, next),
      y: panned(y, box.height, next),
    };
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      else if (event.key === "ArrowLeft" && before !== undefined) onAt(at - 1);
      else if (event.key === "ArrowRight" && after !== undefined) onAt(at + 1);
      else if (event.key === "+" || event.key === "=")
        setView((was) => ({
          ...was,
          scale: limited(was.scale * 1.5),
        }));
      else if (event.key === "-")
        setView((was) => {
          const scale = limited(was.scale / 1.5);
          const box = size();
          return {
            scale,
            x: panned(was.x, box.width, scale),
            y: panned(was.y, box.height, scale),
          };
        });
      else if (event.key === "0") setView(FIT);
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    at,
    before,
    after,
  ]);

  // A trackpad's pinch reaches a page as a wheel with Control held.
  useEffect(() => {
    const el = stage.current;
    if (el === null) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      if (event.ctrlKey)
        setView((was) =>
          zoomedAbout(was, was.scale * Math.exp(-event.deltaY / 100), {
            x: event.clientX,
            y: event.clientY,
          }),
        );
      else
        setView((was) => {
          const box = size();
          return {
            ...was,
            x: panned(was.x - event.deltaX, box.width, was.scale),
            y: panned(was.y - event.deltaY, box.height, was.scale),
          };
        });
    };
    el.addEventListener("wheel", onWheel, {
      passive: false,
    });
    return () => el.removeEventListener("wheel", onWheel);
  }, []);

  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    pointers.current.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
    });
    const held = [
      ...pointers.current.values(),
    ];
    if (held.length === 2) {
      const [a, b] = held as [
        {
          x: number;
          y: number;
        },
        {
          x: number;
          y: number;
        },
      ];
      gesture.current = {
        kind: "two",
        distance: Math.hypot(a.x - b.x, a.y - b.y),
        middle: {
          x: (a.x + b.x) / 2,
          y: (a.y + b.y) / 2,
        },
        start: view,
      };
    } else if (held.length === 1)
      gesture.current = {
        kind: "one",
        from: {
          x: event.clientX,
          y: event.clientY,
        },
        start: view,
        moved: false,
      };
  };

  const move = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!pointers.current.has(event.pointerId)) return;
    pointers.current.set(event.pointerId, {
      x: event.clientX,
      y: event.clientY,
    });
    const now = gesture.current;
    if (now?.kind === "two") {
      const [a, b] = [
        ...pointers.current.values(),
      ] as [
        {
          x: number;
          y: number;
        },
        {
          x: number;
          y: number;
        },
      ];
      const distance = Math.hypot(a.x - b.x, a.y - b.y);
      const middle = {
        x: (a.x + b.x) / 2,
        y: (a.y + b.y) / 2,
      };
      const zoomed = zoomedAbout(
        now.start,
        (now.start.scale * distance) / Math.max(1, now.distance),
        now.middle,
      );
      const box = size();
      setView({
        scale: zoomed.scale,
        x: panned(zoomed.x + middle.x - now.middle.x, box.width, zoomed.scale),
        y: panned(zoomed.y + middle.y - now.middle.y, box.height, zoomed.scale),
      });
      return;
    }
    if (now?.kind !== "one") return;
    const dx = event.clientX - now.from.x;
    const dy = event.clientY - now.from.y;
    if (Math.hypot(dx, dy) > TRAVEL) now.moved = true;
    if (now.start.scale > 1) {
      const box = size();
      setView({
        scale: now.start.scale,
        x: panned(now.start.x + dx, box.width, now.start.scale),
        y: panned(now.start.y + dy, box.height, now.start.scale),
      });
    }
  };

  const up = (event: ReactPointerEvent<HTMLDivElement>) => {
    pointers.current.delete(event.pointerId);
    const now = gesture.current;
    if (pointers.current.size > 0) {
      // One finger of a pinch lifted: the other carries on as a drag from where it is.
      const [rest] = [
        ...pointers.current.values(),
      ];
      gesture.current =
        rest === undefined
          ? undefined
          : {
              kind: "one",
              from: rest,
              start: view,
              moved: true,
            };
      return;
    }
    gesture.current = undefined;
    if (now?.kind !== "one") return;
    const dx = event.clientX - now.from.x;
    const dy = event.clientY - now.from.y;
    if (now.start.scale === 1 && Math.abs(dx) > SWIPE && Math.abs(dx) > Math.abs(dy) * 1.5) {
      if (dx < 0 && after !== undefined) onAt(at + 1);
      else if (dx > 0 && before !== undefined) onAt(at - 1);
      return;
    }
    if (now.moved) return;
    const tapped = event.timeStamp;
    if (tapped - lastTap.current < DOUBLE_MS) {
      lastTap.current = 0;
      setView((was) =>
        was.scale > 1
          ? FIT
          : zoomedAbout(was, DOUBLE, {
              x: event.clientX,
              y: event.clientY,
            }),
      );
    } else lastTap.current = tapped;
  };

  if (picture === undefined) return null;
  const chosen = picked === picture.id;
  return (
    <div className="pk-pictures" role="dialog" aria-modal="true" aria-label={picture.title}>
      <header className="pk-pictures__band">
        <h2 className="pk-pictures__title">{picture.title}</h2>
        <IconButton label="Close" onClick={onClose}>
          <X size={16} aria-hidden="true" />
        </IconButton>
      </header>
      <div
        ref={stage}
        className="pk-pictures__stage"
        data-zoomed={view.scale > 1 ? "true" : undefined}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
      >
        <div
          className="pk-pictures__scene"
          style={{
            transform: `translate(${String(view.x)}px, ${String(view.y)}px) scale(${String(view.scale)})`,
          }}
        >
          {"scene" in picture ? (
            <SketchScene key={picture.id} scene={picture.scene} title={picture.title} preview />
          ) : (
            <img
              className="pk-pictures__shot"
              src={picture.src}
              alt={picture.title}
              draggable={false}
            />
          )}
        </div>
      </div>
      {pictures.length < 2 && onPick === undefined ? null : (
        <footer className="pk-pictures__foot">
          <Button
            variant="ghost"
            size="sm"
            className="pk-pictures__step"
            disabled={before === undefined}
            aria-label={before === undefined ? "No take before" : `Before: ${before.title}`}
            onClick={() => onAt(at - 1)}
          >
            <ChevronLeft size={16} aria-hidden="true" />
            <span className="pk-pictures__name">{before?.title ?? ""}</span>
          </Button>
          {onPick === undefined ? (
            <span />
          ) : (
            <Button
              variant={chosen ? "tonal" : "primary"}
              size="sm"
              onClick={() => onPick(picture.id)}
            >
              {chosen ? <Check size={14} aria-hidden="true" /> : null}
              {chosen ? "Picked" : "Pick"}
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="pk-pictures__step pk-pictures__step--next"
            disabled={after === undefined}
            aria-label={after === undefined ? "No take after" : `After: ${after.title}`}
            onClick={() => onAt(at + 1)}
          >
            <span className="pk-pictures__name">{after?.title ?? ""}</span>
            <ChevronRight size={16} aria-hidden="true" />
          </Button>
        </footer>
      )}
    </div>
  );
}
