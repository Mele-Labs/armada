// What stands in for Bridge's own window on a Job's server, in a browser that
// has no second window to open: the capture window's bar, with no Studio, over
// the page it would load. `prototype-fleet.ts` opens it. Dev-only.
//
// **A window, so the app beside it stays usable**: dragged by its strip and
// resized from its corner, and nothing under it is blocked — the real one is
// a window of its own, and the owner moves between Jobs with it open.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Button, CaptureBar } from "@armada/components";

let held: { root: Root; host: HTMLElement } | null = null;

/** Close what is open. */
export function closeWalkWindow(): void {
  held?.root.unmount();
  held?.host.remove();
  held = null;
}

/**
 * Open the stand-in on `url`, replacing one already open — main raises the
 * one window per server, and a page has nothing to raise.
 *
 * **No page under test.** A frame of the tester's own page would start the
 * suite again inside itself, so a test sees the bar and an empty frame.
 */
export function openWalkWindow(run: string, url: string): void {
  closeWalkWindow();
  const host = document.createElement("div");
  host.className = "armada-mock-walk-window";
  document.body.append(host);
  const root = createRoot(host);
  const testing = (import.meta as { env?: { MODE?: string } }).env?.MODE === "test";
  root.render(
    <StrictMode>
      <div role="dialog" aria-label={`Bridge's window on ${run}`} className="armada-mock-walk-window__frame">
        <div className="armada-mock-walk-window__strip" onPointerDown={dragging(host)}>
          <span>{run} — drag to move, corner to resize</span>
          <Button variant="secondary" size="sm" onClick={closeWalkWindow}>
            Close window
          </Button>
        </div>
        <CaptureBar
          run={run}
          address={new URL(url).origin}
          studio={null}
          serving
          armed={false}
          framesRefused={0}
          onArm={() => {}}
          onReload={() => {
            const frame = host.querySelector("iframe");
            if (frame !== null && !testing) frame.src = url;
          }}
          onFollowRefused={() => {}}
          binding={[]}
        />
        <iframe title={`${run}, served`} src={testing ? "about:blank" : url} />
      </div>
    </StrictMode>,
  );
  held = { root, host };
}

/** Move the window by its strip. The frame is under the pointer too, so it is told to let go of it. */
function dragging(host: HTMLElement) {
  return (down: React.PointerEvent<HTMLDivElement>) => {
    if ((down.target as HTMLElement).closest("button") !== null) return;
    const frame = host.querySelector<HTMLElement>(".armada-mock-walk-window__frame");
    if (frame === null) return;
    const from = frame.getBoundingClientRect();
    const start = { x: down.clientX - from.left, y: down.clientY - from.top };
    frame.style.pointerEvents = "none";
    const move = (at: PointerEvent) => {
      frame.style.left = `${Math.max(0, at.clientX - start.x)}px`;
      frame.style.top = `${Math.max(0, at.clientY - start.y)}px`;
      frame.style.right = "auto";
    };
    const up = () => {
      frame.style.pointerEvents = "";
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
}
