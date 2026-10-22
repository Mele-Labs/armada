// The top of Progress while a turn runs: its running account in his words, and under it the page it is building. The view is a button over
// the frame, so a press or Enter opens the page large in the reading sheet and Esc closes it; while it is large the small frame is not drawn.

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { Maximize2 } from "lucide-react";
import { Tooltip, TurnAccountList, TurnLiveView, type TurnAccount } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { PageFrame } from "../session-artifact";
import { useSessionsDraft } from "../sessions-draft";

/** Whether the page is open large, and what opens it. */
export type LiveOpen = { open: boolean; onOpen: (url: string) => void };

/** The small frame has nothing to close: Esc there is the sheet's. */
const NO_CLOSE = (): void => undefined;

/** The size the page is laid out at; the frame draws all of it, scaled down to the width the panel gives. */
const DESIGN = { width: 800, height: 440 } as const;

/** The whole page at its design size, scaled to the frame's width so nothing is cut off and the frame's height follows. The press over it opens the page. */
function ScaledPage({ shown, title, onOpen, children }: { shown: boolean; title: string; onOpen: () => void; children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = box.current;
    if (el === null) return;
    setWidth(el.clientWidth);
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(() => setWidth(el.clientWidth));
    watch.observe(el);
    return () => watch.disconnect();
  }, []);
  const scale = width / DESIGN.width;
  return (
    <div ref={box} className="sv-account__scaled" style={{ height: DESIGN.height * scale }}>
      {shown ? (
        <div className="sv-account__frame" inert style={{ width: DESIGN.width, height: DESIGN.height, transform: `scale(${scale})` }}>
          {children}
        </div>
      ) : null}
      <button type="button" className="sv-account__open" aria-label={`Open ${title}`} onClick={onOpen}>
        <span className="sv-account__open-corner">
          <Tooltip label="Open">
            <span className="sv-account__open-mark" aria-hidden>
              <Maximize2 size={14} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        </span>
      </button>
    </div>
  );
}

export function TurnAccountBlock({ session, lines, live }: { session: Session; lines: TurnAccount; live: LiveOpen | undefined }) {
  const draft = useSessionsDraft();
  const page = session.live;
  return (
    <div className="sv-account">
      <TurnAccountList lines={lines} />
      {page === undefined || live === undefined ? null : (
        <div className="sv-account__live">
          <TurnLiveView
            live={{
              title: page.title,
              view: (
                <ScaledPage shown={!live.open} title={page.title} onOpen={() => live.onOpen(page.url)}>
                  <PageFrame draft={draft} sessionId={session.id} address={page.url} label={page.title} onClose={NO_CLOSE} />
                </ScaledPage>
              ),
            }}
          />
        </div>
      )}
    </div>
  );
}
