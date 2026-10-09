// Where Settings → Phone gets its state: the Gateway's admin routes, once they are wired. Nothing
// here talks to a Gateway; a mock stands behind it until then. With no source provided the Phone
// card is not drawn.

import { createContext, useContext, useSyncExternalStore } from "react";

export type PhoneGateway = { state: "running" } | { state: "not_running" } | { state: "said"; said: string };

export type PhoneState = {
  gateway: PhoneGateway;
  phones: readonly { id: string; name: string; seenAt: number }[];
  /** The pairing address with its one-time code, and when the code stops working (epoch ms). */
  pairing: { url: string; expiresAt: number } | null;
  /** A phone that posted the code and waits on Confirm. */
  claim: { device: string } | null;
};

export interface PhoneSource {
  get(): PhoneState;
  subscribe(on: () => void): () => void;
  pair(): void;
  confirm(): void;
  unpair(id: string): void;
}

const PhoneSourceContext = createContext<PhoneSource | null>(null);
export const PhoneSourceProvider = PhoneSourceContext.Provider;

/** The source this window was given, or `null` where it has none. */
export function usePhone(): { source: PhoneSource; state: PhoneState } | null {
  const source = useContext(PhoneSourceContext);
  const state = useSyncExternalStore(
    source?.subscribe ?? (() => () => undefined),
    () => source?.get() ?? null,
  );
  return source === null || state === null ? null : { source, state };
}
