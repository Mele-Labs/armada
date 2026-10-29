// The real `CaptureWindow` shown with a Job's `steps` on a page that may play its own walk (`window.__armadaWalk`).
// The page's own walk wins: found at any time, the steps stop for good, the Job is told once, and the bar drives the
// page's walk. The steps' first act waits a moment after the load so a mock that mounts a beat late is found first.
// `docs/practices/capture-window.md`, *Steps Bridge plays*.

import type { WalkStep } from "@armada/protocol";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const world = vi.hoisted(() => ({
  /** The page's `did-*` listeners, by event. */
  handlers: new Map<string, Array<() => void>>(),
  /** What the page answers a read of `window.__armadaWalk` with: `null` for no walk. */
  walk: null as string | null,
  /** Scripts the page was asked to run, other than a read. */
  scripts: [] as string[],
  /** Every `StepWalk` made, with what it was told. */
  steps: [] as Array<{
    host: { stuck: (why: string) => void };
    begun: number;
    stopped: number;
    acted: string[];
  }>,
  /** What the Job was told: `[address, why]`. */
  told: [] as Array<[string, string]>,
}));

vi.mock("electron", () => ({
  BaseWindow: class {},
  screen: { getPrimaryDisplay: () => ({ workAreaSize: { width: 1920 } }) },
  shell: {},
}));
vi.mock("./views", () => ({ barView: () => view(false), pageView: () => view(true), panelView: () => view(false) }));
vi.mock("./finder", () => ({ PageFinder: class {} }));
vi.mock("./stepwalk", async () => ({
  ...(await vi.importActual<typeof import("./stepwalk")>("./stepwalk")),
  StepWalk: class {
    begun = 0;
    stopped = 0;
    acted: string[] = [];
    constructor(_name: string, _steps: unknown, readonly host: { stuck: (why: string) => void }) {
      world.steps.push(this);
    }
    state() {
      return { name: "steps", at: 1, of: 2, said: "", autoplay: false, ready: false, done: false, stopped: false, guide: { box: null, folded: false } };
    }
    begin() {
      this.begun += 1;
    }
    stop() {
      this.stopped += 1;
    }
    act(act: string) {
      this.acted.push(act);
    }
    guide() {}
    signIn() {}
  },
}));

import { CaptureWindow } from "./window";
import type { WalkSeat } from "./window";

const ADDRESS = "https://app.example.com/page";
const PAGE_WALK = JSON.stringify({ name: "Mock", at: 1, of: 3, said: "Hello", autoplay: false, ready: true, done: false });
const HELD_MS = 1000;

const STEPS: WalkStep[] = [{ say: "The stock table", look: { role: "table", name: "Stock" } }];

/** A view whose page is always at the address it was shown, and answers a walk read as `world.walk` says. */
function view(page: boolean) {
  return {
    setBounds: () => {},
    setVisible: () => {},
    webContents: {
      id: 1,
      on: (event: string, listener: () => void) => {
        if (page) world.handlers.set(event, [...(world.handlers.get(event) ?? []), listener]);
      },
      send: () => {},
      isDestroyed: () => false,
      isLoading: () => false,
      close: () => {},
      setWindowOpenHandler: () => {},
      getURL: () => ADDRESS,
      loadURL: () => Promise.resolve(),
      executeJavaScript: (source: string) => {
        if (source.includes("JSON.stringify(walk.state())")) return Promise.resolve(world.walk);
        world.scripts.push(source);
        return Promise.resolve(null);
      },
    },
  };
}

const native = () => ({
  contentView: { addChildView: () => {}, removeChildView: () => {} },
  getContentBounds: () => ({ width: 1280, height: 900 }),
  isDestroyed: () => false,
  isMinimized: () => false,
  setTitle: () => {},
  on: () => {},
  showInactive: () => {},
  show: () => {},
  restore: () => {},
  focus: () => {},
});

let shown: CaptureWindow | null = null;

/** The window in a walk window's seat, with the steps a Job passed. */
function window(steps: readonly WalkStep[] = STEPS): CaptureWindow {
  const seat = {
    window: native(),
    bar: view(false),
    panel: null,
    onScreen: () => true,
    queue: () => ({ entries: [], at: 0 }),
    key: () => {},
    settled: () => {},
  } as unknown as WalkSeat;
  const pin = { run: "run", name: "app", origin: "https://app.example.com", url: ADDRESS, manifestId: null, through: [] };
  const board = {
    focused: () => {},
    heldOn: () => 0,
    sessionFailed: (_id: string, address: string, why: string) => void world.told.push([address, why]),
  } as never;
  shown = new CaptureWindow(pin, { session: { id: "job" } } as never, board, { bar: 40, panel: 300, floor: 100 }, null, [], [], steps, seat);
  return shown;
}

/** The page finished loading: its listeners run, as Electron runs them. */
function loaded(): void {
  for (const event of ["dom-ready", "did-finish-load"]) for (const listener of world.handlers.get(event) ?? []) listener();
}

const hosted = (): number => world.scripts.filter((script) => script.includes("hostCard(true)")).length;
const SET_ASIDE = "steps set aside: this page plays its own walk";

beforeEach(() => {
  world.handlers.clear();
  world.walk = null;
  world.scripts = [];
  world.steps = [];
  world.told = [];
  vi.useFakeTimers();
});
afterEach(() => {
  shown?.dispose();
  shown = null;
  vi.useRealTimers();
});

it("drops the steps for the page's own walk, hands the card to the bar and tells the Job once", async () => {
  window();
  world.walk = PAGE_WALK;
  loaded();
  await vi.advanceTimersByTimeAsync(2000);

  const [steps] = world.steps;
  expect(steps?.stopped).toBe(1);
  expect(steps?.begun).toBe(0);
  expect(hosted()).toBe(1);
  expect(world.told).toEqual([[ADDRESS, SET_ASIDE]]);
});

it("keeps looking for the page's walk past the usual tries while the steps stand, then drops them when it mounts", async () => {
  window();
  loaded();
  // Well past `WALK_FIND_TRIES` misses at the quick beat, with the steps begun long ago.
  await vi.advanceTimersByTimeAsync(20_000);
  const [steps] = world.steps;
  expect(steps?.begun).toBe(1);
  expect(steps?.stopped).toBe(0);

  world.walk = PAGE_WALK;
  await vi.advanceTimersByTimeAsync(1000);
  expect(steps?.stopped).toBe(1);
  expect(world.told).toEqual([[ADDRESS, SET_ASIDE]]);
});

it("runs the bar's act on the page's walk, not on the steps, once they are dropped", async () => {
  const page = window();
  world.walk = PAGE_WALK;
  loaded();
  await vi.advanceTimersByTimeAsync(1000);

  await page.walkAct("next");
  expect(world.scripts.some((script) => script.includes("__armadaWalk.next()"))).toBe(true);
  expect(world.steps[0]?.acted).toEqual([]);
});

it("does not begin the steps before the hold after the load is over", async () => {
  window();
  loaded();
  await vi.advanceTimersByTimeAsync(HELD_MS - 1);
  expect(world.steps[0]?.begun).toBe(0);
  await vi.advanceTimersByTimeAsync(1);
  expect(world.steps[0]?.begun).toBe(1);
});

it("never begins the steps when the page's walk is found within the hold", async () => {
  window();
  loaded();
  await vi.advanceTimersByTimeAsync(HELD_MS / 2);
  world.walk = PAGE_WALK;
  await vi.advanceTimersByTimeAsync(HELD_MS * 2);
  expect(world.steps[0]?.begun).toBe(0);
  expect(world.steps[0]?.stopped).toBe(1);
});

it("a later load restarts the hold", async () => {
  window();
  loaded();
  await vi.advanceTimersByTimeAsync(HELD_MS * 0.6);
  loaded();
  await vi.advanceTimersByTimeAsync(HELD_MS * 0.6);
  expect(world.steps[0]?.begun).toBe(0);
  await vi.advanceTimersByTimeAsync(HELD_MS * 0.4);
  expect(world.steps[0]?.begun).toBe(1);
});

it("ignores Back and Replay until the steps have begun, and passes them on after", async () => {
  const page = window();
  loaded();
  await page.walkAct("back");
  await page.walkAct("replay");
  expect(world.steps[0]?.acted).toEqual([]);
  await page.walkAct("next");
  expect(world.steps[0]?.acted).toEqual(["next"]);

  await vi.advanceTimersByTimeAsync(HELD_MS);
  await page.walkAct("replay");
  expect(world.steps[0]?.acted).toEqual(["next", "replay"]);
});

it("tells the Job a step is stuck while the steps stand, and nothing from an act still in flight after they are dropped", async () => {
  window();
  loaded();
  await vi.advanceTimersByTimeAsync(HELD_MS);
  const [steps] = world.steps;
  steps?.host.stuck("step 1: held at the table");
  expect(world.told).toEqual([[ADDRESS, "step 1: held at the table"]]);

  world.walk = PAGE_WALK;
  await vi.advanceTimersByTimeAsync(1000);
  steps?.host.stuck("step 1: late report");
  expect(world.told.map(([, why]) => why)).toEqual(["step 1: held at the table", SET_ASIDE]);
});
