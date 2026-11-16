import { Account } from "./account";
import { enrol } from "./enrol";
import { signIn, signInPoll } from "./sign_in";
import { DEVICE_HEADER } from "./signing";
import { teamObjectName, teamOf } from "./team";

export { Account };

// A device id is `<account id>.<random>`, so the Worker finds the account's Durable Object
// without a lookup. The Durable Object checks the signature against the device's own key.
const DEVICE = /^([0-9]{1,16})\.[0-9a-f]{32}$/;

// A page on another origin, Pocket on a phone, calls the store. Every call is signed or carries a one-time code and
// none uses a cookie, so any origin may read the answer, and nothing is sent with credentials.
const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Expose-Headers": "Retry-After",
};
const PREFLIGHT = {
  ...CORS,
  "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE",
  "Access-Control-Allow-Headers":
    "Content-Type, X-Armada-Device, X-Armada-Time, X-Armada-Nonce, X-Armada-Signature",
  "Access-Control-Max-Age": "600",
};

/** `answer` with the CORS headers, whether it is a refusal or not. A Durable Object's answer has immutable headers, so it is copied. */
function withCors(answer: Response): Response {
  const copy = new Response(answer.body, answer);
  for (const [name, value] of Object.entries(CORS)) copy.headers.set(name, value);
  return copy;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (!pathname.startsWith("/v1/")) return route(request, env, pathname);
    if (request.method === "OPTIONS")
      return new Response(null, {
        status: 204,
        headers: PREFLIGHT,
      });
    return withCors(await route(request, env, pathname));
  },
} satisfies ExportedHandler<Env>;

async function route(request: Request, env: Env, pathname: string): Promise<Response> {
  const team = teamOf(env);
  if (team === "bad")
    return Response.json(
      {
        error: "bad_team_config",
      },
      {
        status: 500,
      },
    );
  if (pathname === "/v1/enrol") return enrol(request, env);
  if (pathname === "/v1/sign_in") return signIn(request, env);
  if (pathname === "/v1/sign_in/poll") return signInPoll(request, env);
  if (!pathname.startsWith("/v1/"))
    return Response.json(
      {
        error: "no_route",
      },
      {
        status: 404,
      },
    );

  const account = DEVICE.exec(request.headers.get(DEVICE_HEADER) ?? "")?.[1];
  if (!account)
    return Response.json(
      {
        error: "unsigned",
      },
      {
        status: 401,
      },
    );
  // A team store keeps everyone in one object, whatever account id the device's name starts with.
  return env.ACCOUNT.get(env.ACCOUNT.idFromName(team ? teamObjectName(team) : account)).fetch(
    request,
  );
}
