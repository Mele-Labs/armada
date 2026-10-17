// The page an API walk plays over, in Armada's own walk window: the walk's requests, each with a button that sends it, and under
// it the response. **The walk's steps press those buttons**, so an API walks the way a web page does and the window has one way to
// walk anything (`shared/review-walk.ts`). A page cannot send these itself, since the API sits on another origin and a browser
// would refuse the answer, so the requests go from here: the page asks this server to send request N, and this server sends
// it to the base address and hands back what came.
//
// **Bound to loopback on a port of its own, and it sends nothing but the walk's own requests.** The page names a number and
// never a path or a host; a request to the server needs the token only the page holds, which a page on any other origin cannot
// read; and a walk that is over stops the server with its window.

import { randomBytes } from "node:crypto";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";

import type { ReviewRequest } from "@armada/protocol";

import { sendName } from "../../shared/review-walk";

/** The longest a request waits for the API, and the most of its answer that is drawn. */
const SENT_MS = 15_000;
const MOST_BODY = 100_000;

/** What came back for a request, in the words the page draws. `status` 0 is no answer at all, and `error` says why. */
export type Sent = { status: number; ms: number; body: string; error?: string };

export type Runner = { url: string; stop: () => Promise<void> };

const escaped = (text: string): string => text.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c] ?? c);

/** Whether `base` is an http or https address with a host, and the one request address it makes for `path`, or `null` where it would leave the base's origin. */
export function addressOf(base: string, path: string): string | null {
  try {
    const root = new URL(base);
    if (!/^https?:$/.test(root.protocol) || root.host === "") return null;
    if (!path.startsWith("/") || path.startsWith("//")) return null;
    const made = new URL(`${root.origin}${root.pathname.replace(/\/+$/, "")}${path}`);
    return made.origin === root.origin ? made.toString() : null;
  } catch {
    return null;
  }
}

/** A body drawn as it came: JSON indented, anything else as it is, and cut where it is long. */
function drawn(text: string): string {
  let shown = text;
  try {
    shown = JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    // Not JSON: as it came.
  }
  return shown.length > MOST_BODY ? `${shown.slice(0, MOST_BODY)}\n[cut]` : shown;
}

/** Sends request `at` of the walk to `base`. Never throws: a request that got no answer is a `Sent` that says so. */
export async function send(base: string, request: ReviewRequest, through: typeof fetch = fetch): Promise<Sent> {
  const address = addressOf(base, request.path);
  const began = Date.now();
  if (address === null) return { status: 0, ms: 0, body: "", error: `${request.path} is not a path on ${base}` };
  try {
    const answer = await through(address, {
      method: request.method,
      headers: request.body === undefined ? {} : { "content-type": "application/json" },
      ...(request.body === undefined || request.method === "GET" ? {} : { body: request.body }),
      redirect: "manual",
      signal: AbortSignal.timeout(SENT_MS),
    });
    return { status: answer.status, ms: Date.now() - began, body: drawn(await answer.text()) };
  } catch (cause) {
    return { status: 0, ms: Date.now() - began, body: "", error: cause instanceof Error ? cause.message : String(cause) };
  }
}

const STYLE = `
:root { color-scheme: light dark; font: 14px/1.45 ui-sans-serif, system-ui, sans-serif; }
body { margin: 0; padding: 16px; background: Canvas; color: CanvasText; }
main { max-width: 860px; margin: 0 auto; }
code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.against { display: flex; gap: 8px; align-items: baseline; margin: 0 0 16px; }
ol { list-style: none; margin: 0; padding: 0; display: grid; gap: 12px; }
section { border: 1px solid GrayText; border-radius: 6px; }
header { display: flex; gap: 8px; align-items: center; padding: 8px 12px; }
header .n { color: GrayText; min-width: 1.5em; }
header .path { flex: 1; overflow-wrap: anywhere; }
button { font: inherit; padding: 4px 12px; border: 1px solid GrayText; border-radius: 4px; background: ButtonFace; color: ButtonText; cursor: pointer; }
button:focus-visible { outline: 2px solid Highlight; outline-offset: 2px; }
button[aria-busy="true"] { opacity: 0.6; cursor: progress; }
pre { margin: 0; padding: 8px 12px; border-top: 1px solid GrayText; overflow: auto; max-height: 320px; white-space: pre-wrap; }
.response p { margin: 0; padding: 8px 12px 0; display: flex; gap: 12px; }
.response .status { font-weight: 600; }
.response[data-ok="false"] .status { text-decoration: underline; }
`;

const SCRIPT = `
const token = document.body.dataset.token;
document.querySelectorAll("button[data-n]").forEach((button) => {
  button.addEventListener("click", async () => {
    if (button.getAttribute("aria-busy") === "true") return;
    button.setAttribute("aria-busy", "true");
    const group = document.getElementById("response-" + button.dataset.n);
    try {
      const answer = await fetch("/send/" + button.dataset.n, { method: "POST", headers: { "x-armada-runner": token } });
      const sent = await answer.json();
      group.querySelector(".status").textContent = sent.status === 0 ? "No answer" : String(sent.status);
      group.querySelector(".ms").textContent = sent.ms + " ms";
      group.querySelector("pre").textContent = sent.error === undefined ? sent.body : sent.error;
      group.dataset.ok = String(sent.status > 0 && sent.status < 400);
      group.hidden = false;
    } finally {
      button.removeAttribute("aria-busy");
    }
  });
});
`;

/** The page, as HTML. Pure, so it is read without a server. */
export function runnerPage(base: string, requests: readonly ReviewRequest[], token: string): string {
  const items = requests
    .map((request, at) => {
      const name = escaped(sendName(request, at));
      const body = request.body === undefined ? "" : `<pre aria-label="Request body">${escaped(drawn(request.body))}</pre>`;
      return (
        `<li><section aria-label="Request ${at + 1}"><header><span class="n">${at + 1}</span><code>${escaped(request.method)}</code>` +
        `<code class="path">${escaped(request.path)}</code><button type="button" data-n="${at}" aria-label="${name}">Send</button></header>${body}` +
        `<div class="response" id="response-${at}" role="group" aria-label="Response ${at + 1}" hidden><p><span class="status"></span><span class="ms"></span></p><pre></pre></div>` +
        `</section></li>`
      );
    })
    .join("");
  return (
    `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Requests</title><link rel="stylesheet" href="/runner.css"></head>` +
    `<body data-token="${escaped(token)}"><main aria-label="Requests"><p class="against"><span>API base address</span><code>${escaped(base)}</code></p>` +
    `<ol>${items}</ol></main><script src="/runner.js"></script></body></html>`
  );
}

const SECURITY = {
  "content-security-policy": "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'",
  "x-content-type-options": "nosniff",
  "cache-control": "no-store",
} as const;

/**
 * Starts the runner for one walk. `through` is the fetch that reaches the API, which a test hands its own.
 * The server listens on loopback only and answers three reads and one send.
 */
export async function startRunner(base: string, requests: readonly ReviewRequest[], through: typeof fetch = fetch): Promise<Runner> {
  const token = randomBytes(24).toString("hex");
  const page = runnerPage(base, requests, token);
  let port = 0;
  const reply = (res: ServerResponse, status: number, type: string, body: string): void => {
    res.writeHead(status, { "content-type": `${type}; charset=utf-8`, ...SECURITY });
    res.end(body);
  };
  const handle = async (req: IncomingMessage, res: ServerResponse): Promise<void> => {
    // A page on another origin that was pointed at this port arrives with its own Host, or with no token.
    if (req.headers.host !== `127.0.0.1:${port}`) return reply(res, 403, "text/plain", "");
    if (req.method === "GET" && req.url === "/") return reply(res, 200, "text/html", page);
    if (req.method === "GET" && req.url === "/runner.css") return reply(res, 200, "text/css", STYLE);
    if (req.method === "GET" && req.url === "/runner.js") return reply(res, 200, "text/javascript", SCRIPT);
    const sending = /^\/send\/(\d+)$/.exec(req.url ?? "");
    if (req.method === "POST" && sending !== null && req.headers["x-armada-runner"] === token) {
      const request = requests[Number(sending[1])];
      if (request === undefined) return reply(res, 404, "text/plain", "");
      return reply(res, 200, "application/json", JSON.stringify(await send(base, request, through)));
    }
    return reply(res, 404, "text/plain", "");
  };
  const server: Server = createServer((req, res) => void handle(req, res));
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  port = (server.address() as AddressInfo).port;
  return {
    url: `http://127.0.0.1:${port}/`,
    stop: () =>
      new Promise<void>((resolve) => {
        server.closeAllConnections();
        server.close(() => resolve());
      }),
  };
}
