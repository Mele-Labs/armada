// Fixture mode only: pairs the phone with the fixture Gateway's two Macs, the one serving the app and
// the one beside it on the next port, so the app opens on a list rather than on Pair.

import { keepMachine, machines } from "../machines";

export const besideOf = (origin: string): string => {
  const url = new URL(origin);
  url.port = String(Number(url.port || "80") + 1);
  return url.origin;
};

export async function seed(): Promise<void> {
  if ((await machines()).length > 0) return;
  const here = window.location.origin;
  for (const [origin, name] of [[here, "studio"], [besideOf(here), "laptop"]] as const) {
    const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]);
    await keepMachine({ origin, name, deviceId: `fixture-${name}`, key: pair.privateKey });
  }
}
