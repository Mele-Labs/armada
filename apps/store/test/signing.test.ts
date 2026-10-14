import { describe, expect, it } from "vitest";
import vector from "../test-vectors/signing.json";
import { bodyHash, fromBase64, fromHex, importPublicKey, signRequest, signedText, verifySignature } from "../src/signing";

// The same file is read by Fleet's signer, so the two sides agree on the text and the hash.
describe("the shared signing vector", () => {
  it("hashes the body to the vector's hex", async () => {
    expect(await bodyHash(vector.body)).toBe(vector.body_sha256_hex);
  });

  it("joins method, target, time, nonce and hash with nothing between", () => {
    expect(signedText(vector.method, vector.path_and_query, vector.time, vector.nonce, vector.body_sha256_hex)).toBe(
      vector.signed_text,
    );
  });

  it("verifies the vector's signature under its public key, and not a changed text", async () => {
    const key = await importPublicKey(fromBase64(vector.public_key_spki_base64)!);
    const signature = fromHex(vector.signature_hex)!;
    expect(signature).toHaveLength(64);
    expect(await verifySignature(key, vector.signed_text, signature)).toBe(true);
    expect(await verifySignature(key, vector.signed_text + "0", signature)).toBe(false);
  });

  it("signs the vector's request with its private key into a signature the public key verifies", async () => {
    const priv = await crypto.subtle.importKey(
      "pkcs8",
      fromBase64(vector.private_key_pkcs8_base64)!,
      { name: "ECDSA", namedCurve: "P-256" },
      false,
      ["sign"],
    );
    const headers = await signRequest(priv, "device", vector.method, vector.path_and_query, vector.body, vector.time, vector.nonce);

    expect(headers["X-Armada-Time"]).toBe(vector.time);
    expect(headers["X-Armada-Nonce"]).toBe(vector.nonce);
    const key = await importPublicKey(fromBase64(vector.public_key_spki_base64)!);
    expect(await verifySignature(key, vector.signed_text, fromHex(headers["X-Armada-Signature"])!)).toBe(true);
  });
});
