// The Logbook surface, as the window draws it: every repository's Logbook, Habits and This Mac, read on open
// (`docs/concepts/logbook.md`). Apart from `App.tsx`, which is at its length.

import { Logbook } from "@armada/screens/src/logbook/Logbook";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { editLogbookEntry, forgetLogbookEntry, pinLogbookEntry, readLogbook } from "./commands";
import { askToOpenSession } from "./open-session";

export function LogbookSurface({ bridge, onCopied }: { bridge: BridgeState["bridge"]; onCopied: (value: string) => void }) {
  return (
    <Boundary region="Logbook" bridge={bridge} onCopied={onCopied}>
      <Logbook onRead={readLogbook} onEdit={editLogbookEntry} onPin={pinLogbookEntry} onForget={forgetLogbookEntry} onOpenJob={askToOpenSession} />
    </Boundary>
  );
}
