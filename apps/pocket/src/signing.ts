// The request signature the Gateway checks (crates/pocket/src/signing.rs):
// ECDSA P-256 over `method + path-and-query + time + hex(sha256(body))`, joined with nothing.
// WebCrypto's ECDSA output is already the raw 64-byte r||s the Gateway reads.

export const hex = (bytes: ArrayBuffer | Uint8Array): string =>
  Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");

export async function bodyHash(body: string): Promise<string> {
  return hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(body)));
}

export async function signedMessage(method: string, target: string, time: number, body: string): Promise<string> {
  return `${method}${target}${time}${await bodyHash(body)}`;
}

export type Signed = { "X-Pocket-Device": string; "X-Pocket-Time": string; "X-Pocket-Signature": string };

export async function signHeaders(
  key: CryptoKey,
  deviceId: string,
  method: string,
  target: string,
  body = "",
  time = Math.floor(Date.now() / 1000),
): Promise<Signed> {
  const message = await signedMessage(method, target, time, body);
  const signature = await crypto.subtle.sign({ name: "ECDSA", hash: "SHA-256" }, key, new TextEncoder().encode(message));
  return { "X-Pocket-Device": deviceId, "X-Pocket-Time": String(time), "X-Pocket-Signature": hex(signature) };
}
