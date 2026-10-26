// The part of a thread a fork shows of the Session it came from: up to the message it was cut at.

import { describe, expect, it } from "vitest";

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { threadRowsFrom, threadRowsOf, upTo } from "./session-thread-rows";

const rows = ["a", "b", "c", "d"].map((id) => ({ id }));

describe("the old Session's rows a fork shows", () => {
  it("ends at the message the fork was cut at, that message included", () => {
    expect(upTo(rows, "b").map((one) => one.id)).toEqual(["a", "b"]);
  });

  it("is the whole thread for a fork of the whole of an ended Session", () => {
    expect(upTo(rows, undefined)).toBe(rows);
  });

  it("shows nothing where the cut cannot be found, and never what came after it", () => {
    expect(upTo(rows, "gone")).toEqual([]);
    expect(upTo([], "b")).toEqual([]);
  });
});

const scene = (title: string) => ({ nodes: [{ id: "a", kind: "box" as const, x: 0, y: 0, title }], edges: [] });
const call = (id: string, args: Record<string, unknown>, tool = "mcp__armada-fleet__show_sketch"): Session["rows"][number] => ({ id, at: "14:00:00", kind: "tool", text: `${tool} ${JSON.stringify(args)}` });
const drawn = (title: string, against?: { title: string; drawing: ReturnType<typeof scene> }): SessionAttachment => ({
  kind: "sketch",
  id: `sketch:${title}`,
  title,
  by: "agent",
  drawing: scene(title),
  ...(against === undefined ? {} : { against }),
});
type Card = Extract<ReturnType<typeof threadRowsFrom>[number], { kind: "sketch" }>;
const cards = (rows: ReturnType<typeof threadRowsFrom>) => rows.filter((one): one is Card => one.kind === "sketch");

describe("a show_sketch call in a Session's thread", () => {
  it("is a card where the call was, with the scene the call carried", () => {
    const rows = threadRowsFrom([call("t1", { title: "Flow", scene: scene("one") })], [drawn("Flow")]);
    expect(rows).toEqual([{ id: "t1", at: "14:00:00", kind: "sketch", sketch: "sketch:Flow", title: "Flow", drawing: scene("one") }]);
  });

  it("keeps the order of the thread around it", () => {
    const rows = threadRowsFrom(
      [
        { id: "m1", at: "14:00:00", kind: "message", from: { kind: "you" }, text: "draw it" },
        call("t1", { title: "Flow", scene: scene("one") }),
        { id: "m2", at: "14:00:01", kind: "message", from: { kind: "agent" }, text: "Drawn." },
      ],
      [drawn("Flow")],
    );
    expect(rows.map((one) => one.kind)).toEqual(["message", "sketch", "message"]);
  });

  it("is no card when Fleet refused it, which left nothing on the ledger", () => {
    const rows = threadRowsFrom([call("t1", { title: "Flow", scene: scene("one") })], []);
    expect(rows.map((one) => one.kind)).toEqual(["tool"]);
  });

  it("is no card when its scene would not draw", () => {
    const rows = threadRowsFrom([call("t1", { title: "Flow", scene: { nodes: "no", edges: [] } })], [drawn("Flow")]);
    expect(rows.map((one) => one.kind)).toEqual(["tool"]);
  });

  it("leaves a call of another tool as the tool row it was", () => {
    const rows = threadRowsFrom([call("t1", { title: "Flow", scene: scene("one") }, "mcp__armada-fleet__show_window")], [drawn("Flow")]);
    expect(rows.map((one) => one.kind)).toEqual(["tool"]);
  });

  it("marks an earlier card replaced where a later one has its title, and no other", () => {
    const rows = threadRowsFrom(
      [call("t1", { title: "Flow", scene: scene("one") }), call("t2", { title: "Other", scene: scene("x") }), call("t3", { title: "Flow", scene: scene("two") })],
      [drawn("Flow"), drawn("Other")],
    );
    expect(cards(rows).map((one) => [one.id, one.replaced === true])).toEqual([
      ["t1", true],
      ["t2", false],
      ["t3", false],
    ]);
    // Each card keeps what it drew then, so the replaced one is not drawn as its successor.
    expect(cards(rows)[0]!.drawing).toEqual(scene("one"));
  });

  it("is drawn against the latest earlier card with the title it names", () => {
    const rows = threadRowsFrom(
      [
        call("t1", { title: "Before", scene: scene("old") }),
        call("t2", { title: "Before", scene: scene("newer") }),
        call("t3", { title: "After", scene: scene("after"), against: "Before" }),
      ],
      [drawn("Before"), drawn("After")],
    );
    const [, , after] = cards(rows);
    expect(after).toMatchObject({ against: "Before", againstScene: scene("newer") });
  });

  it("falls back to the earlier scene the ledger holds where no earlier card has the title", () => {
    const rows = threadRowsFrom([call("t1", { title: "After", scene: scene("after"), against: "Before" })], [drawn("After", { title: "Before", drawing: scene("kept") })]);
    expect(cards(rows)[0]).toMatchObject({ againstScene: scene("kept") });
  });

  it("is drawn plain where nothing holds what it names", () => {
    const rows = threadRowsFrom([call("t1", { title: "After", scene: scene("after"), against: "Gone" })], [drawn("After", { title: "Other", drawing: scene("kept") })]);
    expect(cards(rows)[0]).not.toHaveProperty("againstScene");
  });

  it("is read off a Session's own ledger", () => {
    const session: Session = { id: "s", turn: { state: "idle" }, rows: [call("t1", { title: "Flow", scene: scene("one") })], attachments: [drawn("Flow")] };
    expect(cards(threadRowsOf(session))).toHaveLength(1);
  });
});

describe("a show_file call as the transcript names it", () => {
  const shown = (path: string): SessionAttachment => ({ kind: "artifact", form: "file", id: path, title: path.split("/").pop()!, shown: "2026-10-19T14:00:00.000Z", by: "agent" });
  const tool = (text: string): Session["rows"][number] => ({ id: "t1", at: "14:00:00", kind: "tool", text });

  it("is a file row opening the ledger's own path, whether the agent gave it absolute or relative", () => {
    const ledger = [shown("/work/docs/notes.md")];
    for (const text of ["mcp__armada-fleet__show_file /work/docs/notes.md", "mcp__armada-fleet__show_file docs/notes.md", "show_file ./docs/notes.md"]) {
      expect(threadRowsFrom([tool(text)], ledger)).toMatchObject([{ kind: "file", path: "/work/docs/notes.md", name: "notes.md" }]);
    }
  });

  it("stays the tool row it was where the ledger holds no such file", () => {
    expect(threadRowsFrom([tool("mcp__armada-fleet__show_file docs/gone.md")], [shown("/work/docs/notes.md")])).toMatchObject([{ kind: "tool" }]);
  });
});

describe("a draft in a Session's thread", () => {
  it("is drawn as the row it came as, so the card has its title, text, the text it replaces and how it was settled", () => {
    const row = { id: "d", at: "14:00:00", kind: "draft", title: "Description", text: "A Job waits.", before: "Adds a queue.", settled: { how: "fixed", words: "A Job waits for a slot.", at: "14:01:00" } } as const;
    expect(threadRowsFrom([row])).toEqual([row]);
  });
});
