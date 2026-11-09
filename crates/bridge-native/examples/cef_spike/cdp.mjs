// Drives the spike's page over the Chrome DevTools Protocol, the way a walk of a web page would.
// Usage: node cdp.mjs <port> <out dir>
const [port, out] = [process.argv[2] ?? "9333", process.argv[3] ?? "/tmp/cef-spike-out"];
import { writeFileSync } from "node:fs";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function get(path) {
  const r = await fetch(`http://127.0.0.1:${port}${path}`);
  return { status: r.status, text: await r.text() };
}

// 1. How soon does the port answer, and with what.
const t0 = Date.now();
let version;
for (;;) {
  try { version = await get("/json/version"); break; } catch { await sleep(50); }
}
console.log(`CDP /json/version answered ${version.status} after ${Date.now() - t0} ms of polling`);
console.log(version.text);
const targets = JSON.parse((await get("/json/list")).text);
console.log("CDP targets:", targets.map((t) => `${t.type} ${t.url}`).join(" | "));
const page = targets.find((t) => t.type === "page");

// 2. A raw WebSocket session on the page target.
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
let id = 0;
const waiting = new Map();
ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && waiting.has(d.id)) waiting.get(d.id)(d); };
const call = (method, params = {}) => new Promise((res) => { const i = ++id; waiting.set(i, res); ws.send(JSON.stringify({ id: i, method, params })); });
const val = async (expression) => (await call("Runtime.evaluate", { expression, returnByValue: true })).result.result.value;

console.log("CDP title:", await val("document.title"));
console.log("CDP userAgent:", await val("navigator.userAgent"));
console.log("CDP devicePixelRatio:", await val("window.devicePixelRatio"), "inner:", await val("innerWidth + 'x' + innerHeight"));

// 3. Page.captureScreenshot works without any window.
const shot = await call("Page.captureScreenshot", { format: "png" });
writeFileSync(`${out}/cdp-screenshot.png`, Buffer.from(shot.result.data, "base64"));
console.log("CDP captureScreenshot bytes:", Buffer.from(shot.result.data, "base64").length);

// 4. Change the page from CDP, as a walk step would.
await val("document.getElementById('top').querySelector('h1').textContent = 'Changed over CDP'");

// 5. Read what the gpui side typed and scrolled, after the app's scripted session ran.
await sleep(Number(process.env.CDP_WAIT_MS ?? 7000));
console.log("CDP input value after gpui-forwarded keys:", JSON.stringify(await val("window.__spike.input()")));
console.log("CDP scroller scrollTop after gpui-forwarded wheel:", await val("window.__spike.scrollTop()"));
console.log("CDP rAF frames counted by the page:", await val("window.__spike.frames()"));
ws.close();
