// The flight controls as the owner arranged them: the order tiles are drawn in, which are hidden, which kinds each set takes, what Bridge
// refuses before a save, and the store that reads them from Fleet and saves them to it. Node: no window, a fake Fleet.

import type { FlightControls, FlightControlsRead } from "@armada/protocol";
import { ACTION, pressesChord } from "@armada/components";
import { describe, expect, test, vi } from "vitest";

import type { FlightControlsAnswer, RepositoryFlightControlsAnswer } from "../../../../shared/api/flight-controls";
import type { BridgeState } from "../../../../shared/bridge";
import { DECK_BUILT_INS, JOB_BUILT_INS, NOTHING_CHANGED, createFlightStore, keyChord, keyPressed, keyProblem, refusalOf, tilesOf, type FlightEntry, type FlightFile, type FlightFleet } from "./flight-controls";

const file = (job: FlightEntry[] = [], deck: FlightEntry[] = []): FlightFile => ({ version: 1, job, deck });
const ids = (set: "job" | "deck", of: FlightFile, repository?: FlightFile) => tilesOf(set, of, repository).map((one) => one.id);

describe("tilesOf", () => {
  test("with nothing changed, a set is the built-ins in Armada's order", () => {
    expect(ids("job", NOTHING_CHANGED)).toEqual(JOB_BUILT_INS.map((one) => one.id));
    expect(ids("deck", NOTHING_CHANGED)).toEqual(DECK_BUILT_INS.map((one) => one.id));
  });

  test("the file's entries come first in their order, then each built-in it does not name", () => {
    const order = ids("job", file([{ id: "pause" }, { id: "my-docs", kind: "page", text: "https://docs.example" }, { id: "ask" }]));
    expect(order.slice(0, 3)).toEqual(["pause", "my-docs", "ask"]);
    expect(order.slice(3)).toEqual(JOB_BUILT_INS.map((one) => one.id).filter((id) => id !== "pause" && id !== "ask"));
  });

  test("a repeated id is drawn once, at its first place", () => {
    expect(ids("job", file([{ id: "map" }, { id: "ask" }, { id: "map" }])).slice(0, 2)).toEqual(["map", "ask"]);
  });

  test("a hidden tile is kept in the list, has no key, and does not take a position's digit", () => {
    const tiles = tilesOf("job", file([{ id: "ask", hidden: true }]));
    expect(tiles[0]).toMatchObject({ id: "ask", hidden: true, key: undefined });
    expect(tiles[1]).toMatchObject({ id: "land", key: "⌥⇧1" });
  });

  test("keys follow position, an explicit key wins, and the tile whose digit it took has none", () => {
    const tiles = tilesOf("job", file([{ id: "ask" }, { id: "land", key: "⌥⇧1" }, { id: "compact" }]));
    expect(tiles.map((one) => one.key).slice(0, 3)).toEqual([undefined, "⌥⇧1", "⌥⇧3"]);
  });

  test("Pause answers p however it is placed", () => {
    expect(tilesOf("job", file([{ id: "pause" }])).find((one) => one.id === "pause")?.key).toBe("p");
  });

  test("the job set takes all four kinds of an action of the owner's", () => {
    const tiles = tilesOf("job", file([
      { id: "w", kind: "words", text: "hello" },
      { id: "c", kind: "command", text: "/review" },
      { id: "s", kind: "shell", text: "pnpm typecheck" },
      { id: "p", kind: "page", text: "https://docs.example" },
    ]));
    expect(tiles.slice(0, 4).map((one) => one.kind)).toEqual(["words", "command", "shell", "page"]);
  });

  test("the deck set takes only a page: the other kinds are dropped", () => {
    const tiles = tilesOf("deck", file([], [
      { id: "w", kind: "words", text: "hello" },
      { id: "c", kind: "command", text: "/review" },
      { id: "s", kind: "shell", text: "pnpm typecheck" },
      { id: "p", kind: "page", text: "https://docs.example" },
    ]));
    expect(tiles.map((one) => one.id)).toEqual(["p", ...DECK_BUILT_INS.map((one) => one.id)]);
  });

  test("an action with a kind nobody knows, or none, is dropped", () => {
    const odd = { id: "x", kind: "teleport", text: "now" } as unknown as FlightEntry;
    expect(ids("job", file([odd, { id: "y", text: "no kind" }]))).toEqual(JOB_BUILT_INS.map((one) => one.id));
  });

  test("a built-in ignores a kind laid over it", () => {
    expect(tilesOf("job", file([{ id: "ask", kind: "shell" }]))[0]).toMatchObject({ id: "ask", kind: undefined });
  });
});

describe("tilesOf, with a repository's own", () => {
  const SPECS: FlightEntry = { id: "my-run-specs", kind: "shell", name: "Run specs", text: "bin/rspec" };
  const LINT: FlightEntry = { id: "my-lint", kind: "shell", name: "Lint", text: "bin/rubocop" };
  const JOB = JOB_BUILT_INS.map((one) => one.id);
  const withoutPause = JOB.filter((id) => id !== "pause");

  test("a repository's tiles are drawn after yours, in their order, and before Pause", () => {
    const tiles = tilesOf("job", NOTHING_CHANGED, file([SPECS, LINT]));
    expect(tiles.map((one) => one.id)).toEqual([...withoutPause, "my-run-specs", "my-lint", "pause"]);
    expect(tiles.map((one) => one.fromRepository)).toEqual([...withoutPause.map(() => false), true, true, false]);
  });

  test("with Pause moved among yours, it stays where you put it and the repository's come after the last of yours", () => {
    expect(ids("job", file([{ id: "pause" }]), file([SPECS])).slice(0, 1)).toEqual(["pause"]);
    expect(ids("job", file([{ id: "pause" }]), file([SPECS])).at(-1)).toEqual("my-run-specs");
  });

  test("a repository's tiles take the keys after yours, and Pause keeps p", () => {
    const hidden = withoutPause.slice(0, 6).map((id) => ({ id, hidden: true }));
    const tiles = tilesOf("job", file(hidden), file([SPECS, LINT]));
    const key = (id: string) => tiles.find((one) => one.id === id)?.key;
    expect(key(withoutPause.at(-1)!)).toBe("⌥⇧9");
    expect(key("my-run-specs")).toBe("⌥⇧0");
    expect(key("my-lint")).toBeUndefined();
    expect(key("pause")).toBe("p");
  });

  test("a key of the repository's that one of yours already has is dropped", () => {
    const tiles = tilesOf("deck", file([], [{ id: "map", key: "R" }]), file([], [{ id: "my-ci", kind: "page", name: "CI", text: "https://ci.example", key: "R" }]));
    expect(tiles.find((one) => one.id === "map")?.key).toBe("r");
    expect(tiles.find((one) => one.id === "my-ci")?.key).toBe("⌥⇧5");
  });

  test("a repository with none draws exactly the tiles drawn without one", () => {
    const yours = file([{ id: "pause" }, { id: "my-docs", kind: "page", text: "https://docs.example", key: "D" }, { id: "ask", hidden: true }]);
    expect(tilesOf("job", yours, NOTHING_CHANGED)).toEqual(tilesOf("job", yours));
    expect(tilesOf("deck", yours, NOTHING_CHANGED)).toEqual(tilesOf("deck", yours));
  });

  test("a repository's entry with a built-in's id or one of yours, or with no kind, is not drawn", () => {
    const yours = file([{ id: "my-run-specs", kind: "words", name: "Mine", text: "hi" }]);
    const tiles = tilesOf("job", yours, file([{ id: "ask", name: "Theirs" }, SPECS, { id: "my-x", text: "no kind" }]));
    expect(tiles.filter((one) => one.fromRepository)).toEqual([]);
    expect(tiles.find((one) => one.id === "my-run-specs")?.kind).toBe("words");
  });
});

// ---- A tile's key: a digit, a letter or a chord in the registry's spelling ----------------------------------------------

const press = (key: string, held: { alt?: boolean; shift?: boolean; meta?: boolean; ctrl?: boolean; code?: string } = {}) => ({ key, code: held.code ?? "", altKey: held.alt === true, shiftKey: held.shift === true, metaKey: held.meta === true, ctrlKey: held.ctrl === true });
const keyOf = (id: string, entries: FlightEntry[]) => tilesOf("job", file(entries)).find((one) => one.id === id)?.key;
const problem = (key: string, entries: FlightEntry[] = [], own?: string) => keyProblem(key, tilesOf("job", file(entries)), own);

describe("a tile's key", () => {
  test("an old file's lone capital letter is the plain key, and is drawn as the registry draws a letter", () => {
    expect(keyChord("R")).toEqual({ ctrl: false, alt: false, shift: false, meta: false, key: "r" });
    expect(keyOf("map", [{ id: "map", key: "R" }])).toBe("r");
    expect(keyOf("map", [{ id: "map", key: "5" }])).toBe("5");
  });

  test("a chord in the registry's spelling survives, drawn as the registry draws it", () => {
    expect(keyOf("map", [{ id: "map", key: "⌥R" }])).toBe("⌥R");
    expect(keyOf("map", [{ id: "map", key: "⇧⌘J" }])).toBe("⇧⌘J");
    expect(keyOf("map", [{ id: "map", key: "⌥1" }])).toBe("⌥1");
  });

  test("a key that is not a tile's is read as none, and the tile keeps its place's digit", () => {
    expect(keyOf("ask", [{ id: "ask", key: "Tab" }])).toBe("⌥⇧1");
    expect(keyOf("ask", [{ id: "ask", key: "⇧J" }])).toBe("⌥⇧1");
  });

  test("Pause may be given a key, and then does not answer p", () => {
    expect(keyOf("pause", [{ id: "pause", key: "⌥P" }])).toBe("⌥P");
    expect(keyOf("pause", [])).toBe("p");
  });

  test("a chord presses by the registry's reading: ⌘ takes Control too, Option is read by the key's place", () => {
    expect(keyPressed("⌘R", press("r", { meta: true }))).toBe(true);
    expect(keyPressed("⌘R", press("r", { ctrl: true }))).toBe(true);
    expect(keyPressed("⌃R", press("r", { meta: true }))).toBe(false);
    expect(keyPressed("⌥R", press("®", { alt: true, code: "KeyR" }))).toBe(true);
    expect(keyPressed("⇧⌘J", press("J", { shift: true, meta: true }))).toBe(true);
    expect(keyPressed("⌥R", press("r"))).toBe(false);
    expect(keyPressed("r", press("r"))).toBe(true);
    expect(keyPressed("R", press("r"))).toBe(true);
    expect(keyPressed("r", press("R", { shift: true }))).toBe(false);
    expect(pressesChord({ ctrl: false, alt: true, shift: false, meta: false, key: "1" }, press("¡", { alt: true, code: "Digit1" }))).toBe(true);
  });
});

describe("keyProblem", () => {
  test("a key the Cockpit answers is named for the registry's act", () => {
    expect(problem("m")).toBe(`Used by ${ACTION.dashboard_view!.verb}`);
  });

  test("⌥1 is Quick dispatch's, named by its registry verb", () => {
    expect(problem("⌥1")).toBe(`Used by ${ACTION.dispatch_repository!.verb}`);
  });

  test("the editing keys of a text box are refused, named for their act", () => {
    expect([problem("⌘C"), problem("⌘V"), problem("⌘X"), problem("⌘A"), problem("⌘Z"), problem("⇧⌘Z")]).toEqual(["Used by Copy", "Used by Paste", "Used by Cut", "Used by Select all", "Used by Undo", "Used by Redo"]);
  });

  test("a key another tile holds is named for that tile, by whatever spelling it was kept in", () => {
    const entries: FlightEntry[] = [{ id: "report", key: "R" }, { id: "map", key: "⌥J" }];
    expect(problem("r", entries, "ask")).toBe("Used by Report issue with job");
    expect(problem("⌥J", entries, "ask")).toBe("Used by Map");
    expect(problem("r", entries, "report")).toBeUndefined();
  });

  test("⌃ and ⌘ are one key to a tile, as they are to the registry's acts, so ⌃R is taken where a tile has ⌘R", () => {
    expect(problem("⌃R", [{ id: "report", key: "⌘R" }], "ask")).toBe("Used by Report issue with job");
    expect(problem("⌘R", [{ id: "report", key: "⌃R" }], "ask")).toBe("Used by Report issue with job");
    expect(problem("⌃⌘R", [{ id: "report", key: "⌘R" }], "ask")).toBeUndefined();
  });

  test("Shift with a letter alone is refused, and so is a key with no modifier that is neither a digit nor a letter", () => {
    expect(problem("⇧j")).toBe("⇧ needs ⌘, ⌃ or ⌥ with it");
    expect(problem("⇧1")).toBe("⇧ needs ⌘, ⌃ or ⌥ with it");
    expect(problem("Tab")).toBe("A digit, a letter or a chord");
    expect(problem("/")).toBe("A digit, a letter or a chord");
  });

  test("a free digit, letter or chord, and no key, are fine", () => {
    expect([problem("5"), problem("r"), problem("⌥R"), problem("⇧⌘J"), problem("⌥⇧1"), problem("")]).toEqual([undefined, undefined, undefined, undefined, undefined, undefined]);
  });

  test("p is Pause's while it wears p, and free once Pause has another key", () => {
    expect(problem("p", [], "ask")).toBe("Used by Pause");
    expect(problem("p", [{ id: "pause", key: "⌥P" }], "ask")).toBeUndefined();
    expect(problem("p", [{ id: "pause", key: "p" }], "ask")).toBe("Used by Pause");
  });

  test("Pause may be given p, and a key of its own", () => {
    expect(problem("p", [], "pause")).toBeUndefined();
    expect(problem("⌥P", [], "pause")).toBeUndefined();
  });
});

describe("refusalOf", () => {
  test("a kind on a built-in id is refused, naming the field as Fleet would", () => {
    expect(refusalOf("job", [{ id: "pause" }, { id: "ask", kind: "words" }])).toBe("job[1].kind");
    expect(refusalOf("deck", [{ id: "map", kind: "page" }])).toBe("deck[0].kind");
  });

  test("on_text and off_text are the land tile's alone", () => {
    expect(refusalOf("job", [{ id: "land", on_text: "on", off_text: "off" }])).toBeUndefined();
    expect(refusalOf("job", [{ id: "ask", on_text: "on" }])).toBe("job[0].on_text");
    expect(refusalOf("job", [{ id: "my-x", kind: "words", off_text: "off" }])).toBe("job[0].off_text");
    expect(refusalOf("deck", [{ id: "land", on_text: "on" }])).toBe("deck[0].on_text");
  });

  test("an action of the owner's with a kind, and a built-in with words, are fine", () => {
    expect(refusalOf("job", [{ id: "my-x", kind: "shell", text: "ls" }, { id: "compact", text: "/compact keep the plan" }])).toBeUndefined();
  });
});

describe("the store", () => {
  const read = (controls: FlightControls): FlightControlsAnswer => ({ ok: true, value: { what: "read", controls } });
  const connection = { state: "connected" } as never;

  function fleetOf(answer: () => Promise<FlightControlsAnswer>, save?: (controls: FlightControls) => Promise<FlightControlsAnswer>) {
    const fleet: FlightFleet = {
      state: async () => ({ connection }),
      subscribe: () => () => {},
      getFlightControls: vi.fn(answer),
      saveFlightControls: vi.fn(save ?? (async (controls) => read(controls))),
      getRepositoryFlightControls: vi.fn(async (repository: string): Promise<RepositoryFlightControlsAnswer> => ({ ok: true, value: { repository, place: "this_mac", read: { what: "read", controls: SPECS }, in_repository: { what: "read", controls: { version: 1, job: [], deck: [] } } } })),
      saveRepositoryFlightControls: vi.fn(async (repository: string, controls: FlightControls): Promise<RepositoryFlightControlsAnswer> => ({ ok: true, value: { repository, place: "this_mac", read: { what: "read", controls }, in_repository: { what: "read", controls: { version: 1, job: [], deck: [] } } } })),
    };
    return fleet;
  }
  const SPECS: FlightControls = { version: 1, job: [{ id: "my-run-specs", kind: "shell", name: "Run specs", text: "bin/rspec" }], deck: [] };
  const LH = "/Users/user/widgets";
  const OTHER = "/Users/user/armada";
  const settled = () => new Promise<void>((done) => setTimeout(done, 0));

  test("it reads the file once Fleet is connected and holds what it says", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [{ id: "pause" }], deck: [] }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    await settled();
    expect(fleet.getFlightControls).toHaveBeenCalledTimes(1);
    expect(store.get().job).toEqual([{ id: "pause" }]);
  });

  test("a file that will not read leaves the built-ins and edits still save", async () => {
    const notRead: FlightControlsRead = { what: "not_read", why: "line 3: unknown key" };
    const fleet = fleetOf(async () => ({ ok: true, value: notRead }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    await settled();
    expect(store.get()).toEqual(NOTHING_CHANGED);
    expect(ids("job", store.get())).toEqual(JOB_BUILT_INS.map((one) => one.id));
    await store.save("job", [{ id: "map" }]);
    expect(fleet.saveFlightControls).toHaveBeenCalledWith({ version: 1, job: [{ id: "map" }], deck: [] });
  });

  test("a save sends the whole file, both sets, and is drawn at once", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [{ id: "map" }] }));
    const store = createFlightStore(fleet);
    const drawn = vi.fn();
    store.subscribe(drawn);
    await settled();
    drawn.mockClear();
    const saving = store.save("job", [{ id: "ask", hidden: true }]);
    expect(store.get().job).toEqual([{ id: "ask", hidden: true }]);
    expect(drawn).toHaveBeenCalled();
    await saving;
    expect(fleet.saveFlightControls).toHaveBeenCalledWith({ version: 1, job: [{ id: "ask", hidden: true }], deck: [{ id: "map" }] });
  });

  test("a save Fleet cannot write keeps the change for this window", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [] }), async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    await settled();
    await store.save("job", [{ id: "pause" }]);
    expect(store.get().job).toEqual([{ id: "pause" }]);
  });

  test("a refused save is neither drawn nor sent", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [] }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    await settled();
    await expect(store.save("job", [{ id: "ask", kind: "words" }])).resolves.toEqual({ refused: "job[0].kind" });
    await expect(store.save("job", [{ id: "map", on_text: "x" }])).resolves.toEqual({ refused: "job[0].on_text" });
    expect(fleet.saveFlightControls).not.toHaveBeenCalled();
    expect(store.get()).toEqual(NOTHING_CHANGED);
  });

  test("with no Fleet to reach it keeps edits for the window", async () => {
    const store = createFlightStore(undefined);
    store.subscribe(() => {});
    await store.save("deck", [{ id: "map" }]);
    expect(store.get().deck).toEqual([{ id: "map" }]);
  });

  test("with no repository open no repository is read, and one opened is read by its root", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [] }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(undefined);
    await settled();
    expect(fleet.getRepositoryFlightControls).not.toHaveBeenCalled();
    store.open(LH);
    await settled();
    expect(fleet.getRepositoryFlightControls).toHaveBeenCalledWith(LH);
    expect(store.repository(LH)).toEqual({ what: "read", file: SPECS, place: "this_mac" });
  });

  test("a repository is read again when the open one changes, and never while it stays open", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [] }));
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(LH);
    await settled();
    store.open(LH);
    await settled();
    store.open(OTHER);
    store.open(undefined);
    store.open(LH);
    await settled();
    expect(vi.mocked(fleet.getRepositoryFlightControls).mock.calls).toEqual([[LH], [OTHER], [LH]]);
  });

  test("a repository opened before Fleet is connected is read once it is", async () => {
    let on: ((state: Pick<BridgeState, "connection">) => void) | undefined;
    const fleet: FlightFleet = {
      ...fleetOf(async () => read({ version: 1, job: [], deck: [] })),
      state: async () => ({ connection: { state: "connecting" } }) as never,
      subscribe: (listen) => ((on = listen), () => {}),
    };
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(LH);
    await settled();
    expect(fleet.getRepositoryFlightControls).not.toHaveBeenCalled();
    on?.({ connection: { state: "connected" } } as never);
    await settled();
    expect(fleet.getRepositoryFlightControls).toHaveBeenCalledWith(LH);
  });

  test("a repository file that will not read, or a refusal, is held with why and never as an empty file", async () => {
    const fleet = fleetOf(async () => read({ version: 1, job: [], deck: [] }));
    vi.mocked(fleet.getRepositoryFlightControls)
      .mockResolvedValueOnce({ ok: true, value: { repository: LH, place: "cloud", read: { what: "not_read", why: "The bucket answered 403" }, in_repository: { what: "read", controls: { version: 1, job: [], deck: [] } } } })
      .mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.run_memory_repository_not_served", message: "Fleet does not serve /Users/user/armada", run_id: "", fields: {}, chain: [] } } } as never);
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(LH);
    await settled();
    expect(store.repository(LH)).toEqual({ what: "not_read", why: "The bucket answered 403" });
    store.open(OTHER);
    await settled();
    expect(store.repository(OTHER)).toEqual({ what: "not_read", why: "Fleet does not serve /Users/user/armada" });
  });

  test("a repository not answered yet is unread", async () => {
    const store = createFlightStore(fleetOf(async () => read({ version: 1, job: [], deck: [] })));
    expect(store.repository(LH)).toEqual({ what: "unread" });
  });
});

describe("tilesOf, with the repository's committed tiles", () => {
  const TESTS: FlightEntry = { id: "run-tests", kind: "shell", name: "Run tests", text: "make test" };
  const MINE: FlightEntry = { id: "my-lint", kind: "shell", name: "Lint", text: "bin/lint" };
  const withoutPause = JOB_BUILT_INS.map((one) => one.id).filter((id) => id !== "pause");

  test("they are drawn after yours for the repository, marked committed, and before Pause", () => {
    const tiles = tilesOf("job", NOTHING_CHANGED, file([MINE]), file([TESTS]));
    expect(tiles.map((one) => one.id)).toEqual([...withoutPause, "my-lint", "run-tests", "pause"]);
    expect(tiles.find((one) => one.id === "run-tests")).toMatchObject({ committed: true, fromRepository: false, hides: undefined });
  });

  test("yours of the same id runs in its place, and is marked as hiding it only where the two differ", () => {
    const yours = { ...TESTS, text: "make test-fast" };
    const hiding = tilesOf("job", NOTHING_CHANGED, file([yours]), file([TESTS]));
    expect(hiding.filter((one) => one.id === "run-tests")).toHaveLength(1);
    expect(hiding.find((one) => one.id === "run-tests")).toMatchObject({ committed: false, fromRepository: true, words: "make test-fast", hides: "Run tests" });
    const same = tilesOf("job", NOTHING_CHANGED, file([{ text: "make test", name: "Run tests", kind: "shell", id: "run-tests" }]), file([TESTS]));
    expect(same.find((one) => one.id === "run-tests")?.hides).toBeUndefined();
  });

  test("a tile of yours in Kit hides one too, and a built-in id committed is never drawn", () => {
    const tiles = tilesOf("job", file([{ ...TESTS, name: "Tests" }]), undefined, file([TESTS, { id: "ask", kind: "words", text: "hi" }]));
    expect(tiles.find((one) => one.id === "run-tests")?.hides).toBe("Run tests");
    expect(tiles.filter((one) => one.committed)).toEqual([]);
  });

  test("a committed key that one of yours has is dropped", () => {
    const tiles = tilesOf("job", file([{ id: "map", key: "T" }]), undefined, file([{ ...TESTS, key: "T" }]));
    expect(tiles.find((one) => one.id === "map")?.key).toBe("t");
    expect(tiles.find((one) => one.id === "run-tests")?.key).not.toBe("t");
  });
});

describe("the flight store, with the open Job", () => {
  const ROOT = "/Users/user/widgets";
  const COMMITTED: FlightControls = { version: 1, job: [{ id: "run-tests", kind: "shell", name: "Run tests", text: "make test" }], deck: [] };
  const ASK = { id: "run-tests", command: "make test", command_hash: "h1", approved: false };
  const flightFleet = (): FlightFleet => ({
    state: async () => ({ connection: { state: "connected" } }) as never,
    subscribe: () => () => {},
    getFlightControls: vi.fn(async (): Promise<FlightControlsAnswer> => ({ ok: true, value: { what: "read", controls: { version: 1, job: [], deck: [] } } })),
    saveFlightControls: vi.fn(),
    getRepositoryFlightControls: vi.fn(async (repository: string): Promise<RepositoryFlightControlsAnswer> => ({ ok: true, value: { repository, place: "this_mac", read: { what: "read", controls: { version: 1, job: [], deck: [] } }, in_repository: { what: "read", controls: COMMITTED }, asking: [ASK] } })),
    saveRepositoryFlightControls: vi.fn(),
  });
  const settled = () => new Promise((done) => setTimeout(done, 0));

  test("reads the committed tiles from the open Job's tree, with the ask of each shell tile", async () => {
    const fleet = flightFleet();
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(ROOT, "s1");
    await settled();
    expect(fleet.getRepositoryFlightControls).toHaveBeenCalledWith(ROOT, "s1");
    expect(store.repository(ROOT)).toMatchObject({ what: "read", committed: { job: COMMITTED.job }, asking: [ASK] });
  });

  test("another Job in the same repository reads again", async () => {
    const fleet = flightFleet();
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(ROOT, "s1");
    await settled();
    store.open(ROOT, "s2");
    store.open(ROOT, "s2");
    await settled();
    expect(vi.mocked(fleet.getRepositoryFlightControls).mock.calls).toEqual([[ROOT, "s1"], [ROOT, "s2"]]);
  });

  test("an approval's answer is taken as the repository's read", async () => {
    const fleet = flightFleet();
    const store = createFlightStore(fleet);
    store.subscribe(() => {});
    store.open(ROOT, "s1");
    await settled();
    store.took(ROOT, { ok: true, value: { repository: ROOT, place: "this_mac", read: { what: "read", controls: { version: 1, job: [], deck: [] } }, in_repository: { what: "read", controls: COMMITTED }, asking: [{ ...ASK, approved: true }] } });
    expect(store.repository(ROOT)).toMatchObject({ asking: [{ approved: true }] });
  });
});
