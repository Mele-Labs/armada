// Web Push, after pairing. The routes come from #1996:
// GET /api/push/key -> {public_key} (base64url uncompressed P-256 point), POST /api/push/subscribe <PushSubscription.toJSON()>.
// iOS grants push only to a Home Screen app, so anywhere else this does nothing.

import { call, getJson } from "./client";

export const bytesOf = (base64url: string): Uint8Array<ArrayBuffer> => {
  const padded = base64url.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(base64url.length / 4) * 4, "=");
  return Uint8Array.from(atob(padded), (char) => char.charCodeAt(0));
};

export const standalone = (): boolean =>
  window.matchMedia("(display-mode: standalone)").matches || (navigator as { standalone?: boolean }).standalone === true;

export async function subscribe(): Promise<void> {
  if (!("serviceWorker" in navigator) || !("PushManager" in window) || !standalone()) return;
  if ((await Notification.requestPermission()) !== "granted") return;
  const registration = await navigator.serviceWorker.ready;
  const { public_key: key } = await getJson<{ public_key: string }>("/api/push/key");
  const subscription =
    (await registration.pushManager.getSubscription()) ??
    (await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytesOf(key) }));
  await call("/api/push/subscribe", { method: "POST", body: subscription.toJSON() });
}
