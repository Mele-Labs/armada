import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";

import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

import { answer, type Posted } from "./src/fixture/gateway";
import { fixtureMacs, type FixtureMac } from "./src/fixture/wire";

const read = (request: IncomingMessage): Promise<string> =>
  new Promise((resolve, reject) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk: string) => (body += chunk));
    request.on("end", () => resolve(body));
    request.on("error", reject);
  });

/** Answers one Mac's routes from the fixture, holding `/api/live` open as the Gateway does. */
function serve(mac: () => FixtureMac, posted: Posted[]) {
  return async (request: IncomingMessage, response: ServerResponse, next?: () => void) => {
    const target = request.url ?? "/";
    const origin = request.headers.origin;
    if (origin !== undefined) {
      response.setHeader("Access-Control-Allow-Origin", origin);
      response.setHeader("Access-Control-Allow-Headers", "content-type, x-pocket-device, x-pocket-time, x-pocket-signature");
      response.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    }
    const ours = target.startsWith("/api/") || target === "/pair";
    if (!ours) return next === undefined ? response.writeHead(404).end() : next();
    if (request.method === "OPTIONS") return response.writeHead(204).end();
    if (process.env["POCKET_FIXTURE_UNPAIRED"] === mac().name && target.startsWith("/api/")) {
      return response.writeHead(401, { "Content-Type": "text/plain" }).end("This phone is not paired with this Mac. Pair it again from Bridge, Settings, Phone.");
    }
    if (target.startsWith("/api/live")) {
      response.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache" });
      const beat = setInterval(() => response.write(": beat\n\n"), 15_000);
      request.on("close", () => clearInterval(beat));
      return;
    }
    const body = await read(request);
    const said = answer(mac(), posted, request.method ?? "GET", target, body, /^[0-9a-f]{128}$/.test(String(request.headers["x-pocket-signature"] ?? "")));
    if (posted.length > 0) console.log(`[fixture ${mac().name}] ${JSON.stringify(posted.splice(0))}`);
    response.writeHead(said.status, said.type === undefined ? {} : { "Content-Type": said.type }).end(said.body);
  };
}

/**
 * `pnpm fixture`: the app on a fixture Gateway with two Macs. The Mac serving the app is `studio`; the
 * one on the next port, with CORS as the real Gateway answers it, is `laptop`. `src/fixture/seed.ts`
 * pairs the phone with both on first load. `POCKET_FIXTURE_UNPAIRED=laptop` makes that Mac answer 401, as a
 * Mac that forgot the phone does.
 */
function fixtureGateway(): Plugin {
  const now = Date.now();
  const macs = fixtureMacs(now, { studio: "studio", laptop: "laptop" });
  return {
    name: "pocket-fixture-gateway",
    configureServer(server) {
      server.middlewares.use(serve(() => macs["studio"]!, []));
      server.httpServer?.once("listening", () => {
        const { port, address } = server.httpServer!.address() as AddressInfo;
        const beside = createServer(serve(() => macs["laptop"]!, []));
        beside.listen(port + 1, address, () => console.log(`  fixture laptop Mac on port ${port + 1}`));
        server.httpServer!.once("close", () => beside.close());
      });
    },
  };
}

// `appType: "spa"` so `/jobs/<Mac>/<id>` opens directly rather than 404ing.
export default defineConfig(({ mode }) => ({
  plugins: [react(), tailwindcss(), ...(mode === "fixture" ? [fixtureGateway()] : [])],
  appType: "spa",
}));
