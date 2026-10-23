// A chevron at the foot of the flight controls while tiles sit below the two rows in view: the `peek` take (`flight-take.ts`). It is drawn
// as the last child of the block that scrolls, stuck to the block's foot, and pressing it scrolls the block to its end. It carries no count.

import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

export function MoreBelow() {
  const here = useRef<HTMLDivElement>(null);
  const [more, setMore] = useState(false);
  useEffect(() => {
    const block = here.current?.parentElement;
    if (!block) return;
    const read = () => setMore(block.scrollTop + block.clientHeight < block.scrollHeight - 1);
    read();
    block.addEventListener("scroll", read, { passive: true });
    // The block resizes with the panel and its content with the tiles drawn, so both are watched.
    const watch = new ResizeObserver(read);
    watch.observe(block);
    for (const child of Array.from(block.children)) watch.observe(child);
    return () => {
      block.removeEventListener("scroll", read);
      watch.disconnect();
    };
  }, []);
  const down = () => {
    const block = here.current?.parentElement;
    block?.scrollTo({ top: block.scrollHeight, behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth" });
  };
  return (
    <div ref={here} className="deck-controls__more">
      {!more ? null : (
        <Tooltip label="More controls" asChild>
          <Button variant="ghost" size="sm" iconOnly aria-label="More controls" onClick={down}>
            <ChevronDown size={12} aria-hidden="true" />
          </Button>
        </Tooltip>
      )}
    </div>
  );
}
