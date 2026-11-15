// The fixture's store and its pairing routes, so the whole path from Sign in with GitHub to a paired Mac plays with no Mac
// and no store behind it. The store answers under `/fixture-store/v1/` on the server that serves the app, and a Mac's
// `GET /pair/store` names it. Like the rest of the fixture it checks that a call is signed, not that the signature is good.

import type { Answered } from "./gateway";
import type { FixtureMac } from "./wire";

export const STORE_AT = "/fixture-store";
const POLL_SECONDS = 3;

/** An address `steps` ports above `origin`: the fixture's Macs are on consecutive ports. */
export const beyondOf = (origin: string, steps: number): string => {
  const url = new URL(origin);
  url.port = String(Number(url.port || "80") + steps);
  return url.origin;
};

const json = (value: unknown, status = 200): Answered => ({
  status,
  type: "application/json",
  body: JSON.stringify(value),
});
const refused = (status: number, error: string): Answered =>
  json(
    {
      error,
    },
    status,
  );

/** How many times each device code has been polled. A sign in is a code, two pending answers, then signed in. */
const polled = new Map<string, number>();
let codes = 0;

/** His Macs as the store lists them: studio is paired on the phone already, mini vouches, desk does not. */
const peersAt = (here: string) => [
  {
    device: "7001.studio",
    machine: "studio",
    kind: "mac",
    address: here,
    public_key: "AAAA",
  },
  {
    device: "7001.mini",
    machine: "mini",
    kind: "mac",
    address: beyondOf(here, 2),
    public_key: "AAAA",
  },
  {
    device: "7001.desk",
    machine: "desk",
    kind: "mac",
    address: beyondOf(here, 3),
    public_key: "AAAA",
  },
  // A Mac with no address and a phone: listed by the store, left off the phone's list.
  {
    device: "7001.attic",
    machine: "attic",
    kind: "mac",
    address: "",
    public_key: "AAAA",
  },
  {
    device: "7001.tablet",
    machine: "tablet",
    kind: "phone",
    address: "",
    public_key: "AAAA",
  },
];

/**
 * What the fixture answers outside a Mac's own `/api/` routes, or undefined where `target` is not one of them: the Mac's
 * `/pair/store` and `/pair/vouched`, and the store's sign in and peers.
 */
export function pairing(
  mac: FixtureMac,
  method: string,
  target: string,
  body: string,
  here: string,
  signed: boolean,
): Answered | undefined {
  const path = new URL(target, "http://fixture").pathname;
  if (method === "GET" && path === "/pair/store")
    return json({
      store: `${here}${STORE_AT}`,
      client_id: "Iv1.fixture",
    });
  if (method === "POST" && path === "/pair/vouched") {
    if (!signed) return refused(403, "not_vouched");
    // A Mac that cannot find the phone in his store refuses, as the Gateway does, and keeps nothing.
    if (mac.name === "desk") return refused(403, "not_vouched");
    return json({
      device_id: `fixture-${mac.name}`,
    });
  }
  if (!path.startsWith(`${STORE_AT}/v1/`)) return undefined;
  const call = path.slice(STORE_AT.length + 3);
  if (method === "POST" && call === "/sign_in") {
    codes += 1;
    return json({
      device_code: `fixture-code-${codes}`,
      user_code: "WDJB-MJHT",
      verification_uri: "https://github.com/login/device",
      interval: POLL_SECONDS,
      expires_in: 900,
    });
  }
  if (method === "POST" && call === "/sign_in/poll") {
    const code = String(
      (
        JSON.parse(body || "{}") as {
          device_code?: unknown;
        }
      ).device_code,
    );
    const times = (polled.get(code) ?? 0) + 1;
    polled.set(code, times);
    if (times <= 2)
      return json(
        {
          waiting: "pending",
        },
        202,
      );
    return json({
      device: "7001.phone",
    });
  }
  if (method === "GET" && call === "/peers") {
    if (!signed) return refused(401, "unsigned");
    return json({
      devices: peersAt(here),
    });
  }
  return refused(404, "no_route");
}
