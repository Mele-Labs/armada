// The account this window is signed in to, from Fleet, and the shared home Kit is marked against. The providers are
// always there, so the read that follows connecting does not remount the app; until one lands, no account and no home
// is offered and none is drawn. What syncs is drawn here as the mock draws it, beside the window.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { AccountProvider, SharedHomeProvider, WhatSyncsSheet } from "@armada/components";

import type { AccountSource } from "./fleet-account";
import type { SharedHomeSource } from "./fleet-shared-home";

export function AccountFromFleet({ source, home, children }: { source: AccountSource; home: SharedHomeSource; children: ReactNode }) {
  const held = useSyncExternalStore(source.subscribe, source.get);
  const shared = useSyncExternalStore(home.subscribe, home.get);
  return (
    <AccountProvider value={held?.account}>
      <SharedHomeProvider value={shared}>
        {children}
        <WhatSyncsSheet />
      </SharedHomeProvider>
    </AccountProvider>
  );
}
