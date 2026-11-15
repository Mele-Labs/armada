// Pairing through the owner's claims store. The phone signs in to the store with GitHub, the store lists his Macs, and a Mac
// that finds this phone in the same store pairs with it at once: it needs no code and no Confirm. The wire is
// docs/contracts/central-store.md, and `POST /pair/vouched` on the Mac.

import { Refused } from "./client";
import type { Machine, StoreSignIn } from "./machines";
import { generateKey } from "./pairing";
import { hex, storeHeaders, toBase64 } from "./signing";

/** A GitHub device code, as the store hands it on from GitHub. */
export type DeviceCode = {
  device_code: string;
  user_code: string;
  verification_uri: string;
  interval: number;
  expires_in: number;
};

/** A device of his in the store, as `GET /v1/peers` lists it. */
export type Peer = {
  device: string;
  machine: string;
  kind: string;
  address: string;
  public_key: string;
};

const notAnswering = "The store is not answering. Try again.";

const json = {
  "Content-Type": "application/json",
};

async function reasonOf(response: Response): Promise<{
  error: string;
  reason: string;
}> {
  const said = (await response.json().catch(() => undefined)) as
    | {
        error?: unknown;
        reason?: unknown;
      }
    | undefined;
  return {
    error: typeof said?.error === "string" ? said.error : "",
    reason: typeof said?.reason === "string" ? said.reason : "",
  };
}

const without = (address: string): string => address.replace(/\/+$/, "");

/** The store and the sign-in app's client id from the Mac Pocket was opened from; undefined where that Mac has no sign-in set up. */
export async function storeOf(origin: string): Promise<
  | {
      store: string;
      clientId: string;
    }
  | undefined
> {
  try {
    const response = await fetch(`${origin}/pair/store`);
    if (!response.ok) return undefined;
    const said = (await response.json()) as {
      store?: unknown;
      client_id?: unknown;
    };
    return typeof said.store === "string" && typeof said.client_id === "string"
      ? {
          store: without(said.store),
          clientId: said.client_id,
        }
      : undefined;
  } catch {
    return undefined;
  }
}

/** The sentence for a sign-in the store or GitHub turned down. */
function signInSaid(status: number, error: string, reason: string): string {
  if (status === 410) return "The code expired. Sign in again.";
  if (status === 403) return "GitHub denied the sign in. Sign in again.";
  if (status === 429)
    return "The store counted too many sign ins from this network. Try again in a minute.";
  if (status === 502) return "The store could not reach GitHub. Try again.";
  if (error === "forge_refused" && reason === "device_flow_disabled")
    return "The GitHub app needs device sign in turned on. Turn it on in the app's settings on GitHub, then sign in again.";
  if (error === "forge_refused")
    return `GitHub refused the sign in: ${reason || "no reason given"}.`;
  return `The store refused the sign in: ${error || status}.`;
}

async function post(url: string, body: unknown): Promise<Response> {
  try {
    return await fetch(url, {
      method: "POST",
      headers: json,
      body: JSON.stringify(body),
    });
  } catch {
    throw new Refused(0, notAnswering);
  }
}

/** Asks the store for a GitHub device code. Throws a `Refused` with the sentence. */
export async function startSignIn(store: string, clientId: string): Promise<DeviceCode> {
  const response = await post(`${store}/v1/sign_in`, {
    client_id: clientId,
  });
  if (!response.ok) {
    const { error, reason } = await reasonOf(response);
    throw new Refused(response.status, signInSaid(response.status, error, reason));
  }
  return (await response.json()) as DeviceCode;
}

export type Polled =
  | {
      at: "waiting";
      /** The seconds GitHub asked the phone to wait between polls, where it changed. */
      interval?: number;
    }
  | {
      at: "in";
      deviceId: string;
    };

/** One poll of the sign-in. Throws a `Refused` with the sentence once the sign in cannot go on. */
export async function pollSignIn(
  store: string,
  clientId: string,
  code: DeviceCode,
  key: CryptoKeyPair,
  machine: string,
): Promise<Polled> {
  const response = await post(`${store}/v1/sign_in/poll`, {
    client_id: clientId,
    device_code: code.device_code,
    public_key: toBase64(await crypto.subtle.exportKey("spki", key.publicKey)),
    machine,
  });
  if (response.status === 202) {
    const said = (await response.json().catch(() => ({}))) as {
      interval?: unknown;
    };
    return typeof said.interval === "number"
      ? {
          at: "waiting",
          interval: said.interval,
        }
      : {
          at: "waiting",
        };
  }
  if (response.ok) {
    const said = (await response.json()) as {
      device?: unknown;
    };
    if (typeof said.device === "string")
      return {
        at: "in",
        deviceId: said.device,
      };
    throw new Refused(502, "The store answered in a way this phone cannot read.");
  }
  const { error, reason } = await reasonOf(response);
  throw new Refused(response.status, signInSaid(response.status, error, reason));
}

async function signed(
  sign: StoreSignIn,
  method: string,
  url: string,
  body: string,
  more: Record<string, string> = {},
): Promise<Response> {
  const at = new URL(url);
  try {
    return await fetch(url, {
      method,
      headers: {
        ...(await storeHeaders(sign.key, sign.deviceId, method, at.pathname + at.search, body)),
        ...(body === "" ? {} : json),
        ...more,
      },
      ...(body === ""
        ? {}
        : {
            body,
          }),
    });
  } catch {
    throw new Refused(0, notAnswering);
  }
}

/** His Macs the store knows an address for, oldest first. A 401 is the store forgetting this phone. */
export async function macsOf(sign: StoreSignIn): Promise<Peer[]> {
  const response = await signed(sign, "GET", `${sign.store}/v1/peers`, "");
  if (response.status === 401)
    throw new Refused(401, "The store does not know this phone. Sign in again.");
  if (!response.ok)
    throw new Refused(
      response.status,
      `The store refused the list: ${(await reasonOf(response)).error || response.status}.`,
    );
  const { devices } = (await response.json()) as {
    devices: Peer[];
  };
  return devices.filter((one) => one.kind === "mac" && one.address !== "");
}

/** Pairs with a Mac the store lists: a key for that Mac, handed over signed with the store key. Throws a `Refused` with the sentence. */
export async function vouch(sign: StoreSignIn, peer: Peer, name: string): Promise<Machine> {
  const pair = await generateKey();
  const address = without(peer.address);
  const response = await signed(
    sign,
    "POST",
    `${address}/pair/vouched`,
    JSON.stringify({
      name: name.trim(),
      spki_hex: hex(await crypto.subtle.exportKey("spki", pair.publicKey)),
    }),
    {
      "X-Armada-Store": sign.store,
    },
  ).catch((why: unknown) => {
    if (why instanceof Refused)
      throw new Refused(
        0,
        `Armada is not answering on ${peer.machine}. Open it there, then try again.`,
      );
    throw why;
  });
  if (response.status === 403)
    throw new Refused(
      403,
      "This Mac did not find this phone in your store. Sign in to the same GitHub account on the Mac, then try again.",
    );
  if (!response.ok)
    throw new Refused(
      response.status,
      (await response.text()) || `Armada did not pair on ${peer.machine}.`,
    );
  const said = (await response.json().catch(() => undefined)) as
    | {
        device_id?: unknown;
      }
    | undefined;
  if (typeof said?.device_id !== "string")
    throw new Refused(
      502,
      "Armada answered in a way this phone cannot read. Update Armada on your Mac.",
    );
  return {
    origin: address,
    name: peer.machine,
    deviceId: said.device_id,
    key: pair.privateKey,
  };
}
