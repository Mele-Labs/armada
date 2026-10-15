// The account this window is signed in to, from Fleet. The provider is always there, so the read that
// follows connecting does not remount the app; until one lands, no account is offered and none is drawn.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { AccountProvider } from "@armada/components";

import type { AccountSource } from "./fleet-account";

export function AccountFromFleet({ source, children }: { source: AccountSource; children: ReactNode }) {
  const held = useSyncExternalStore(source.subscribe, source.get);
  return <AccountProvider value={held?.account}>{children}</AccountProvider>;
}
