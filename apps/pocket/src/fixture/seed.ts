// Fixture mode only: pairs the phone with the fixture Gateway's two Macs, the one serving the app and
// the one beside it on the next port, so the app opens on a list rather than on Pair.

import {
  forgetMachine,
  forgetStoreSignIn,
  keepMachine,
  keepStoreSignIn,
  machines,
} from "../machines";
import { STORE_AT, beyondOf } from "./store";

export const besideOf = (origin: string): string => {
  const url = new URL(origin);
  url.port = String(Number(url.port || "80") + 1);
  return url.origin;
};

/**
 * Every load starts the store's path over: signed out, and the mini not paired. `?signed_in` starts it signed in instead,
 * on the list of Macs.
 */
async function startOver(here: string): Promise<void> {
  await forgetMachine(beyondOf(here, 2));
  await forgetStoreSignIn();
  if (!new URLSearchParams(window.location.search).has("signed_in")) return;
  const pair = await crypto.subtle.generateKey(
    {
      name: "ECDSA",
      namedCurve: "P-256",
    },
    false,
    [
      "sign",
      "verify",
    ],
  );
  await keepStoreSignIn({
    store: `${here}${STORE_AT}`,
    deviceId: "7001.phone",
    key: pair.privateKey,
  });
}

export async function seed(): Promise<void> {
  await startOver(window.location.origin);
  if ((await machines()).length > 0) return;
  const here = window.location.origin;
  for (const [origin, name] of [
    [
      here,
      "studio",
    ],
    [
      besideOf(here),
      "laptop",
    ],
  ] as const) {
    const pair = await crypto.subtle.generateKey(
      {
        name: "ECDSA",
        namedCurve: "P-256",
      },
      false,
      [
        "sign",
        "verify",
      ],
    );
    await keepMachine({
      origin,
      name,
      deviceId: `fixture-${name}`,
      key: pair.privateKey,
    });
  }
}
