// A shell tile the repository committed asks once before it first runs here, showing its command, and Run approves that command by
// its hash before pressing; an approved one runs at once, naming itself so Fleet runs the committed command. A tile of the person's
// that hides a committed one carries the mark naming it. Browser: a press and a rendering are what is read, from a fake `window.armada`.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { FlightControls, RepositoryFlightControls, TileAsk } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { NOTHING_YET } from "../../../../shared/bridge";
import { SessionTiles } from "./SessionTiles";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ROOT = "/Users/user/widgets";
const COMMITTED: FlightControls = { version: 1, job: [{ id: "run-tests", kind: "shell", name: "Run tests", text: "make test" }], deck: [] };
const ASK: TileAsk = { id: "run-tests", command: "make test", command_hash: "h1", approved: false };

function fleet({ asking = [ASK], mine = [] as FlightControls["job"], refuse = false } = {}) {
  const state = { ...NOTHING_YET, connection: { state: "connected" } } as unknown as BridgeState;
  const answer = (approved: boolean): { ok: true; value: RepositoryFlightControls } => ({
    ok: true,
    value: { repository: ROOT, place: "this_mac", read: { what: "read", controls: { version: 1, job: mine, deck: [] } }, in_repository: { what: "read", controls: COMMITTED }, asking: asking.map((one) => ({ ...one, approved: approved || one.approved })) },
  });
  const notApproved = { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.tile_not_approved", message: "Not approved", run_id: "", fields: { command: "make test-all", command_hash: "h2" }, chain: [] } } };
  const armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: state, seq: 1 }),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: { version: 1, job: [], deck: [] } } }),
    saveFlightControls: async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }),
    getRepositoryFlightControls: vi.fn(async () => answer(false)),
    approveRepositoryTile: vi.fn(async () => answer(true)),
    runSessionShell: vi.fn(async (..._args: unknown[]) => (refuse ? notApproved : { ok: true, value: { exit: 0, timed_out: false, output: "ok\n", truncated: false } })),
  };
  (window as unknown as { armada: unknown }).armada = armada;
  return armada;
}

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const session = { id: "s1", rows: [], attachments: [{ kind: "branch", name: "mock/widgets", slot: 2 }], turn: { state: "idle" } } as unknown as Session;
  const draft = { commands: [], get: () => [], send: vi.fn(async () => true) } as unknown as SessionsDraft;
  const bays = { bays: [{ slot: { slot: 2 }, branch: "mock/widgets", wrong: undefined, said: undefined }], clashes: [] } as never;
  act(() => root.render(<SessionTiles session={session} draft={draft} repository={{ root: ROOT, name: "acme/widgets" }} bays={bays} />));
  roots.push({ root, host });
  await flush();
  await flush();
  return host;
}

const press = (host: HTMLElement, name: string) => act(async () => host.querySelector<HTMLElement>(`button[aria-label="${name}"]`)!.click());
const ask = () => document.querySelector<HTMLElement>('[role="dialog"], [role="alertdialog"]');
const button = (within: HTMLElement, name: string) => [...within.querySelectorAll("button")].find((one) => one.textContent?.trim() === name)!;

test("a committed shell tile not approved here asks first, showing its command, and runs nothing", async () => {
  const armada = fleet();
  const host = await mount();
  expect(armada.getRepositoryFlightControls).toHaveBeenCalledWith(ROOT, "s1");
  await press(host, "Run tests");
  expect(ask()?.textContent).toContain("make test");
  expect(armada.runSessionShell).not.toHaveBeenCalled();
});

test("Run approves the command by the hash shown, then presses the tile naming itself", async () => {
  const armada = fleet();
  const host = await mount();
  await press(host, "Run tests");
  await act(async () => button(ask()!, "Run").click());
  await flush();
  expect(armada.approveRepositoryTile).toHaveBeenCalledWith({ repository: ROOT, session_id: "s1", tile_id: "run-tests", command_hash: "h1" });
  expect(armada.runSessionShell).toHaveBeenCalledWith("s1", "make test", { repository: ROOT, id: "run-tests" });
  expect(ask()).toBeNull();
});

test("an approved tile runs at once", async () => {
  const armada = fleet({ asking: [{ ...ASK, approved: true }] });
  const host = await mount();
  await press(host, "Run tests");
  await flush();
  expect(ask()).toBeNull();
  expect(armada.runSessionShell).toHaveBeenCalledWith("s1", "make test", { repository: ROOT, id: "run-tests" });
});

test("a press Fleet refuses as not approved asks with the command it names", async () => {
  fleet({ asking: [{ ...ASK, approved: true }], refuse: true });
  const host = await mount();
  await press(host, "Run tests");
  await flush();
  expect(ask()?.textContent).toContain("make test-all");
});

test("a tile of yours with the committed one's id hides it, and says so", async () => {
  fleet({ mine: [{ id: "run-tests", kind: "shell", name: "Run tests", text: "make test-fast" }] });
  const host = await mount();
  expect(host.querySelectorAll('button[aria-label="Run tests"]')).toHaveLength(1);
  expect(host.querySelector('[role="img"][aria-label="Hides the repository\'s Run tests"]')).not.toBeNull();
});

test("the ask is drawn over the whole window, out of the flight controls, and Cancel gives focus back to the tile", async () => {
  fleet();
  const host = await mount();
  const tile = host.querySelector<HTMLElement>('button[aria-label="Run tests"]')!;
  tile.focus();
  await act(async () => tile.click());
  expect(ask()).not.toBeNull();
  expect(host.contains(ask())).toBe(false);
  await act(async () => [...ask()!.querySelectorAll("button")].find((one) => one.textContent?.trim().startsWith("Cancel"))!.click());
  await flush();
  expect(ask()).toBeNull();
  expect(document.activeElement).toBe(tile);
});
