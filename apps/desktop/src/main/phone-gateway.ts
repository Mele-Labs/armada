// Bridge's one call to the Phone Gateway, on loopback. It is made here and not in the renderer
// because the Gateway's admin routes refuse any request carrying `Origin`, and a renderer's fetch
// carries one. Node's fetch adds none. The renderer names an operation; it never names a path.

import { fetch } from "undici";
import type { PhoneAnswer, PhoneRequest } from "@armada/settings/api";

export const GATEWAY = "http://127.0.0.1:8443";

type Route = { method: "GET" | "POST" | "DELETE"; path: string; body?: unknown };

function routeOf(request: PhoneRequest): Route {
  switch (request.op) {
    case "start":
      return { method: "POST", path: "/admin/pair/start" };
    case "pending":
      return { method: "GET", path: "/admin/pair/pending" };
    case "confirm":
      return { method: "POST", path: "/admin/pair/confirm", body: { code: request.code } };
    case "devices":
      return { method: "GET", path: "/admin/devices" };
    case "unpair":
      return { method: "DELETE", path: `/admin/devices/${encodeURIComponent(request.id)}` };
    case "status":
      return { method: "GET", path: "/admin/status" };
  }
}

/** Refused connection is the Gateway not running; anything else the Gateway said is its own sentence. */
export async function askGateway(request: PhoneRequest, base: string = GATEWAY): Promise<PhoneAnswer> {
  const route = routeOf(request);
  let answer;
  try {
    answer = await fetch(`${base}${route.path}`, {
      method: route.method,
      ...(route.body === undefined ? {} : { headers: { "content-type": "application/json" }, body: JSON.stringify(route.body) }),
      signal: AbortSignal.timeout(5000),
    });
  } catch (why) {
    const code = (why as { cause?: { code?: string } }).cause?.code;
    if (code === "ECONNREFUSED") return { ok: false, why: "unreachable" };
    return { ok: false, why: "refused", said: "The Gateway did not answer." };
  }
  const text = (await answer.text()).trim();
  if (!answer.ok) return { ok: false, why: "refused", said: text === "" ? `The Gateway answered ${answer.status}.` : text };
  if (answer.status === 204 || text === "") return { ok: true, body: null };
  try {
    return { ok: true, body: JSON.parse(text) };
  } catch {
    return { ok: false, why: "refused", said: "The Gateway sent something Bridge could not read." };
  }
}
