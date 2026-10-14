// The request signature the store checks. ECDSA P-256 over SHA-256, the signature the raw
// 64 bytes r||s as hex, which is what WebCrypto emits and what the Rust side reads.
// `docs/contracts/central-store.md`, Signing. `test-vectors/signing.json` pins the text.

export const SKEW_SECONDS = 60;
export const NONCE_KEEP_SECONDS = 120;

export const DEVICE_HEADER = "X-Armada-Device";
export const TIME_HEADER = "X-Armada-Time";
export const NONCE_HEADER = "X-Armada-Nonce";
export const SIGNATURE_HEADER = "X-Armada-Signature";

const ECDSA = { name: "ECDSA", hash: "SHA-256" } as const;
const CURVE = { name: "ECDSA", namedCurve: "P-256" } as const;

export const hex = (bytes: ArrayBuffer | Uint8Array): string =>
  Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");

export function fromHex(text: string): Uint8Array | null {
  if (text.length % 2 !== 0 || !/^[0-9a-f]*$/.test(text)) return null;
  const out = new Uint8Array(text.length / 2);
  for (let at = 0; at < out.length; at++) out[at] = parseInt(text.slice(at * 2, at * 2 + 2), 16);
  return out;
}

export function fromBase64(text: string): Uint8Array | null {
  try {
    return Uint8Array.from(atob(text), (char) => char.charCodeAt(0));
  } catch {
    return null;
  }
}

export const toBase64 = (bytes: ArrayBuffer | Uint8Array): string =>
  btoa(String.fromCharCode(...new Uint8Array(bytes)));

export async function bodyHash(body: Uint8Array | string): Promise<string> {
  const bytes = typeof body === "string" ? new TextEncoder().encode(body) : body;
  return hex(await crypto.subtle.digest("SHA-256", bytes));
}

/** `method + path_and_query + time + nonce + hex(sha256(body))`, joined with nothing. */
export function signedText(method: string, target: string, time: string, nonce: string, hash: string): string {
  return `${method}${target}${time}${nonce}${hash}`;
}

export async function importPublicKey(spki: Uint8Array): Promise<CryptoKey> {
  return crypto.subtle.importKey("spki", spki, CURVE, false, ["verify"]);
}

export async function verifySignature(key: CryptoKey, text: string, signature: Uint8Array): Promise<boolean> {
  if (signature.length !== 64) return false;
  return crypto.subtle.verify(ECDSA, key, signature, new TextEncoder().encode(text));
}

export type SignedHeaders = Record<
  "X-Armada-Device" | "X-Armada-Time" | "X-Armada-Nonce" | "X-Armada-Signature",
  string
>;

export const newNonce = (): string => hex(crypto.getRandomValues(new Uint8Array(16)));

export async function signRequest(
  key: CryptoKey,
  device: string,
  method: string,
  target: string,
  body = "",
  time = String(Math.floor(Date.now() / 1000)),
  nonce = newNonce(),
): Promise<SignedHeaders> {
  const text = signedText(method, target, time, nonce, await bodyHash(body));
  const signature = await crypto.subtle.sign(ECDSA, key, new TextEncoder().encode(text));
  return {
    "X-Armada-Device": device,
    "X-Armada-Time": time,
    "X-Armada-Nonce": nonce,
    "X-Armada-Signature": hex(signature),
  };
}
