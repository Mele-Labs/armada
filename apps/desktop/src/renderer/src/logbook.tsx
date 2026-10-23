// The Logbook surface, as the window draws it: every repository's Logbook, Habits and This Mac, read on open, and
// Armada fixes below them (`docs/concepts/logbook.md`). Apart from `App.tsx`, which is at its length.

import { useCallback, useState } from "react";

import { Logbook } from "@armada/screens/src/logbook/Logbook";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { editLogbookEntry, forgetLogbookEntry, lockLogbookEntry, readLogbook } from "./commands";
import { ArmadaFixes } from "./lessons";
import { takeFixesAsked, useFixesAsked } from "./open-retro";
import { askToOpenSession } from "./open-session";

export function LogbookSurface({
  repository,
  bridge,
  onCopied,
}: {
  repository: string | null;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
}) {
  // Armada fixes comes into view when something asked for it before this page opened, and again when it is asked of the open page.
  const [toFixes, setToFixes] = useState(() => (takeFixesAsked() ? 1 : 0));
  useFixesAsked(useCallback(() => setToFixes((was) => was + 1), []));
  return (
    <Boundary region="Logbook" bridge={bridge} onCopied={onCopied}>
      <Logbook
        onRead={readLogbook}
        onEdit={editLogbookEntry}
        onLock={lockLogbookEntry}
        onForget={forgetLogbookEntry}
        onOpenJob={askToOpenSession}
        fixes={<ArmadaFixes repository={repository} />}
        toFixes={toFixes}
      />
    </Boundary>
  );
}
