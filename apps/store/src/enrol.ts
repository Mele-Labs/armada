import { exactly, MACHINE, MAX_BODY_BYTES, parseJson } from "./rows";
import { fromBase64, hex, importPublicKey } from "./signing";

// The one file that names the forge: the account's id comes from its user endpoint.
// `FORGE_USER_URL` overrides it, which is how the tests stand in for the forge.
const FORGE_USER_URL = "https://api.github.com/user";

const refuse = (status: number, error: string): Response => Response.json({ error }, { status });

/** The account's numeric id for a device-flow token, or the response to refuse with. */
async function accountOf(token: string, env: Env): Promise<number | Response> {
  let answer: Response;
  try {
    answer = await fetch(env.FORGE_USER_URL ?? FORGE_USER_URL, {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/json",
        "User-Agent": "armada-store",
      },
    });
  } catch {
    return refuse(502, "forge_unreachable");
  }
  if (answer.status === 401 || answer.status === 403) return refuse(401, "forge_refused");
  if (!answer.ok) return refuse(502, "forge_unreachable");
  const user = (await answer.json().catch(() => null)) as { id?: unknown } | null;
  const id = user?.id;
  if (typeof id !== "number" || !Number.isSafeInteger(id) || id <= 0) return refuse(502, "forge_unreadable");
  return id;
}

/**
 * Registers a device's public key under the account its token names. The token is read
 * for the id and goes no further: not to the Durable Object, not to storage, not to a log.
 */
export async function enrol(request: Request, env: Env): Promise<Response> {
  if (request.method !== "POST") return refuse(405, "bad_method");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return refuse(413, "too_large");
  const body = exactly(parseJson(text), ["token", "public_key", "machine"]);
  const { token, public_key: publicKey, machine } = body ?? {};
  if (
    typeof token !== "string" || token === "" || token.length > 512 ||
    typeof publicKey !== "string" ||
    typeof machine !== "string" || !MACHINE.test(machine)
  ) {
    return refuse(400, "invalid_body");
  }
  const spki = fromBase64(publicKey);
  try {
    await importPublicKey(spki ?? new Uint8Array());
  } catch {
    return refuse(400, "invalid_public_key");
  }

  // Past this point every request costs a call to the forge, so this is where an address is counted.
  const address = request.headers.get("CF-Connecting-IP") ?? "unknown";
  if (!(await env.ENROL_LIMIT.limit({ key: address })).success) return refuse(429, "enrol_rate_limited");

  const account = await accountOf(token, env);
  if (account instanceof Response) return account;

  const device = `${account}.${hex(crypto.getRandomValues(new Uint8Array(16)))}`;
  const placed = await env.ACCOUNT.get(env.ACCOUNT.idFromName(String(account))).register(device, publicKey, machine);
  if (placed === "full") return refuse(413, "devices_full");
  return Response.json({ device });
}
