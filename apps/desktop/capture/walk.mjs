// A walk photographed with nobody watching: `pnpm -C apps/desktop walk <name>`.
// One PNG per step, and a recording of the whole walk with `--video`.
//
// **It plays the walk the way the link does**, by loading `?walk=<name>` and
// pressing the card's own Next, so a picture here is a picture of what the owner
// would see on the link. `docs/practices/running-locally.md`, *Walks*.
//
//   pnpm -C apps/desktop walk backFromADrone
//   pnpm -C apps/desktop walk backFromADrone --video --size 1512x817
//   pnpm -C apps/desktop walk backFromADrone --url http://localhost:41097
//
// With no `--url` it starts the mock itself on a free port and stops it after.
// It writes to `.armada/walks/<name>-<when>/` at the repository root and prints
// that folder. It exits 1 when the walk stops on a step, after photographing
// the stop.

import { mkdirSync, renameSync } from "node:fs";
import { createServer as freePort } from "node:net";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const HERE = dirname(fileURLToPath(import.meta.url));
const DESKTOP = join(HERE, "..");
const REPOSITORY = join(DESKTOP, "..", "..");

/** How long a step stays on screen in a recording, so it can be read. */
const DWELL_MS = 2_500;
/** Longer than the walk's own five seconds for a target, so the walk is what says it stopped. */
const STEP_TIMEOUT_MS = 30_000;

const USAGE = "usage: pnpm -C apps/desktop walk <name> [--video] [--size 1440x900] [--url http://localhost:<port>]";

function parse(argv) {
  const asked = { name: undefined, video: false, size: { width: 1440, height: 900 }, url: undefined };
  for (let at = 0; at < argv.length; at += 1) {
    const one = argv[at];
    if (one === "--video") asked.video = true;
    else if (one === "--url") asked.url = argv[(at += 1)];
    else if (one === "--size") {
      const match = /^(\d+)x(\d+)$/.exec(argv[(at += 1)] ?? "");
      if (match === null) throw new Error(`--size takes WIDTHxHEIGHT\n${USAGE}`);
      asked.size = { width: Number(match[1]), height: Number(match[2]) };
    } else if (one.startsWith("--")) throw new Error(`no option ${one}\n${USAGE}`);
    else asked.name = one;
  }
  if (asked.name === undefined) throw new Error(USAGE);
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

/** The caption as a file name: lower case, words joined by hyphens. */
const slug = (words) =>
  words
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 60);

const pad = (n) => String(n).padStart(2, "0");

const stamp = () => new Date().toISOString().slice(0, 19).replace(/[-:]/g, "").replace("T", "-");

/** Where the walk's card says it is, once it has settled on step `at`, stopped, or finished. */
async function settled(page, at) {
  await page.waitForFunction(
    (step) => {
      const card = document.querySelector("[data-walk-state]");
      if (card === null) return false;
      const state = card.getAttribute("data-walk-state");
      return state === "stopped" || state === "done" || (state === "ready" && card.getAttribute("data-walk-step") === String(step));
    },
    at,
    { timeout: STEP_TIMEOUT_MS },
  );
  const card = page.locator("[data-walk-state]");
  return {
    state: await card.getAttribute("data-walk-state"),
    said: (await page.locator(".armada-mock-walk__said").textContent().catch(() => null))?.trim() ?? "",
  };
}

async function main() {
  const asked = parse(process.argv.slice(2));
  const out = join(REPOSITORY, ".armada", "walks", `${asked.name}-${stamp()}`);
  mkdirSync(out, { recursive: true });

  let server;
  let base = asked.url;
  if (base === undefined) {
    const chosen = await port();
    server = await createServer({
      configFile: join(DESKTOP, "vite.mock.config.ts"),
      server: { port: chosen, strictPort: true, host: "127.0.0.1" },
      logLevel: "error",
    });
    await server.listen();
    base = `http://127.0.0.1:${chosen}`;
  }

  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: asked.size,
    ...(asked.video ? { recordVideo: { dir: out, size: asked.size } } : {}),
  });
  const page = await context.newPage();
  // A throw is a failure even where the walk went on: Bridge draws one as a
  // banner and keeps going, so the pictures would look like a walk that passed.
  const thrown = [];
  page.on("pageerror", (error) => {
    thrown.push(error.message);
    console.error(`the page threw: ${error.message}`);
  });
  const written = [];
  let stop = null;
  try {
    const link = `${base.replace(/\/$/, "")}/?walk=${encodeURIComponent(asked.name)}`;
    console.log(`walk ${asked.name}, from ${link}`);
    await page.goto(link);
    for (let at = 1; ; at += 1) {
      const { state, said } = await settled(page, at);
      if (state === "done") {
        // The last press's own result, which the last step's picture was taken before.
        await page.waitForTimeout(600);
        const file = join(out, `${pad(at)}-where-it-ended.png`);
        await page.screenshot({ path: file });
        written.push(file);
        break;
      }
      const file = join(out, `${pad(at)}-${state === "stopped" ? "stopped" : slug(said)}.png`);
      await page.screenshot({ path: file });
      written.push(file);
      if (state === "stopped") {
        stop = said;
        if (asked.video) await page.waitForTimeout(DWELL_MS);
        break;
      }
      if (asked.video) await page.waitForTimeout(DWELL_MS);
      await page.locator("[data-walk-ui]").getByRole("button", { name: "Next", exact: true }).click();
    }
  } finally {
    const video = page.video();
    await context.close();
    if (video !== null) {
      const file = join(out, `${asked.name}.webm`);
      renameSync(await video.path(), file);
      written.push(file);
    }
    await browser.close();
    await server?.close();
  }

  console.log(`wrote ${out}/`);
  for (const file of written) console.log(`  ${relative(out, file)}`);
  if (stop !== null) console.error(stop);
  if (stop !== null || thrown.length > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 2;
});
