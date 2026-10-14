import { Account } from "./account";
import { enrol } from "./enrol";
import { DEVICE_HEADER } from "./signing";

export { Account };

// A device id is `<account id>.<random>`, so the Worker finds the account's Durable Object
// without a lookup. The Durable Object checks the signature against the device's own key.
const DEVICE = /^([0-9]{1,16})\.[0-9a-f]{32}$/;

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const { pathname } = new URL(request.url);
    if (pathname === "/v1/enrol") return enrol(request, env);
    if (!pathname.startsWith("/v1/")) return Response.json({ error: "no_route" }, { status: 404 });

    const account = DEVICE.exec(request.headers.get(DEVICE_HEADER) ?? "")?.[1];
    if (!account) return Response.json({ error: "unsigned" }, { status: 401 });
    return env.ACCOUNT.get(env.ACCOUNT.idFromName(account)).fetch(request);
  },
} satisfies ExportedHandler<Env>;
