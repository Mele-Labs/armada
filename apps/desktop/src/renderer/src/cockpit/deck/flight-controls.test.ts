// The flight controls as the owner arranged them: the order tiles are drawn in, which are hidden, which kinds each set takes, what Bridge
// refuses before a save, and the store that reads them from Fleet and saves them to it. Node: no window, a fake Fleet.

import type { FlightControls, FlightControlsRead } from "@armada/protocol";
import { describe, expect, test, vi } from "vitest";

import type { FlightControlsAnswer } from "../../../../shared/api/flight-controls";
import { DECK_BUILT_INS, JOB_BUILT_INS, NOTHING_CHANGED, createFlightStore, refusalOf, tilesOf, type FlightEntry, type FlightFile, type FlightFleet } from "./flight-controls";

const file = (job: FlightEntry[] = [], deck: FlightEntry[] = []): FlightFile => ({ version: 1, job, deck });
const ids = (set: "job" | "deck", of: FlightFile) => tilesOf(set, of).map((one) => one.id);

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
    expect(tiles[1]).toMatchObject({ id: "land", key: "1" });
  });

  test("keys follow position, an explicit key wins, and the tile whose digit it took has none", () => {
    const tiles = tilesOf("job", file([{ id: "ask" }, { id: "land", key: "1" }, { id: "compact" }]));
    expect(tiles.map((one) => one.key).slice(0, 3)).toEqual([undefined, "1", "3"]);
  });

  test("Pause answers P however it is placed", () => {
    expect(tilesOf("job", file([{ id: "pause" }])).find((one) => one.id === "pause")?.key).toBe("P");
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
    };
    return fleet;
  }
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
});
