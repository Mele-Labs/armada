// Web Push. The phone holds one subscription, made with its first Mac's VAPID key. The first Mac is
// given it as it is; every other Mac is given it with that key's private half, read from the first
// Mac (`GET /api/push/vapid_private`), so each Mac pushes to the phone itself.
// iOS grants push only to a Home Screen app, so anywhere else this does nothing.

import { call, getJson } from "./client";
import type { Machine } from "./machines";

export const bytesOf = (base64url: string): Uint8Array<ArrayBuffer> => {
  const padded = base64url.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(base64url.length / 4) * 4, "=");
  return Uint8Array.from(atob(padded), (char) => char.charCodeAt(0));
};

export const standalone = (): boolean =>
  window.matchMedia("(display-mode: standalone)").matches || (navigator as { standalone?: boolean }).standalone === true;

/** What `POST /api/push/subscribe` is sent: the subscription, and for a Mac but the first, the key to sign with. */
export async function subscriptionFor(machine: Machine, first: Machine, subscription: PushSubscriptionJSON): Promise<PushSubscriptionJSON & { vapid_private?: string }> {
  if (machine.origin === first.origin) return subscription;
  const { vapid_private } = await getJson<{ vapid_private: string }>(first, "/api/push/vapid_private");
  return { ...subscription, vapid_private };
}

/** Subscribes the phone, once, and gives the subscription to each Mac in `macs`. `macs[0]` is the first Mac. */
export async function subscribe(macs: readonly Machine[]): Promise<void> {
  const first = macs[0];
  if (first === undefined || !("serviceWorker" in navigator) || !("PushManager" in window) || !standalone()) return;
  if ((await Notification.requestPermission()) !== "granted") return;
  const registration = await navigator.serviceWorker.ready;
  let subscription = await registration.pushManager.getSubscription();
  if (subscription === null) {
    const { public_key: key } = await getJson<{ public_key: string }>(first, "/api/push/key");
    subscription = await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytesOf(key) });
  }
  const json = subscription.toJSON();
  await Promise.all(macs.map(async (machine) => call(machine, "/api/push/subscribe", { method: "POST", body: await subscriptionFor(machine, first, json) }).catch(() => undefined)));
}
