// The block the flight controls are drawn in. In the `pages` take (`flight-take.ts`) its tiles are in a strip that scrolls sideways and snaps to a
// page, as iPhone home screens do, with a dot for each page under it; in the other take it is the plain block. The dots stand outside the strip so
// they stay while it moves, in the block's own padding, drawn only while there is more than one page. A tile that takes focus or a press (a key
// presses it from anywhere) brings its page into view.

import { useEffect, useRef, useState, type ComponentProps } from "react";
import { Tooltip } from "@armada/components";

const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

export function FlightBlock({ pages, children, ...block }: { pages: number | undefined } & ComponentProps<"div">) {
  const strip = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState(0);
  const firsts = () => Array.from(strip.current?.querySelectorAll<HTMLElement>("[data-page-first]") ?? []);
  useEffect(() => {
    const el = strip.current;
    if (el === null) return;
    // The page on view is the one whose first tile stands nearest the strip's left edge, which a snap makes exact.
    const read = () => {
      const left = el.getBoundingClientRect().left;
      const away = firsts().map((one) => Math.abs(one.getBoundingClientRect().left - left));
      setAt(Math.max(0, away.indexOf(Math.min(...away))));
    };
    read();
    el.addEventListener("scroll", read, { passive: true });
    return () => el.removeEventListener("scroll", read);
  }, [pages]);
  const go = (page: number) => {
    const el = strip.current;
    const first = firsts()[page];
    if (el === null || first === undefined) return;
    el.scrollTo({ left: el.scrollLeft + first.getBoundingClientRect().left - el.getBoundingClientRect().left, behavior: reduced() ? "auto" : "smooth" });
  };
  const bring = (event: { target: EventTarget }) => {
    const page = Number((event.target as HTMLElement).closest<HTMLElement>("[data-page]")?.dataset.page);
    if (!Number.isNaN(page) && page !== at) go(page);
  };
  if (pages === undefined) return <div {...block}>{children}</div>;
  return (
    <div {...block}>
      <div ref={strip} className="deck-controls__pages" onFocusCapture={bring} onClickCapture={bring}>
        {children}
      </div>
      {pages < 2 ? null : (
        <div className="deck-controls__dots">
          {Array.from({ length: pages }, (_, page) => (
            <Tooltip key={page} label={`Page ${page + 1}`} asChild>
              <button type="button" aria-label={`Page ${page + 1}`} aria-current={at === page || undefined} onClick={() => go(page)}>
                <span />
              </button>
            </Tooltip>
          ))}
        </div>
      )}
    </div>
  );
}
