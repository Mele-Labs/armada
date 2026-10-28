import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { FleetConnection } from "./connection";
import { parsePicture, Remembered, REMEMBERED_FOR_MS } from "./remembered";

const NOW = 5_000_000;
const session = { id: "s1", state: "live", harness: "claude", cwd: "/x", started_at: "2026-10-28T00:00:00Z" };
const whole = {
  at: NOW - 1000,
  sessions: [session],
  mergeLines: { lines: [] },
  holds: { workflows: [], manifests: [], repositories: [] },
  planUsage: { windows: [] },
  timeline: [{ id: 3, at: "2026-10-28T00:00:00Z", kind: "x", subject: { kind: "main" }, tone: "ok" }],
};

describe("parsePicture", () => {
  it("reads a picture under the cutoff whole", () => {
    const { at: _, ...kept } = whole;
    expect(parsePicture(JSON.stringify(whole), NOW)).toEqual(kept);
  });

  it("is nothing at the cutoff, past it, or dated ahead", () => {
    expect(parsePicture(JSON.stringify({ ...whole, at: NOW - REMEMBERED_FOR_MS + 1 }), NOW)).not.toBeNull();
    expect(parsePicture(JSON.stringify({ ...whole, at: NOW - REMEMBERED_FOR_MS }), NOW)).toBeNull();
    expect(parsePicture(JSON.stringify({ ...whole, at: NOW + 1 }), NOW)).toBeNull();
  });

  it("is nothing for text that does not parse, an unstamped file, or one holding nothing usable", () => {
    expect(parsePicture("{", NOW)).toBeNull();
    expect(parsePicture("[]", NOW)).toBeNull();
    expect(parsePicture(JSON.stringify({ sessions: [session] }), NOW)).toBeNull();
    expect(parsePicture(JSON.stringify({ at: NOW, sessions: "no" }), NOW)).toBeNull();
  });

  it("drops a field that is not the shape it was and keeps the rest", () => {
    const older = { ...whole, sessions: [{ id: 4 }], mergeLines: [], holds: { workflows: [] }, planUsage: { windows: "none" }, timeline: [{ id: "3" }] };
    expect(parsePicture(JSON.stringify(older), NOW)).toBeNull();
    const partly = { ...whole, sessions: [{ id: 4 }], holds: { workflows: [] } };
    expect(Object.keys(parsePicture(JSON.stringify(partly), NOW) ?? {}).sort()).toEqual(["mergeLines", "planUsage", "timeline"]);
  });
});

describe("Remembered", () => {
  let dir: string;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "remembered-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it("loads nothing where there is no file or no place for one", () => {
    expect(new Remembered(join(dir, "none.json"), () => NOW).load()).toBeNull();
    expect(new Remembered(undefined, () => NOW).load()).toBeNull();
  });

  it("loads a fresh file and ignores a stale one", () => {
    const file = join(dir, "p.json");
    writeFileSync(file, JSON.stringify(whole));
    expect(new Remembered(file, () => NOW).load()?.sessions).toEqual([session]);
    expect(new Remembered(file, () => NOW + REMEMBERED_FOR_MS).load()).toBeNull();
  });

  it("flushes what changed, stamped now, and writes nothing for a picture only loaded", () => {
    const file = join(dir, "p.json");
    const remembered = new Remembered(file, () => NOW);
    remembered.flush();
    expect(() => readFileSync(file)).toThrow();
    remembered.keep({ planUsage: { windows: [] } });
    remembered.flush();
    expect(JSON.parse(readFileSync(file, "utf8"))).toEqual({ at: NOW, planUsage: { windows: [] } });
    const next = new Remembered(file, () => NOW + 10);
    expect(next.load()).toEqual({ planUsage: { windows: [] } });
    writeFileSync(file, "kept");
    next.flush();
    expect(readFileSync(file, "utf8")).toBe("kept");
  });
});

describe("a Bridge opened on the picture", () => {
  let dir: string;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "remembered-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  const opening = (pictureFile: string, now: number) =>
    new FleetConnection({ home: undefined, publish: () => {}, publishToWindow: () => {}, windowIds: () => [], now: () => now, pictureFile });

  it("draws the kept Jobs, lines, repositories, plan and timeline before Fleet has answered", async () => {
    const file = join(dir, "p.json");
    writeFileSync(file, JSON.stringify(whole));
    const connection = opening(file, NOW);
    const state = await connection.state();
    expect(state.connection.state).toBe("reading");
    expect(state.drawnFromDisk).toBe(true);
    expect(state.sessions).toEqual({ state: "read", sessions: [session] });
    expect(state.mergeLines).toEqual({ lines: [] });
    expect(state.holds.repositories).toEqual([]);
    expect(await connection.planUsage.get()).toEqual({ ok: true, value: { windows: [] } });
    expect(await connection.timeline.get({})).toMatchObject({ ok: true, value: { events: [{ id: 3 }] } });
  });

  it("opens on nothing where the file is stale or is not a picture", async () => {
    const file = join(dir, "p.json");
    writeFileSync(file, JSON.stringify({ ...whole, at: NOW - REMEMBERED_FOR_MS }));
    expect((await opening(file, NOW).state()).drawnFromDisk).toBe(false);
    writeFileSync(file, "not json");
    const state = await opening(file, NOW).state();
    expect(state.sessions).toEqual({ state: "none" });
    expect(await opening(file, NOW).planUsage.get()).toMatchObject({ ok: false });
  });

  it("writes a reading that arrives, stamped when it was written", () => {
    const file = join(dir, "p.json");
    const connection = opening(file, NOW);
    connection.planUsage.changed({ windows: [] });
    connection.rememberNow();
    expect(JSON.parse(readFileSync(file, "utf8"))).toEqual({ at: NOW, planUsage: { windows: [] } });
  });
});
