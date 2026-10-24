// A fixture Phone Gateway: answers the phone's routes for one Mac from `wire.ts`. Shared by the fixture
// server (`vite.config.ts`, mode `fixture`) and the acceptance test, so both read the same Macs.
// It checks that a call is signed, not that the signature is good: there is no pairing behind it.

import type { FixtureMac } from "./wire";

export type Answered = { status: number; type?: string; body: string };

/** What the fixture was sent, oldest first, for a test to read. */
export type Posted = { path: string; body: unknown };

const json = (value: unknown, status = 200): Answered => ({ status, type: "application/json", body: JSON.stringify(value) });
const said = (status: number, sentence: string): Answered => ({ status, type: "text/plain", body: sentence });

export function answer(mac: FixtureMac, posted: Posted[], method: string, target: string, body: string, signed: boolean): Answered {
  const url = new URL(target, "http://fixture");
  const path = url.pathname;
  if (method === "POST" && path === "/pair") return json({ device_id: `fixture-${mac.name}` }, 202);
  if (!path.startsWith("/api/")) return said(404, "Not found.");
  if (!signed) return said(400, "This request is not signed.");
  const id = url.searchParams.get("session_id") ?? "";
  if (method === "GET") {
    switch (path) {
      case "/api/machine":
        return json({ name: mac.name });
      case "/api/needs":
        return json({ sessions: mac.needs });
      case "/api/sessions":
        return json({ sessions: mac.live });
      case "/api/repositories":
        return json(mac.repositories);
      case "/api/sessions/one": {
        const thread = mac.threads[id];
        return thread === undefined ? said(404, "No Job has that id on this Mac.") : json(thread);
      }
      case "/api/sessions/shown": {
        const text = mac.shown[id]?.[url.searchParams.get("path") ?? ""];
        return text === undefined ? said(404, "That Job did not show that file.") : json({ path: url.searchParams.get("path"), text });
      }
      case "/api/push/key":
        return said(404, "Push is not served by the fixture.");
    }
    return said(404, "Not found.");
  }
  if (method === "POST") {
    posted.push({ path, body: body === "" ? undefined : JSON.parse(body) });
    if (path === "/api/sessions") return json({ session: mac.live[0] }, 201);
    return { status: 204, body: "" };
  }
  return said(405, "Not allowed.");
}
