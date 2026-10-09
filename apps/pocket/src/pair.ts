// Pairing: make the key, hand its public half to the Gateway with the code from Bridge's
// QR, then wait for the owner to press Confirm on the Mac.

import { call, Refused } from "./client";
import { generateKey, keepDeviceId, keepKey } from "./device";
import { hex } from "./signing";

const NOT_ANSWERING = "Armada is not answering on your Mac. Open it there, then try again.";

/** The name a phone offers for itself, from the user agent. */
export function defaultName(agent: string): string {
  if (/iPhone/.test(agent)) return "iPhone";
  if (/iPad/.test(agent)) return "iPad";
  if (/Android/.test(agent)) return "Android phone";
  return "Phone";
}

/** Set by the first visit to `/pair?code=…`; the address bar loses it when the app navigates. */
let code = "";
export function codeFromUrl(search: string): string {
  const found = new URLSearchParams(search).get("code");
  if (found !== null && found !== "") code = found;
  return code;
}

/**
 * TODO(open question): the Gateway's `POST /pair` answers 202 with no body and picks the
 * device id when Bridge confirms (`pairing.rs`, `confirm`), so the phone has no way to learn
 * it. This reads `device_id` from the claim's answer; the Gateway has to send it there.
 */
export const deviceIdOf = (answer: unknown): string | undefined =>
  typeof answer === "object" && answer !== null && typeof (answer as { device_id?: unknown }).device_id === "string"
    ? (answer as { device_id: string }).device_id
    : undefined;

/** Posts the claim. Resolves once the key and id are kept; throws a `Refused` with a sentence. */
export async function claim(name: string): Promise<void> {
  const pair = await generateKey();
  const spki = hex(await crypto.subtle.exportKey("spki", pair.publicKey));
  let response: Response;
  try {
    response = await fetch("/pair", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code, name: name.trim(), spki_hex: spki }),
    });
  } catch {
    throw new Refused(0, NOT_ANSWERING);
  }
  if (!response.ok) throw new Refused(response.status, (await response.text()) || NOT_ANSWERING);
  const id = deviceIdOf(await response.json().catch(() => undefined));
  if (id === undefined) throw new Refused(502, "Armada answered in a way this phone cannot read. Update Armada on your Mac.");
  await keepKey(pair.privateKey);
  await keepDeviceId(id);
}

/** Polls a signed read until it stops answering 401: the owner has confirmed. */
export async function waitForConfirm(wait = 3000, limit = 5 * 60_000, signal?: AbortSignal): Promise<boolean> {
  const until = Date.now() + limit;
  while (Date.now() < until && signal?.aborted !== true) {
    try {
      await call("/api/needs", { leavePairing: false });
      return true;
    } catch (why) {
      if (!(why instanceof Refused && why.status === 401)) return true; // paired; the Gateway had another trouble
    }
    await new Promise((resolve) => setTimeout(resolve, wait));
  }
  return false;
}
