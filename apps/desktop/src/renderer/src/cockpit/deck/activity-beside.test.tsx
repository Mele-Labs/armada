// Activity rests at the height of the merge queue's top to the flight controls' foot bottom, set once on the grid.

import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import { expect, test, vi } from "vitest";

import { useActivityBeside } from "./Deck";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function Grid() {
  const grid = useRef<HTMLDivElement>(null);
  useActivityBeside(grid, "idle");
  return (
    <div ref={grid} className="deck-grid" style={{ display: "flex", flexDirection: "column", width: 300 }}>
      <div style={{ height: 30 }} />
      <div className="deck-centre" style={{ display: "block" }}>
        <div className="deck-stage" style={{ flex: "none", display: "block" }}>
          <div style={{ height: 100 }} />
          <div className="deck-queue" style={{ height: 40, minHeight: 40 }} />
        </div>
        <div className="deck-foot" style={{ display: "block", height: 60 }} />
      </div>
    </div>
  );
}

test("the grid carries the height from the merge queue's top to the foot's bottom", async () => {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<Grid />));
  const grid = host.querySelector<HTMLElement>(".deck-grid")!;
  const queue = host.querySelector<HTMLElement>(".deck-queue")!;
  const foot = host.querySelector<HTMLElement>(".deck-foot")!;
  const wanted = Math.round(foot.getBoundingClientRect().bottom - queue.getBoundingClientRect().top);
  expect(wanted).toBe(100);
  await vi.waitFor(() => expect(grid.style.getPropertyValue("--deck-activity")).toBe(`${wanted}px`));
  act(() => root.unmount());
  host.remove();
});
