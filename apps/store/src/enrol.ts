import { exactly, MACHINE, MAX_BODY_BYTES, parseJson } from "./rows";
import { fromBase64, hex, importPublicKey } from "./signing";
import { MAX_DEVICES, MAX_TEAM_DEVICES } from "./rows";
import { teamObjectName, teamOf, type Team } from "./team";

// The one file that names the forge: the account's id comes from its user endpoint.
// `FORGE_USER_URL` overrides it, which is how the tests stand in for the forge.
const FORGE_USER_URL = "https://api.github.com/user";

const refuse = (status: number, error: string): Response =>
  Response.json(
    {
      error,
    },
    {
      status,
    },
  );

/** A GET to the forge as the token's owner: the answer, or the response to refuse with when it cannot be reached. */
async function askForge(url: string, token: string): Promise<Response> {
  try {
    return await fetch(url, {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/json",
        "User-Agent": "armada-store",
      },
    });
  } catch {
    return refuse(502, "forge_unreachable");
  }
}

/** The account's numeric id and login for a device-flow token, or the response to refuse with. */
async function accountOf(
  token: string,
  env: Env,
): Promise<
  | {
      id: number;
      login: unknown;
    }
  | Response
> {
  const answer = await askForge(env.FORGE_USER_URL ?? FORGE_USER_URL, token);
  if (answer.status === 401 || answer.status === 403) return refuse(401, "forge_refused");
  if (!answer.ok) return refuse(502, "forge_unreachable");
  const user = (await answer.json().catch(() => null)) as {
    id?: unknown;
    login?: unknown;
  } | null;
  const id = user?.id;
  if (typeof id !== "number" || !Number.isSafeInteger(id) || id <= 0)
    return refuse(502, "forge_unreadable");
  return {
    id,
    login: user?.login,
  };
}

/** The refusal for a token whose owner is not in the team, or `null` when they are. Asked of the forge's API, at the origin of its user endpoint. */
async function refuseNonMember(
  token: string,
  login: unknown,
  team: Team,
  env: Env,
): Promise<Response | null> {
  const api = new URL(env.FORGE_USER_URL ?? FORGE_USER_URL).origin;
  if (team.slug !== undefined && (typeof login !== "string" || login === ""))
    return refuse(502, "forge_unreadable");
  const url =
    team.slug === undefined
      ? `${api}/user/memberships/orgs/${team.org}`
      : `${api}/orgs/${team.org}/teams/${team.slug}/memberships/${encodeURIComponent(String(login))}`;
  const answer = await askForge(url, token);
  if (answer.status === 404) return refuse(403, "not_a_member");
  if (answer.status === 403) return refuse(403, "org_restricted");
  if (answer.status === 401) return refuse(401, "forge_refused");
  if (!answer.ok) return refuse(502, "forge_unreachable");
  const membership = (await answer.json().catch(() => null)) as {
    state?: unknown;
  } | null;
  return membership?.state === "active" ? null : refuse(403, "not_a_member");
}

/**
 * Registers a device's public key under the account its token names, or under the team's one object on a team store
 * once the token's owner is checked to be a member. The token is read for the id and the membership and goes no
 * further: not to the Durable Object, not to storage, not to a log.
 */
export async function enrol(request: Request, env: Env): Promise<Response> {
  if (request.method !== "POST") return refuse(405, "bad_method");
  const team = teamOf(env);
  if (team === "bad") return refuse(500, "bad_team_config");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return refuse(413, "too_large");
  const body = exactly(parseJson(text), [
    "token",
    "public_key",
    "machine",
  ]);
  const { token, public_key: publicKey, machine } = body ?? {};
  if (
    typeof token !== "string" ||
    token === "" ||
    token.length > 512 ||
    typeof publicKey !== "string" ||
    typeof machine !== "string" ||
    !MACHINE.test(machine)
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
  if (
    env.ENROL_LIMIT &&
    !(
      await env.ENROL_LIMIT.limit({
        key: address,
      })
    ).success
  )
    return refuse(429, "enrol_rate_limited");

  const account = await accountOf(token, env);
  if (account instanceof Response) return account;

  if (team) {
    const refused = await refuseNonMember(token, account.login, team, env);
    if (refused) return refused;
  }

  const device = `${account.id}.${hex(crypto.getRandomValues(new Uint8Array(16)))}`;
  const placed = await env.ACCOUNT.get(
    env.ACCOUNT.idFromName(team ? teamObjectName(team) : String(account.id)),
  ).register(device, publicKey, machine, team ? MAX_TEAM_DEVICES : MAX_DEVICES);
  if (placed === "full") return refuse(413, "devices_full");
  return Response.json({
    device,
  });
}
