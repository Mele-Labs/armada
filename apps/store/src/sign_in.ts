import { FORGE_LOGIN_URL, isP256Key, refuse, refuseIfCounted, registerToken } from "./enrol";
import { exactly, MACHINE, MAX_BODY_BYTES, parseJson } from "./rows";
import { teamOf } from "./team";

// The phone signs in through the Worker, because the forge's sign-in endpoints answer no CORS to a page and the token
// must not reach the phone. `FORGE_LOGIN_URL` overrides the forge's address, as `FORGE_USER_URL` does for the user endpoint.

const CLIENT_ID = /^[A-Za-z0-9._-]{1,64}$/;
const DEVICE_CODE = /^[\x20-\x7e]{1,128}$/;
/** What the forge names a refusal by: letters and `_`, so nothing else it says is passed on. */
const REASON = /^[a-z_]{1,64}$/;
/** Seconds a phone waits between polls when the forge's `slow_down` names none. */
const SLOWER_INTERVAL = 10;

const reasonOf = (error: unknown): Record<string, string> =>
  typeof error === "string" && REASON.test(error)
    ? {
        reason: error,
      }
    : {};

/** The forge's JSON answer to a form post, or the response to refuse with when it cannot be reached or read. */
async function askLogin(
  env: Env,
  path: string,
  form: Record<string, string>,
): Promise<
  | {
      status: number;
      body: Record<string, unknown>;
    }
  | Response
> {
  let answer: Response;
  try {
    answer = await fetch(`${env.FORGE_LOGIN_URL ?? FORGE_LOGIN_URL}${path}`, {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/x-www-form-urlencoded",
        "User-Agent": "armada-store",
      },
      body: new URLSearchParams(form),
    });
  } catch {
    return refuse(502, "forge_unreachable");
  }
  const body = (await answer.json().catch(() => null)) as unknown;
  return {
    status: answer.status,
    body:
      typeof body === "object" && body !== null && !Array.isArray(body)
        ? (body as Record<string, unknown>)
        : {},
  };
}

/** The refusal for a forge answer that holds no `error` the route knows: its own 4xx is `forge_refused`, its 5xx is not an answer. */
const refusedBy = (status: number, error: unknown): Response | null =>
  status >= 500
    ? refuse(502, "forge_unreachable")
    : error !== undefined || status >= 400
      ? Response.json(
          {
            error: "forge_refused",
            ...reasonOf(error),
          },
          {
            status: 400,
          },
        )
      : null;

/** The body of a POST to a sign-in route as exactly `keys`, or the response to refuse with. */
async function bodyOf(
  request: Request,
  keys: readonly string[],
): Promise<Record<string, unknown> | Response> {
  if (request.method !== "POST") return refuse(405, "bad_method");
  const text = await request.text();
  if (text.length > MAX_BODY_BYTES) return refuse(413, "too_large");
  const body = exactly(parseJson(text), keys);
  const clientId = body?.client_id;
  if (!body || typeof clientId !== "string" || !CLIENT_ID.test(clientId))
    return refuse(400, "invalid_body");
  return body;
}

/** Starts the device flow for a phone: the forge's code and where to enter it. Counted like an enrolment, since it calls the forge. */
export async function signIn(request: Request, env: Env): Promise<Response> {
  const body = await bodyOf(request, [
    "client_id",
  ]);
  if (body instanceof Response) return body;
  const counted = await refuseIfCounted(request, env);
  if (counted) return counted;

  const asked = await askLogin(env, "/login/device/code", {
    client_id: String(body.client_id),
  });
  if (asked instanceof Response) return asked;
  const { device_code, user_code, verification_uri, interval, expires_in, error } = asked.body;
  const refused = refusedBy(asked.status, error);
  if (refused) return refused;
  if (
    typeof device_code !== "string" ||
    typeof user_code !== "string" ||
    typeof verification_uri !== "string" ||
    typeof interval !== "number" ||
    typeof expires_in !== "number"
  )
    return refuse(502, "forge_unreadable");
  return Response.json({
    device_code,
    user_code,
    verification_uri,
    interval,
    expires_in,
  });
}

/**
 * Asks the forge whether the person has entered the code. A token is registered as a phone's device, exactly as an
 * enrolment registers a Mac, and dropped: it is not stored, logged or sent to the Durable Object. Polls are not counted,
 * since the forge's `slow_down` bounds them.
 */
export async function signInPoll(request: Request, env: Env): Promise<Response> {
  const team = teamOf(env);
  if (team === "bad") return refuse(500, "bad_team_config");
  const body = await bodyOf(request, [
    "client_id",
    "device_code",
    "public_key",
    "machine",
  ]);
  if (body instanceof Response) return body;
  const { client_id, device_code, public_key, machine } = body;
  if (
    typeof device_code !== "string" ||
    !DEVICE_CODE.test(device_code) ||
    typeof public_key !== "string" ||
    typeof machine !== "string" ||
    !MACHINE.test(machine)
  )
    return refuse(400, "invalid_body");
  if (!(await isP256Key(public_key))) return refuse(400, "invalid_public_key");

  const asked = await askLogin(env, "/login/oauth/access_token", {
    client_id: String(client_id),
    device_code,
    grant_type: "urn:ietf:params:oauth:grant-type:device_code",
  });
  if (asked instanceof Response) return asked;
  const { access_token: token, error, interval } = asked.body;
  if (asked.status >= 500) return refuse(502, "forge_unreachable");
  if (error === "authorization_pending")
    return Response.json(
      {
        waiting: "pending",
      },
      {
        status: 202,
      },
    );
  if (error === "slow_down")
    return Response.json(
      {
        waiting: "slow_down",
        interval: typeof interval === "number" ? interval : SLOWER_INTERVAL,
      },
      {
        status: 202,
      },
    );
  if (error === "expired_token") return refuse(410, "expired");
  if (error === "access_denied") return refuse(403, "denied");
  const refused = refusedBy(asked.status, error);
  if (refused) return refused;
  if (typeof token !== "string" || token === "") return refuse(502, "forge_unreadable");
  return registerToken(token, public_key, machine, "phone", team, env);
}
