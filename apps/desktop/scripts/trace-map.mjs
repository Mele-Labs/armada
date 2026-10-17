// What the Cockpit map costs to draw, measured in headless Chromium: `pnpm -C apps/desktop trace-map`.
// Three phases on the `battlefield-progress` scenario, each with its CPU time, frames and composited layers:
//   idle  nothing happens; the CSS map still runs its loops (bob, ping, breathe).
//   busy  a moment of the scenario passes every second: edits land, subagents fly, a pull request opens, queues, merges.
//   pan   the wheel pans and zooms the camera.
// Two passes on a fresh page each, because tracing perturbs what it measures: one reads CDP counters and the process CPU
// times, the other counts frames from a trace. Nothing is shown on a screen.
//
//   taskpolicy -c utility pnpm -C apps/desktop trace-map
//   taskpolicy -c utility pnpm -C apps/desktop trace-map --map pixi --label after
//
// Numbers go to stdout as a table and to `.armada/traces/<label>-<when>.json`. The software GL of a headless browser draws the
// canvas on the CPU, in the GPU process, so read `GPU` beside `renderer` before comparing two maps.

import { mkdirSync, writeFileSync } from "node:fs";
import { createServer as freePort } from "node:net";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const HERE = dirname(fileURLToPath(import.meta.url));
const DESKTOP = join(HERE, "..");
const REPOSITORY = join(DESKTOP, "..", "..");

const USAGE = "usage: trace-map [--map css|pixi] [--label name] [--idle 10] [--busy 10] [--pan 5] [--quiet] [--size 1440x900] [--url http://localhost:<port>]";

function parse(argv) {
  const asked = { map: "css", label: "baseline", idle: 10, busy: 10, pan: 5, quiet: false, size: { width: 1440, height: 900 }, url: undefined };
  for (let at = 0; at < argv.length; at += 1) {
    const one = argv[at];
    const value = () => argv[(at += 1)];
    if (one === "--map") asked.map = value();
    else if (one === "--label") asked.label = value();
    else if (one === "--idle") asked.idle = Number(value());
    else if (one === "--busy") asked.busy = Number(value());
    else if (one === "--pan") asked.pan = Number(value());
    else if (one === "--url") asked.url = value();
    else if (one === "--quiet") asked.quiet = true;
    else if (one === "--size") {
      const match = /^(\d+)x(\d+)$/.exec(value() ?? "");
      if (match === null) throw new Error(`--size takes WIDTHxHEIGHT\n${USAGE}`);
      asked.size = { width: Number(match[1]), height: Number(match[2]) };
    } else throw new Error(`no option ${one}\n${USAGE}`);
  }
  return asked;
}

function port() {
  return new Promise((resolve, reject) => {
    const probe = freePort();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const { port: found } = probe.address();
      probe.close(() => resolve(found));
    });
  });
}

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const METRICS = ["TaskDuration", "ScriptDuration", "LayoutDuration", "RecalcStyleDuration"];

async function metrics(cdp) {
  const { metrics: list } = await cdp.send("Performance.getMetrics");
  return Object.fromEntries(list.map((one) => [one.name, one.value]));
}

/** CPU seconds each kind of process has used so far. */
async function processes(browserCdp) {
  const { processInfo } = await browserCdp.send("SystemInfo.getProcessInfo");
  const out = {};
  for (const one of processInfo) out[one.type] = (out[one.type] ?? 0) + one.cpuTime;
  return out;
}

const delta = (after, before) => Object.fromEntries(Object.keys(after).map((key) => [key, (after[key] ?? 0) - (before[key] ?? 0)]));
const round = (n) => Math.round(n * 1000) / 1000;

/** Drive one phase for `seconds` and return how long it really ran. */
async function drive(page, phase, seconds) {
  const end = Date.now() + seconds * 1000;
  if (phase === "busy") {
    // One moment of the scenario a second: the story's own twelve hours, drawn as they would be.
    while (Date.now() < end) {
      await page.evaluate(() => import("/time-passes.ts").then((mod) => mod.timePasses()));
      await sleep(1000);
    }
  } else if (phase === "pan") {
    const box = await page.locator("[role=listbox][aria-label=Map]").first().boundingBox();
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;
    await page.mouse.move(x, y);
    let turn = 0;
    while (Date.now() < end) {
      // Two-finger slides pan, a ctrl wheel zooms: the small deltas a trackpad sends, sixty a second.
      turn += 1;
      await page.mouse.wheel(Math.sin(turn / 12) * 14, Math.cos(turn / 17) * 10);
      if (turn % 90 === 0) await page.keyboard.press("+");
      if (turn % 90 === 45) await page.keyboard.press("-");
      await sleep(16);
    }
  } else {
    await sleep(seconds * 1000);
  }
}

async function open(browser, asked, base) {
  const context = await browser.newContext({ viewport: asked.size });
  const page = await context.newPage();
  const thrown = [];
  page.on("pageerror", (error) => thrown.push(error.message));
  const link = `${base.replace(/\/$/, "")}/?scenario=battlefield-progress&frame${asked.map === "css" ? "" : `&map=${asked.map}`}`;
  await page.goto(link);
  await page.locator("[role=listbox][aria-label=Map]").first().waitFor({ timeout: 30_000 });
  // The canvas map starts a moment after its listbox is there; software GL takes the better part of a minute.
  if (asked.map === "pixi") await page.locator(".bf-pixi canvas").first().waitFor({ state: "attached", timeout: 120_000 });
  // The map's arrival animations and the first layout settle before anything is read.
  await sleep(4000);
  // `--quiet` measures the page as it is after a minute with no input: the mark `still.ts` sets then, which the mock's entry does not run.
  if (asked.quiet) {
    await page.evaluate(() => document.documentElement.setAttribute("data-idle", ""));
    await sleep(1000);
  }
  return { context, page, thrown };
}

/** Pass one: the counters and the process CPU times, no tracing. */
async function counters(browser, asked, base, phases) {
  const { context, page, thrown } = await open(browser, asked, base);
  const cdp = await context.newCDPSession(page);
  const root = await browser.newBrowserCDPSession();
  await cdp.send("Performance.enable", { timeDomain: "timeTicks" });
  let layers = 0;
  cdp.on("LayerTree.layerTreeDidChange", (event) => {
    layers = event.layers?.length ?? layers;
  });
  await cdp.send("LayerTree.enable");
  const out = {};
  for (const [phase, seconds] of phases) {
    const before = await metrics(cdp);
    const cpuBefore = await processes(root);
    const started = Date.now();
    await drive(page, phase, seconds);
    const wall = (Date.now() - started) / 1000;
    const after = await metrics(cdp);
    const cpuAfter = await processes(root);
    const d = delta(after, before);
    out[phase] = {
      seconds: round(wall),
      renderer: Object.fromEntries(METRICS.map((name) => [name, round(d[name] ?? 0)])),
      cpu: Object.fromEntries(Object.entries(delta(cpuAfter, cpuBefore)).map(([kind, value]) => [kind, round(value)])),
      layers,
      nodes: after.Nodes,
    };
  }
  await context.close();
  return { out, thrown };
}

/** Pass two: a trace, counting the frames the compositor drew and the paints the main thread made. */
async function frames(browser, asked, base, phases) {
  const { context, page } = await open(browser, asked, base);
  const root = await browser.newBrowserCDPSession();
  const out = {};
  for (const [phase, seconds] of phases) {
    const events = [];
    root.removeAllListeners("Tracing.dataCollected");
    root.on("Tracing.dataCollected", (event) => events.push(...event.value));
    const finished = new Promise((done) => root.once("Tracing.tracingComplete", done));
    await root.send("Tracing.start", { traceConfig: { includedCategories: ["devtools.timeline", "disabled-by-default-devtools.timeline.frame", "cc"] }, transferMode: "ReportEvents" });
    await drive(page, phase, seconds);
    await root.send("Tracing.end");
    await finished;
    const count = (name) => events.filter((one) => one.name === name).length;
    out[phase] = { drawnFrames: count("DrawFrame"), paints: count("Paint"), layouts: count("Layout"), styleRecalcs: count("UpdateLayoutTree"), rasterTasks: count("RasterTask") };
  }
  await context.close();
  return out;
}

async function main() {
  const asked = parse(process.argv.slice(2));
  const phases = [
    ["idle", asked.idle],
    ["busy", asked.busy],
    ["pan", asked.pan],
  ].filter(([, seconds]) => seconds > 0);
  let server;
  let base = asked.url;
  if (base === undefined) {
    const chosen = await port();
    server = await createServer({ configFile: join(DESKTOP, "vite.mock.config.ts"), server: { port: chosen, strictPort: true, host: "127.0.0.1" }, logLevel: "error" });
    await server.listen();
    base = `http://127.0.0.1:${chosen}`;
  }
  // The default launch is the headless shell: no window, no Dock icon. Its GL is software, which the canvas map needs asked for.
  const browser = await chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
  try {
    const first = await counters(browser, asked, base, phases);
    const second = await frames(browser, asked, base, phases);
    const result = { label: asked.label, map: asked.map, size: asked.size, at: new Date().toISOString(), thrown: first.thrown, phases: {} };
    for (const [phase] of phases) result.phases[phase] = { ...first.out[phase], ...second[phase] };
    const folder = join(REPOSITORY, ".armada", "traces");
    mkdirSync(folder, { recursive: true });
    const file = join(folder, `${asked.label}-${asked.map}-${result.at.slice(0, 19).replace(/[-:]/g, "").replace("T", "-")}.json`);
    writeFileSync(file, JSON.stringify(result, null, 2));
    console.log(`${asked.label} (map ${asked.map}, ${asked.size.width}x${asked.size.height})`);
    console.table(
      Object.fromEntries(
        Object.entries(result.phases).map(([phase, one]) => [
          phase,
          {
            s: one.seconds,
            task: one.renderer.TaskDuration,
            script: one.renderer.ScriptDuration,
            layout: one.renderer.LayoutDuration,
            style: one.renderer.RecalcStyleDuration,
            "cpu renderer": one.cpu.renderer,
            "cpu gpu": one.cpu.GPU,
            "cpu browser": one.cpu.browser,
            drawn: one.drawnFrames,
            paints: one.paints,
            layers: one.layers,
          },
        ]),
      ),
    );
    if (result.thrown.length > 0) {
      console.error(`the page threw: ${result.thrown.join("; ")}`);
      process.exitCode = 1;
    }
    console.log(`wrote ${file}`);
  } finally {
    await browser.close();
    await server?.close();
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 2;
});
