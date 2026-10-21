import { beforeEach, expect, test, vi } from "vitest";
import type { Outcome, RunMemory, RunMemoryPlace } from "@armada/protocol";

import { RunBook, cardOn, forgetFirsts, type RunBookApi } from "./run-book";

beforeEach(forgetFirsts);

test("the first Job on a repository carries the card until its first send, and no later Job does", () => {
  expect(cardOn("/r", "job-1", false)).toBe(true);
  expect(cardOn("/r", "job-1", false)).toBe(true);
  expect(cardOn("/r", "job-2", false)).toBe(false);
  expect(cardOn("/r", "job-1", true)).toBe(false);
  expect(cardOn("/r", "job-2", false)).toBe(false);
});

test("each repository has a first Job of its own", () => {
  expect(cardOn("/a", "job-1", false)).toBe(true);
  expect(cardOn("/b", "job-2", false)).toBe(true);
  expect(cardOn("/a", "job-1", false)).toBe(true);
});

test("a repository first met through a Job already sent in has no card, now or for the next Job", () => {
  expect(cardOn("/r", "old", true)).toBe(false);
  expect(cardOn("/r", "new", false)).toBe(false);
});

const refused = (message: string): Outcome => ({ ok: false, why: "refused", error: { code: "fleet.run_place_not_yet", message, run_id: "r", fields: {}, chain: [] } });
const fact = { workspace: ".", kind: "tests", command: "pnpm test", succeeded: true, last_ran: "", source: "jobs", state: "kept", where: "this_mac" } as const;
const memory = (place: RunMemoryPlace["place"], runs = [fact]): RunMemory => ({ repository: "/r", place, workspaces: [{ workspace: ".", runs: [...runs] }] });

function api(overrides: Partial<RunBookApi> = {}): RunBookApi {
  return {
    getRunMemory: async () => ({ ok: true, value: memory("this_mac") }),
    getRunMemoryPlace: async () => ({ ok: true, value: { repository: "/r", place: "this_mac" } }),
    setRunMemoryPlace: async (body) => ({ ok: true, value: body }),
    dropRunFact: async () => ({ ok: true, value: memory("this_mac", []) }),
    ...overrides,
  };
}

test("a place Fleet refuses leaves the old place, and answers Fleet's words", async () => {
  const set = vi.fn(async () => ({ ok: false, outcome: refused("Keeping runs in the cloud is not built yet") }) as const);
  const book = new RunBook(api({ setRunMemoryPlace: set }));
  await book.read("/r");
  expect(book.placeOf("/r")).toEqual({ id: "mac" });
  expect(await book.place("/r", { id: "cloud", detail: "s3://bucket/runs" })).toBe("Keeping runs in the cloud is not built yet");
  expect(set).toHaveBeenCalledWith({ repository: "/r", place: "cloud", cloud: { bucket: "s3://bucket/runs" } });
  expect(book.placeOf("/r")).toEqual({ id: "mac" });
});

test("a place Fleet takes becomes the place, and the runs are read again under it", async () => {
  let kept: RunMemoryPlace["place"] = "this_mac";
  const read = vi.fn(async () => ({ ok: true, value: memory(kept) }) as const);
  const set = vi.fn(async (body: RunMemoryPlace) => {
    kept = body.place;
    return { ok: true, value: body } as const;
  });
  const book = new RunBook(api({ getRunMemory: read, setRunMemoryPlace: set }));
  await book.read("/r");
  expect(await book.place("/r", { id: "repository" })).toBeUndefined();
  expect(book.placeOf("/r")).toEqual({ id: "repository" });
  expect(read).toHaveBeenCalledTimes(2);
});

test("dropping a line asks Fleet to forget that command, and the layer shows what Fleet answered", async () => {
  const drop = vi.fn(async () => ({ ok: true, value: memory("this_mac", []) }) as const);
  const book = new RunBook(api({ dropRunFact: drop }));
  await book.read("/r");
  const [row] = book.runsOf("/r", "my-app")!.rows;
  expect(await book.drop("/r", row!.id)).toBeUndefined();
  expect(drop).toHaveBeenCalledWith({ repository: "/r", workspace: ".", kind: "tests", command: "pnpm test" });
  expect(book.runsOf("/r", "my-app")!.rows).toEqual([]);
});

test("a run memory Fleet would not answer is the layer's words, with the place it has", async () => {
  const book = new RunBook(api({ getRunMemory: async () => ({ ok: false, outcome: refused("Fleet could not read the runs") }) }));
  await book.read("/r");
  expect(book.runsOf("/r", "my-app")).toMatchObject({ rows: [], said: "Fleet could not read the runs" });
});
