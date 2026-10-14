import { env } from "cloudflare:workers";
import { vi } from "vitest";
import { signRequest, toBase64, type SignedHeaders } from "../src/signing";
import worker from "../src/index";
import type { Holder } from "../src/rows";

export const STORE = "https://store.test";

/** A request to the Worker, as the platform would hand it over. */
export const toStore = (url: string, init?: RequestInit): Promise<Response> =>
  worker.fetch(new Request(url, init), env);

/** 64 characters standing for a hash a machine made. */
export const hash = (letter: string): string => letter.repeat(64);

export const holder = (id: string, machine = "studio"): Holder => ({ kind: "job", id, machine, since: 1760449990 });

export type Device = { id: string; key: CryptoKey; account: number; token: string };

/** Answers the forge's user endpoint for `token` with `account`, until restored. */
export function forgeAnswers(account: number, status = 200) {
  return vi.spyOn(globalThis, "fetch").mockImplementation(async () =>
    status === 200
      ? Response.json({ id: account, login: "someone", name: "A Name" })
      : Response.json({ message: "Bad credentials" }, { status }),
  );
}

export async function newKeyPair() {
  const pair = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
    "sign",
    "verify",
  ])) as CryptoKeyPair;
  const spki = (await crypto.subtle.exportKey("spki", pair.publicKey)) as ArrayBuffer;
  return { key: pair.privateKey, spki: toBase64(spki) };
}

/** Enrols a fresh machine under `account`, through the Worker, against a stand-in forge. */
export async function enrolDevice(account: number, machine = "studio"): Promise<Device> {
  const { key, spki } = await newKeyPair();
  const token = `device-flow-token-${crypto.randomUUID()}`;
  const forge = forgeAnswers(account);
  try {
    const answer = await toStore(`${STORE}/v1/enrol`, {
      method: "POST",
      body: JSON.stringify({ token, public_key: spki, machine }),
    });
    const { device } = (await answer.json()) as { device: string };
    return { id: device, key, account, token };
  } finally {
    forge.mockRestore();
  }
}

/** A signed request from `device`. `sign` overrides what the signature is made over. */
export async function call(
  device: Device,
  method: string,
  target: string,
  body?: unknown,
  sign: { key?: CryptoKey; time?: string; nonce?: string; signedBody?: string; headers?: Partial<SignedHeaders> } = {},
): Promise<Response> {
  const text = body === undefined ? "" : JSON.stringify(body);
  const headers = await signRequest(sign.key ?? device.key, device.id, method, target, sign.signedBody ?? text, sign.time, sign.nonce);
  return toStore(`${STORE}${target}`, {
    method,
    headers: { ...headers, ...sign.headers },
    body: text === "" ? undefined : text,
  });
}

export async function claim(device: Device, repo: string, red: string, who: Holder) {
  return call(device, "POST", "/v1/claims", { repo, red, holder: who });
}
