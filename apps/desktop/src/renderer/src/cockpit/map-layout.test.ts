import { describe, expect, test } from "vitest";

import { nearest, skyOf } from "./map-layout";

const field = { width: 900, height: 600 };

type Made = Parameters<typeof skyOf>[0][number];

/** A Job tile: only what the map reads of an item. */
const job = (id: string, manifest: string, extra: Record<string, unknown> = {}) =>
  ({ key: id, title: id, hue: "running", kind: "Job", job: { id, owner_manifest_id: manifest, ...extra } }) as unknown as Made;

const session = (id: string) => ({ key: `session:${id}`, title: id, hue: "running", kind: "Session", session: true }) as unknown as Made;

const sky = (items: Made[], sessions: Parameters<typeof skyOf>[1] = [], views: Parameters<typeof skyOf>[2] = []) =>
  skyOf(items, sessions, views, (root) => (root === "/armada" ? "armada" : undefined), (manifest) => manifest, field);

describe("the map", () => {
  test("is the same on every render, with nothing settling", () => {
    const items = [job("a", "armada"), job("b", "armada"), job("c", "pocket"), session("s1")];
    expect(sky(items)).toEqual(sky(items));
    expect(sky([...items].reverse()).stars.map((star) => [star.key, star.x, star.y])).toEqual(sky(items).stars.map((star) => [star.key, star.x, star.y]));
  });

  test("clusters Jobs by repository, one ring each", () => {
    const made = sky([job("a", "armada"), job("b", "armada"), job("c", "pocket")]);
    expect(made.clusters.map((one) => one.id).sort()).toEqual(["armada", "pocket"]);
    expect(made.stars.find((star) => star.key === "c")?.cluster).toBe("pocket");
  });

  test("keeps every star inside the field it was given", () => {
    const made = sky(Array.from({ length: 14 }, (_, at) => job(`j${at}`, at % 2 === 0 ? "armada" : "pocket")));
    for (const star of made.stars) {
      expect(star.x).toBeGreaterThan(0);
      expect(star.x).toBeLessThan(field.width);
      expect(star.y).toBeGreaterThan(0);
      expect(star.y).toBeLessThan(field.height);
    }
  });

  test("joins what the data connects: a parent to its child, a Job to what it waits on, a Session to its Job", () => {
    const items = [job("parent", "armada"), job("child", "armada", { dispatched_by: "parent" }), job("later", "armada", { waits_on: ["parent"] }), session("s1")];
    const held = [{ id: "s1", attachments: [{ kind: "job", id: "parent" }], rows: [] }] as unknown as Parameters<typeof skyOf>[1];
    const links = sky(items, held).links;
    expect(links).toContainEqual({ from: "parent", to: "child", kind: "dispatched" });
    expect(links).toContainEqual({ from: "later", to: "parent", kind: "waits" });
    expect(links).toContainEqual({ from: "session:s1", to: "parent", kind: "works" });
  });

  test("draws no line to a Job that is not on the map", () => {
    expect(sky([job("child", "armada", { dispatched_by: "gone", waits_on: ["gone"] })]).links).toEqual([]);
  });

  test("puts a Session with the Job it holds, and one with none on its own", () => {
    const held = [{ id: "s1", attachments: [{ kind: "job", id: "a" }], rows: [] }] as unknown as Parameters<typeof skyOf>[1];
    const made = sky([job("a", "pocket"), session("s1"), session("s2")], held);
    expect(made.stars.find((star) => star.key === "session:s1")?.cluster).toBe("pocket");
    expect(made.stars.find((star) => star.key === "session:s2")?.cluster).toBe("~");
  });

  test("stands the merge line beside its repository and joins the Job whose branch is queued", () => {
    const views = [{ root: "/armada", line: [{ branch: "fleet/x", state: "gating" }, { branch: "fleet/y", state: "waiting" }] }] as unknown as Parameters<typeof skyOf>[2];
    const made = sky([job("a", "armada", { branch: "fleet/x" })], [], views);
    expect(made.landmarks).toHaveLength(1);
    expect(made.landmarks[0]!.branches.map((one) => one.key)).toEqual(["fleet/x", "fleet/y"]);
    expect(made.links).toContainEqual({ from: "a", to: "main:/armada", kind: "line" });
  });

  test("moves the cursor to the star nearest in the direction pressed", () => {
    const made = sky([job("a", "armada"), job("b", "armada"), job("c", "armada")]);
    const [first] = made.stars;
    const right = nearest(made.stars, first!.key, "ArrowRight");
    if (right !== undefined) expect(made.stars.find((star) => star.key === right)!.x).toBeGreaterThan(first!.x);
    expect(nearest(made.stars, "nobody", "ArrowRight")).toBeUndefined();
  });
});
