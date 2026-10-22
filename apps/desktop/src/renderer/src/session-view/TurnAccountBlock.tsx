// The top of Progress while a turn runs: its running account in his words, and under it the page it is building. The view is a button over
// the frame, so a press or Enter opens the page large in the reading sheet and Esc closes it; while it is large the small frame is not drawn.

import { TurnAccountList, TurnLiveView, type TurnAccount } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { PageFrame } from "../session-artifact";
import { useSessionsDraft } from "../sessions-draft";

/** Whether the page is open large, and what opens it. */
export type LiveOpen = { open: boolean; onOpen: (url: string) => void };

/** The small frame has nothing to close: Esc there is the sheet's. */
const NO_CLOSE = (): void => undefined;

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
              view: live.open ? null : (
                <div className="sv-account__frame" inert>
                  <PageFrame draft={draft} sessionId={session.id} address={page.url} label={page.title} onClose={NO_CLOSE} />
                </div>
              ),
            }}
          />
          <button type="button" className="sv-account__open" aria-label={`Open ${page.title}`} onClick={() => live.onOpen(page.url)} />
        </div>
      )}
    </div>
  );
}
