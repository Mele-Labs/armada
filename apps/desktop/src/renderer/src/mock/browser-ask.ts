// A page's ask in the Job browser, as the Job's Needs you item it is in the app: Fleet raises it and the person answers it there. **The mock** has
// the browser store hear the page and puts the item on the Job through the same `waitingFor` a Fleet item rides, so the row pulses and the panel
// draws it as it draws any other. Allow and Block settle the request; dismissing refuses it now and keeps nothing.

import type { WaitingItem } from "@armada/protocol";

import {
  answerRequest,
  dismissRequest,
  watchBrowserAsks,
  type BrowserAsk,
} from "../job-browser/store";
import { labelOfPermission } from "../../../shared/site-permissions";
import type { SessionsStore } from "./sessions/script";

/** The id of a page's ask: the tab that asked follows. */
const PREFIX = "browser_ask:";

const RISK = "It can reach apps on this Mac and devices on your network.";

/** The Needs you item for a page's ask. `fleet_call` is the nearest source the wire has: a call the person takes off with no word to the agent. */
export const browserAskItem = (ask: BrowserAsk, since: string): WaitingItem => ({
  id: `${PREFIX}${ask.tabId}`,
  text: `Allow ${labelOfPermission(ask.permission)} for ${ask.host}?`,
  since,
  source: "fleet_call",
  act: {
    kind: "answer",
    target: `${PREFIX}${ask.tabId}`,
  },
  options: [
    {
      label: "Allow",
      ...(ask.permission === "local_network"
        ? {
            description: RISK,
          }
        : {}),
    },
    {
      label: "Block",
    },
  ],
});

/**
 * Keeps each Job's ask in its `waitingFor` as the browser store hears it, and takes the person's answer or dismissal back to the store.
 * `answer` and `drop` say whether the item was theirs, so a scenario's own routes handle every other.
 */
export function serveBrowserAsks(store: SessionsStore): {
  answer: (id: string, itemId: string, choice: number | undefined) => boolean;
  drop: (id: string, itemId: string) => boolean;
  stop: () => void;
} {
  const stop = watchBrowserAsks((id, ask) =>
    store.edit(id, (one) => ({
      ...one,
      waitingFor: [
        ...(one.waitingFor ?? []).filter((it) => !it.id.startsWith(PREFIX)),
        ...(ask === null
          ? []
          : [
              browserAskItem(ask, new Date().toISOString()),
            ]),
      ],
    })),
  );
  return {
    answer: (id, itemId, choice) => {
      if (!itemId.startsWith(PREFIX)) return false;
      const tab = itemId.slice(PREFIX.length);
      // The ask card's own Other has no word the page can take, so it refuses the request now.
      if (choice === 0) answerRequest(id, tab, "allow");
      else if (choice === 1) answerRequest(id, tab, "block");
      else dismissRequest(id, tab);
      return true;
    },
    drop: (id, itemId) => {
      if (!itemId.startsWith(PREFIX)) return false;
      dismissRequest(id, itemId.slice(PREFIX.length));
      return true;
    },
    stop,
  };
}
