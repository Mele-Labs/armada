// Where Settings → Phone gets its state: Fleet's phone switch and the Gateway's admin routes, through
// `phone-gateway-source.ts`, or the mock's in a walk. With no source provided the Phone card is not drawn.

import { createContext, useContext, useSyncExternalStore } from "react";

export type PhoneGateway = { state: "running" } | { state: "not_running" } | { state: "said"; said: string };

export type PhoneState = {
  /** Fleet's phone switch, `turning` while a press waits on Fleet. `null` until Fleet has said. */
  access: { on: boolean; turning: boolean } | null;
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
  /** Ask Fleet to turn phone access on or off. */
  turn(on: boolean): void;
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
