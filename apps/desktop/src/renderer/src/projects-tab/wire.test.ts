// Fleet's projects read into what the tab draws: subtasks nested under their parent, a tracker's bare number named `#7`, and a
// row's state read as done, asked or held by a Job.

import { describe, expect, it } from "vitest";
import type { Projects } from "@armada/protocol";

import { projectsOf, referenceOf } from "./wire";

const WIRE: Projects = {
  manifest_id: "armada",
  projects: [
    {
      id: "p1",
      name: "Pocket launch",
      rows: [
        { kind: "task", id: "t1", title: "Pair a phone", notes: "QR", state: "held", held_by: { session_id: "s1", title: "Wire pairing" }, issues: [{ issue: "2815", title: "Pair code", state: "open" }] },
        { kind: "task", id: "t1a", title: "Draw the code", notes: "", state: "done", parent: "t1" },
        { kind: "task", id: "t1b", title: "Scan it", notes: "", state: "open", parent: "t1" },
        { kind: "issue", id: "i1", issue: "2801", title: "Socket drops", state: "open" },
        { kind: "task", id: "t2", title: "Sign the build", notes: "", state: "asked", held_by: { session_id: "s2" } },
        { kind: "task", id: "t3", title: "Test on a phone", notes: "", state: "open", pulls: [{ number: 2874, title: "Fit the screen", state: "merged" }] },
      ],
    },
  ],
};

describe("a repository's projects as the tab draws them", () => {
  const [project] = projectsOf(WIRE, new Map([["p1", 7]]));

  it("nests each subtask under its parent, in the order Fleet lists them", () => {
    expect(project!.rows.map((row) => row.id)).toEqual(["t1", "i1", "t2", "t3"]);
    expect(project!.rows[0]).toMatchObject({ subtasks: [{ id: "t1a", done: true }, { id: "t1b", done: false }] });
  });

  it("reads a held task's Job, an asked task as asked, and a tracker number as #n", () => {
    expect(project!.rows[0]).toMatchObject({ job: { id: "s1", title: "Wire pairing" }, issues: [{ reference: "#2815", title: "Pair code", state: "open" }], notes: "QR" });
    expect(project!.rows[2]).toMatchObject({ asked: true, job: { id: "s2", title: "Job" } });
    expect(project!.rows[1]).toMatchObject({ kind: "issue", reference: "#2801", state: "open" });
    expect(project!.rows[3]).toMatchObject({ pulls: [{ number: 2874, title: "Fit the screen", state: "merged" }] });
  });

  it("carries the Manifest it was read under and when the project was last used here", () => {
    expect(project).toMatchObject({ id: "p1", manifestId: "armada", name: "Pocket launch", usedAt: 7 });
    expect(projectsOf(WIRE, new Map())[0]!.usedAt).toBe(0);
  });

  it("names a bare number #n and leaves any other reference as it is", () => {
    expect(referenceOf(" 7 ")).toBe("#7");
    expect(referenceOf("ABC-12")).toBe("ABC-12");
  });
});
