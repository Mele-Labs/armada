import { useLayoutEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";

import type { NoteAbout, NoteRegion } from "./compose";
import { NoteNumber, useViewerNotes, type ViewerNotesScope } from "./ViewerNotes";

type Region = NoteRegion;

/** Under this many pixels a drag is a press, and draws no region. */
const SLOP = 4;

/**
 * The part of `image` inside `region`, as a PNG `data:` address: the whole picture without one. Taken at the picture's own
 * size, or at the size it is drawn where that is larger, so a small picture shown large is not sent blurred.
 */
function cropOf(image: HTMLImageElement, region: Region | undefined): string {
  const natural = { w: image.naturalWidth || image.width, h: image.naturalHeight || image.height };
  const scale = Math.max(1, image.width / Math.max(1, natural.w));
  const r = region ?? { x: 0, y: 0, w: 1, h: 1 };
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(r.w * natural.w * scale));
  canvas.height = Math.max(1, Math.round(r.h * natural.h * scale));
  canvas.getContext("2d")?.drawImage(image, r.x * natural.w, r.y * natural.h, r.w * natural.w, r.h * natural.h, 0, 0, canvas.width, canvas.height);
  try {
    return canvas.toDataURL("image/png");
  } catch {
    // A picture from another origin cannot be read back; the note goes without it.
    return "";
  }
}

const inside = (region: Region, x: number, y: number) => x >= region.x && x <= region.x + region.w && y >= region.y && y <= region.y + region.h;

const percent = (n: number) => `${String(n * 100)}%`;

/**
 * A picture that takes notes: drag a region and right-press it, or right-press anywhere for the whole picture. Held
 * notes stand on it as numbered rectangles; a note on the whole picture is a number in its corner. **Drawn beside the
 * `img` and not around it**, so the picture keeps the layout its host gave it: the host's box must be positioned.
 */
export function NotableImage({
  src,
  name,
  path,
  marked,
  className,
  notes,
}: {
  src: string;
  name: string;
  /** Where the picture is read from, which a note names it by. */
  path?: string;
  /** A region a sent note named, drawn when it opens. */
  marked?: Region;
  className?: string;
  notes?: ViewerNotesScope;
}) {
  const image = useRef<HTMLImageElement>(null);
  const viewer = useViewerNotes(notes, image);
  const [region, setRegion] = useState<Region | undefined>(marked);
  const drag = useRef<{ x: number; y: number } | undefined>(undefined);
  // Where the picture is drawn in its positioned box, which the marks are laid over.
  const [box, setBox] = useState<{ left: number; top: number; width: number; height: number } | undefined>(undefined);

  useLayoutEffect(() => {
    const el = image.current;
    if (el === null || !viewer.on) return;
    const measure = () => setBox({ left: el.offsetLeft, top: el.offsetTop, width: el.offsetWidth, height: el.offsetHeight });
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    if (el.parentElement !== null) watch.observe(el.parentElement);
    el.addEventListener("load", measure);
    return () => {
      watch.disconnect();
      el.removeEventListener("load", measure);
    };
  }, [viewer.on]);

  // A region held as a note is drawn as the note's mark from then on, and a sent one is gone with it.
  const settled = useRef(viewer.settled);
  useLayoutEffect(() => {
    if (settled.current === viewer.settled) return;
    settled.current = viewer.settled;
    setRegion(undefined);
  }, [viewer.settled]);
  const writing = viewer.marks.some((one) => one.writing);

  const at = (event: { clientX: number; clientY: number }) => {
    const rect = image.current!.getBoundingClientRect();
    return { x: Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width)), y: Math.min(1, Math.max(0, (event.clientY - rect.top) / rect.height)) };
  };

  const aboutAt = (point: { x: number; y: number } | undefined): NoteAbout => {
    const kept = region !== undefined && (point === undefined || inside(region, point.x, point.y)) ? region : undefined;
    return { kind: "image", name, ...(path === undefined ? {} : { path }), ...(kept === undefined ? {} : { region: kept }), crop: cropOf(image.current!, kept) };
  };
  /** Where the note's subject is on screen: the region, or the whole picture. */
  const anchorOf = (about: NoteAbout) => {
    const rect = image.current!.getBoundingClientRect();
    const r = about.kind === "image" && about.region !== undefined ? about.region : { x: 0, y: 0, w: 1, h: 1 };
    return { left: rect.left + r.x * rect.width, top: rect.top + r.y * rect.height, right: rect.left + (r.x + r.w) * rect.width, bottom: rect.top + (r.y + r.h) * rect.height };
  };
  const menu = (point: { x: number; y: number }, about: NoteAbout) => viewer.menu(point, about, anchorOf(about));

  if (!viewer.on) return <img ref={image} className={className} src={src} alt={name} />;

  const down = (event: ReactPointerEvent<HTMLImageElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    drag.current = at(event);
  };
  const move = (event: ReactPointerEvent<HTMLImageElement>) => {
    const from = drag.current;
    if (from === undefined) return;
    const to = at(event);
    setRegion({ x: Math.min(from.x, to.x), y: Math.min(from.y, to.y), w: Math.abs(to.x - from.x), h: Math.abs(to.y - from.y) });
  };
  const up = (event: ReactPointerEvent<HTMLImageElement>) => {
    const from = drag.current;
    drag.current = undefined;
    if (from === undefined) return;
    const to = at(event);
    const rect = event.currentTarget.getBoundingClientRect();
    // A press with no drag clears the region.
    if (Math.abs(to.x - from.x) * rect.width < SLOP && Math.abs(to.y - from.y) * rect.height < SLOP) setRegion(undefined);
  };

  const held = viewer.marks.filter((one) => one.about.kind === "image");
  const whole = held.filter((one) => one.about.kind === "image" && one.about.region === undefined);

  return (
    <>
      <img
        ref={image}
        className={className}
        src={src}
        alt={name}
        draggable={false}
        tabIndex={0}
        data-notable=""
        // A region drawn is let go of with Esc, before anything it lies in hears it.
        data-keeps-esc={region !== undefined || undefined}
        aria-keyshortcuts="Shift+F10 ContextMenu Meta+Enter"
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        // A press on a picture that takes notes is not a press on what it lies in: an enlarged one stays open.
        onClick={(event) => event.stopPropagation()}
        onContextMenu={(event) => {
          event.preventDefault();
          event.stopPropagation();
          menu({ x: event.clientX, y: event.clientY }, aboutAt(at(event)));
        }}
        onKeyDown={(event) => {
          viewer.onKeyDown(event);
          if (event.key === "Escape" && region !== undefined) {
            event.stopPropagation();
            setRegion(undefined);
          } else if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
            event.preventDefault();
            const rect = event.currentTarget.getBoundingClientRect();
            const point = region === undefined ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : { x: rect.left + (region.x + region.w) * rect.width, y: rect.top + (region.y + region.h) * rect.height };
            menu(point, aboutAt(undefined));
          }
        }}
      />
      {box === undefined ? null : (
        <span className="armada-notable-image__marks" style={{ left: box.left, top: box.top, width: box.width, height: box.height }} aria-hidden={held.length === 0 && region === undefined}>
          {region === undefined || writing ? null : <span className="armada-notable-image__region" data-drawing="" style={{ left: percent(region.x), top: percent(region.y), width: percent(region.w), height: percent(region.h) }} />}
          {held.map((one) =>
            one.about.kind !== "image" || one.about.region === undefined ? null : (
              <span
                key={one.n}
                className="armada-notable-image__region"
                data-writing={one.writing || undefined}
                style={{ left: percent(one.about.region.x), top: percent(one.about.region.y), width: percent(one.about.region.w), height: percent(one.about.region.h) }}
              >
                <NoteNumber n={one.n} writing={one.writing} said={one.text} />
              </span>
            ),
          )}
          {whole.length === 0 ? null : (
            <span className="armada-notable-image__whole">
              {whole.map((one) => (
                <NoteNumber key={one.n} n={one.n} writing={one.writing} said={one.text} />
              ))}
            </span>
          )}
        </span>
      )}
      {viewer.layer}
    </>
  );
}
