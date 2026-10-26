// Where Settings → Phone gets its state: Fleet's phone switch and the Gateway's admin routes, through
// `phone-gateway-source.ts`, or the mock's in a walk. With no source provided the Phone card is not drawn.

import { createContext, useContext, useSyncExternalStore } from "react";

export type PhoneGateway = { state: "running" } | { state: "not_running" } | { state: "said"; said: string };

/**
 * How far this Mac is along the way in, as the Gateway's `/admin/setup` says it. `named` is a
 * `pocket-address` the owner wrote, a tunnel of his own, and then the Tailscale steps are not his.
 */
export type PhoneSetup = {
  tailscale: "missing" | "signed_out" | "running";
  served: "none" | "tailnet" | "funnel";
  named: boolean;
  address?: string;
  /** A press of Turn on Funnel: waiting on it, the page the tailnet needs it allowed on, or Tailscale's refusal. */
  funnel?: { state: "turning" } | { state: "approve"; page: string } | { state: "said"; said: string };
};

/** A phone can reach this Mac: a tunnel the owner named, or Funnel or `tailscale serve` in front of the Gateway. */
export const reachable = (setup: PhoneSetup | null): boolean => setup === null || setup.named || setup.served !== "none";

export type PhoneState = {
  /** Fleet's phone switch, `turning` while a press waits on Fleet. `null` until Fleet has said. */
  access: { on: boolean; turning: boolean } | null;
  gateway: PhoneGateway;
  phones: readonly { id: string; name: string; seenAt: number }[];
  /** The pairing address with its one-time code, and when the code stops working (epoch ms). */
  pairing: { url: string; expiresAt: number } | null;
  /** A phone that posted the code and waits on Confirm. */
  claim: { device: string } | null;
  /** The way in. `null` until the Gateway has said, and where it cannot. */
  setup: PhoneSetup | null;
};

export interface PhoneSource {
  get(): PhoneState;
  subscribe(on: () => void): () => void;
  /** Ask Fleet to turn phone access on or off. */
  turn(on: boolean): void;
  pair(): void;
  confirm(): void;
  unpair(id: string): void;
  /** Turn Funnel on in front of the Gateway. */
  funnel(): void;
  /** Open a page in the browser: Tailscale's download, or the tailnet's page to allow Funnel. */
  open(page: string): void;
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
