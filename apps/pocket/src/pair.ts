// Pairing one Mac: make a key for it, hand the public half to that Mac with the code from its
// pairing link, then wait for the owner to press Confirm on that Mac. The link's origin is the Mac;
// a bare code, or a code in this page's own address, is for the Mac that served the app.

import { Refused, call, getJson, notAnswering } from "./client";
import type { Machine } from "./machines";
import { hex } from "./signing";

/** The name a phone offers for itself, from the user agent. */
export function defaultName(agent: string): string {
  if (/iPhone/.test(agent)) return "iPhone";
  if (/iPad/.test(agent)) return "iPad";
  if (/Android/.test(agent)) return "Android phone";
  return "Phone";
}

const CODE = /^[0-9a-f]{32}$/i;

/** The Mac and the code out of a whole pairing link, or a bare code for `here`; undefined when there is none. */
export function linkOf(text: string, here: string): { origin: string; code: string } | undefined {
  const t = text.trim();
  if (CODE.test(t)) return { origin: here, code: t };
  try {
    const url = new URL(t);
    const code = url.searchParams.get("code") ?? "";
    return CODE.test(code) ? { origin: url.origin, code } : undefined;
  } catch {
    return undefined;
  }
}

/** The 32-hex code out of a whole pairing link or a bare code; "" when there is none. */
export const codeFromInput = (text: string): string => linkOf(text, "")?.code ?? "";

/** `POST /pair` answers 202 with `{device_id}`, chosen at claim. */
export const deviceIdOf = (answer: unknown): string | undefined =>
  typeof answer === "object" && answer !== null && typeof (answer as { device_id?: unknown }).device_id === "string"
    ? (answer as { device_id: string }).device_id
    : undefined;

const generateKey = (): Promise<CryptoKeyPair> => crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, false, ["sign", "verify"]);

/** Posts the claim to the Mac at `origin`. Resolves to that Mac, not yet confirmed or named; throws a `Refused` with a sentence. */
export async function claim(origin: string, name: string, code: string): Promise<Machine> {
  const pair = await generateKey();
  const spki = hex(await crypto.subtle.exportKey("spki", pair.publicKey));
  let response: Response;
  try {
    response = await fetch(`${origin}/pair`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code, name: name.trim(), spki_hex: spki }),
    });
  } catch {
    throw new Refused(0, notAnswering());
  }
  if (!response.ok) throw new Refused(response.status, (await response.text()) || notAnswering());
  const id = deviceIdOf(await response.json().catch(() => undefined));
  if (id === undefined) throw new Refused(502, "Armada answered in a way this phone cannot read. Update Armada on your Mac.");
  return { origin, name: "", deviceId: id, key: pair.privateKey };
}

/** Polls a signed read until it stops answering 401: the owner has confirmed. */
export async function waitForConfirm(machine: Machine, wait = 3000, limit = 5 * 60_000, signal?: AbortSignal): Promise<boolean> {
  const until = Date.now() + limit;
  while (Date.now() < until && signal?.aborted !== true) {
    try {
      await call(machine, "/api/needs");
      return true;
    } catch (why) {
      if (!(why instanceof Refused && why.status === 401)) return true; // paired; the Mac had another trouble
    }
    await new Promise((resolve) => setTimeout(resolve, wait));
  }
  return false;
}

/** The confirmed Mac with the name it gives itself; its origin's host where it does not say. */
export async function named(machine: Machine): Promise<Machine> {
  try {
    const { name } = await getJson<{ name: string }>(machine, "/api/machine");
    return { ...machine, name };
  } catch {
    return { ...machine, name: new URL(machine.origin).hostname.split(".")[0] ?? "" };
  }
}
