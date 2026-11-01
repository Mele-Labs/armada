// The open Job's area as panes: the tree in `tree.ts` drawn side by side and one over another, each pane a strip of tabs over a body, with a handle on every line between two to drag.
// Every pane is placed by its share of the area, so splitting or resizing moves the panes without drawing any again: a thread keeps what is typed in it.

import { Fragment, useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { DockHandle } from "@armada/components";

import { LayoutControl } from "./LayoutControl";
import { PaneView } from "./PaneView";
import { usePaneKeys } from "./keys";
import { droppableGap, measured, rearrange, useJobPanes } from "./store";
import { endZoom, useZoomBox, usePaneZoomed } from "./zoom";
import { RATIO_MIN, hasKind, minExtent, moveTabToGap, panesOf, rectsOf, resizeSplit, type Pane, type Tab } from "./tree";
import "./job-panes.css";

const byCreation = (a: Pane, b: Pane) => Number(a.id.slice(1)) - Number(b.id.slice(1));

/** The area's size in px, kept as it changes. */
function useSize() {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const measure = () => setSize({ w: element.clientWidth, h: element.clientHeight });
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const watching = new ResizeObserver(measure);
    watching.observe(element);
    return () => watching.disconnect();
  }, []);
  return [ref, size] as const;
}

const pct = (share: number) => `${share * 100}%`;

/** The title bar's place for the open Job's controls, found once the bar is drawn. Where there is no bar, the controls stand over the panes. */
function useJobSlot(): Element | null {
  const [slot, setSlot] = useState<Element | null>(null);
  useLayoutEffect(() => setSlot(document.querySelector("[data-job-slot]")), []);
  return slot;
}

/**
 * `body` draws the tab in front of each pane. `bar` is what stands beside the Layout control in the title bar, and `over` is laid across the panes, as a view the Job opens is.
 */
export function JobPanes({ jobId, body, bar, over }: { jobId: string; body: (tab: Tab) => ReactNode; bar?: ReactNode; over?: ReactNode }) {
  const { arrangement, layout, edited, layouts, forAll } = useJobPanes(jobId);
  const [menu, setMenu] = useState(false);
  const [naming, setNaming] = useState(false);
  const [dragging, setDragging] = useState<string | undefined>(undefined);
  // The split whose gap a dragged tab is over, lit while it can take the tab.
  const [lit, setLit] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (dragging === undefined) setLit(undefined);
  }, [dragging]);
  const [area, size] = useSize();
  // The Layout menu ends a zoom, and so does leaving the Job.
  const openMenu = useCallback(() => (endZoom(), setMenu(true)), []);
  useEffect(() => endZoom, [jobId]);
  const zoomedId = usePaneZoomed(jobId);
  const zoomed = zoomedId !== undefined && panesOf(arrangement.root).some((one) => one.id === zoomedId) ? zoomedId : undefined;
  const box = useZoomBox(zoomed !== undefined, 0, area);
  usePaneKeys(jobId, openMenu);
  const slot = useJobSlot();
  useEffect(() => measured(jobId, size), [jobId, size]);

  const { panes: rects, splits } = rectsOf(arrangement.root);
  const thread = hasKind(arrangement, "thread");

  const controls = (
    <>
      <LayoutControl jobId={jobId} layout={layout} edited={edited} layouts={layouts} forAll={forAll} open={menu} onOpen={setMenu} naming={naming} onNaming={setNaming} />
      {bar}
    </>
  );

  return (
    <div className="job-panes">
      {slot === null ? <div className="job-panes__bar">{controls}</div> : createPortal(controls, slot)}
      <div className="job-panes__area" ref={area}>
        {panesOf(arrangement.root)
          .sort(byCreation)
          .map((pane) => {
            const rect = rects.get(pane.id)!;
            const active = pane.tabs.find((tab) => tab.id === pane.active);
            return (
              <div
                key={pane.id}
                className="job-panes__cell"
                data-zoomed={pane.id === zoomed || undefined}
                style={pane.id === zoomed && box !== undefined ? { position: "fixed", left: box.left, top: box.top, width: box.width, height: box.height, zIndex: 20 } : { left: pct(rect.x), top: pct(rect.y), width: pct(rect.w), height: pct(rect.h) }}
              >
                <PaneView jobId={jobId} pane={pane} focused={arrangement.focus === pane.id} zoomed={pane.id === zoomed} thread={thread} dragging={dragging} gapLit={lit !== undefined} onDrag={setDragging}>
                  {active === undefined ? null : body(active)}
                </PaneView>
              </div>
            );
          })}
        {size.w === 0 || zoomed !== undefined
          ? null
          : splits.map((split) => {
              const across = split.dir === "column";
              const extent = (across ? split.rect.h * size.h : split.rect.w * size.w) || 1;
              const first = extent * split.ratio;
              // Each side keeps the room its panes need, or the share the ratio keeps where the area is too small for that.
              const low = Math.max(extent * RATIO_MIN, minExtent(split.a, split.dir));
              const high = extent - Math.max(extent * RATIO_MIN, minExtent(split.b, split.dir));
              const at = across ? split.rect.y + split.rect.h * split.ratio : split.rect.x + split.rect.w * split.ratio;
              const place = across ? { top: `calc(${pct(at)} - var(--space-1))`, left: pct(split.rect.x), width: pct(split.rect.w) } : { left: `calc(${pct(at)} - var(--space-1))`, top: pct(split.rect.y), height: pct(split.rect.h) };
              const gap = across ? { top: pct(at), left: pct(split.rect.x), width: pct(split.rect.w) } : { left: pct(at), top: pct(split.rect.y), height: pct(split.rect.h) };
              return (
                <Fragment key={split.id}>
                  <div className="job-panes__line" data-axis={across ? "y" : "x"} style={place}>
                    <DockHandle
                      quiet
                      axis={across ? "y" : "x"}
                      grows="right"
                      label="the panes"
                      width={first}
                      min={low <= high ? low : first}
                      max={low <= high ? high : first}
                      onResize={(px) => rearrange(jobId, (now) => resizeSplit(now, split.id, px / extent), `resize ${split.id}`)}
                      onReset={() => rearrange(jobId, (now) => resizeSplit(now, split.id, 0.5))}
                    />
                  </div>
                  {dragging === undefined ? null : (
                    <div
                      className="job-panes__gap"
                      data-axis={across ? "y" : "x"}
                      data-lit={lit === split.id || undefined}
                      style={gap}
                      aria-hidden="true"
                      onDragOver={(event) => {
                        if (droppableGap(jobId, dragging, split.id)) {
                          event.preventDefault();
                          setLit(split.id);
                        } else event.dataTransfer.dropEffect = "none";
                      }}
                      onDragLeave={() => setLit((now) => (now === split.id ? undefined : now))}
                      onDrop={(event) => {
                        const id = event.dataTransfer.getData("text/plain");
                        event.preventDefault();
                        setLit(undefined);
                        setDragging(undefined);
                        if (id !== "") rearrange(jobId, (now) => moveTabToGap(now, id, split.id));
                      }}
                    >
                      <div className="job-panes__gap-bar" />
                    </div>
                  )}
                </Fragment>
              );
            })}
        {over === undefined || over === null ? null : <div className="job-panes__over">{over}</div>}
      </div>
    </div>
  );
}
