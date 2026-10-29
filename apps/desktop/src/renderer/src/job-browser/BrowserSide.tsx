// The Job browser in the side panel, and the Job's own thread and composer floating on the left over the app, as a plate off the
// sheet's leading edge (`Sheet`'s `aside`), so the page and the Job are read and answered together. The browser's toolbar folds the
// thread and brings it back. The thread is `SessionView`, the Cockpit's, as the walk window draws its Job. The sheet's own handle
// moves the split between the two: the thread keeps the width it was given and the browser takes the rest.

import { useRef } from "react";
import { Square } from "lucide-react";
import { Sheet, clampToRange, tokenPx } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { OPEN_SESSION_ATTRIBUTE, askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { useBack } from "../shared-panel/panels";
import { closePanel, type PanelItem } from "../shared-panel/store";
import { JobBrowserPane } from "./JobBrowserPane";
import { useJobBrowser } from "./store";
import { useThreadWidth } from "./thread-width";

const HELD = { held: { state: "none" }, onWant: () => {} } as const;

/** The Job beside its browser: its thread and its composer, the Cockpit's own. */
export function JobBeside({ sessionId, className }: { sessionId: string; className?: string }) {
  const session = useSessions().find((one) => one.id === sessionId);
  const draft = useSessionsDraft();
  if (session === undefined || draft === undefined) return null;
  return (
    <section className={["deck-pane", className].filter(Boolean).join(" ")} data-frame="" aria-label={`Job: ${session.title ?? session.id}`} {...{ [OPEN_SESSION_ATTRIBUTE]: session.id }}>
      <SessionView
        session={session}
        goes={{ onGoTo: () => {}, onOpenLink: () => {}, onOpenCheck: () => {}, onOpenLoadout: () => {} }}
        onOpen={askToOpenSession}
        held={HELD}
        deck={{ ledger: null, back: null, acts: draft.stop === undefined ? [] : [{ id: "stop", label: "Stop", icon: Square, run: () => draft.stop?.(session.id) }] }}
      />
    </section>
  );
}

export function BrowserSide({ item }: { item: Extract<PanelItem, { kind: "browser" }> }) {
  const browser = useJobBrowser(item.sessionId);
  const title = useSessions().find((one) => one.id === item.sessionId)?.title;
  const floor = useAtFloor();
  const back = useBack();
  const [threadWidth, setThreadWidth] = useThreadWidth();
  const beside = useRef<HTMLDivElement>(null);
  if (browser === undefined) return null;
  const shown = !browser.threadHidden;

  // The sheet's handle reports the sheet's width; the thread is whatever the two share less that, held to its floor and to the
  // browser's. The sum is read off layout since the sheet's width is the flex remainder and nothing here holds it.
  function resize(sheetWidth: number): void {
    const sheet = beside.current?.closest(".armada-sheet-scrim")?.querySelector(".armada-sheet");
    if (beside.current === null || sheet === null || sheet === undefined) return;
    const shared = beside.current.getBoundingClientRect().width + sheet.getBoundingClientRect().width;
    const min = tokenPx("--w-dock-min");
    setThreadWidth(clampToRange(shared - sheetWidth, { min, max: Math.max(min, shared - tokenPx("--w-work-min")) }));
  }

  const aside = (
    <div ref={beside} className="job-browser-beside" data-shown={shown ? "" : undefined} style={threadWidth === undefined ? undefined : { width: `${threadWidth}px` }}>
      {shown ? <JobBeside sessionId={item.sessionId} className="job-browser-beside__job" /> : null}
    </div>
  );
  return (
    <Sheet
      open
      floating
      floor={floor}
      title="Job browser"
      {...(title === undefined ? {} : { subtitle: title })}
      size="widest"
      bleed
      {...(back === undefined ? {} : { back })}
      closeLabel="Close"
      closeBinding="Esc"
      aside={aside}
      width={undefined}
      onResize={resize}
      onClose={closePanel}
    >
      <JobBrowserPane sessionId={item.sessionId} browser={browser} placed="panel" />
    </Sheet>
  );
}
