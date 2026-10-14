// What Helm's message box puts on the wire and takes off it, read as the TypeScript mirror types it: the
// exact JSON each shape comes as with optional fields absent rather than null, and the rows of the Helm
// Session's thread drawn as a Session's are. **Held here and not in `packages/protocol`**, which has no
// test runner (`thought-wire.test.ts`).

import { describe, expect, it } from "vitest";

import type { AskHelm, IssueRead, SessionRow, SessionTag } from "@armada/protocol";
import { rowsOfThread } from "@armada/screens/src/sessions-wire";

/** What Fleet's JSON decodes to, typed as the mirror says. */
const decoded = <T>(json: string): T => JSON.parse(json) as T;

const ISSUE: SessionTag = { kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" };
const PULL: SessionTag = { kind: "pull_request", id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" };
const FILE = { id: "file-1", name: "Screenshot.png", media_type: "image/png" };

describe("ask_helm", () => {
  it("asks with the words alone when nothing is carried, the fields left out and never null", () => {
    const body: AskHelm = { text: "why did 12 stop", context: { screen: "board" } };
    expect(JSON.stringify(body)).toBe('{"text":"why did 12 stop","context":{"screen":"board"}}');
  });

  it("names the model and effort for the message that starts the Session, and leaves Auto out", () => {
    const body: AskHelm = { text: "why", model: "sonnet", effort: "high" };
    expect(JSON.stringify(body)).toBe('{"text":"why","model":"sonnet","effort":"high"}');
    const auto: AskHelm = { text: "why", effort: "low" };
    expect(JSON.stringify(auto)).toBe('{"text":"why","effort":"low"}');
  });

  it("carries uploads and mentions beside the words, and the words may be blank", () => {
    const body: AskHelm = {
      text: "",
      attachments: [{ name: "Screenshot.png", media_type: "image/png", data: "iVBOR" }],
      mentions: [ISSUE],
    };
    expect(decoded<AskHelm>(JSON.stringify(body))).toEqual(body);
    expect(JSON.stringify(body)).toContain('"kind":"issue"');
    expect(JSON.stringify(body)).toContain('"url":"https://forge.invalid/issues/2214"');
  });
});

describe("get_issue", () => {
  it("answers one issue with where it stands", () => {
    const read = decoded<IssueRead>('{"number":61,"title":"Doctor reports a stale pid as running","url":"https://forge.invalid/issues/61","state":"closed"}');
    expect(read).toEqual({ number: 61, title: "Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61", state: "closed" });
  });
});

describe("the Helm Session's thread", () => {
  const TARGET = { kind: "session", id: "01S", title: "Sessions list views", manifest_id: "01M1", slot: 3 } as const;
  const repository = (id: string | undefined) => (id === "01M1" ? "armada" : (id ?? ""));
  const SENT: SessionRow = { kind: "sent", id: "r3", at: "2026-10-12T10:00:00.000Z", text: "the words", target: TARGET, files: [FILE], tags: [ISSUE] };

  it("draws the files and tags a person's message carried, as a Session's row does", () => {
    const message: SessionRow = { kind: "message", id: "r1", at: "2026-10-12T10:00:00.000Z", from: { kind: "you" }, text: "see this", files: [FILE], tags: [ISSUE, PULL] };
    const [row] = rowsOfThread("01H", [message], () => "blob:shot");
    expect(row).toMatchObject({ kind: "message", text: "see this", files: [{ id: "file-1", name: "Screenshot.png", src: "blob:shot" }] });
    expect(row && "tags" in row && row.tags?.map((tag) => tag.id)).toEqual(["2214", "2219"]);
  });

  it("draws words sent on to other work as a row of their own, with the repository named as the picker names it", () => {
    const [row] = rowsOfThread("01H", [SENT], () => undefined, repository);
    expect(row).toMatchObject({
      kind: "sent",
      text: "the words",
      target: { kind: "session", id: "01S", title: "Sessions list views", manifest_id: "01M1", slot: 3, repository: "armada" },
      files: [{ id: "file-1", name: "Screenshot.png" }],
    });
  });

  it("leaves the fields off a thought that went with nothing", () => {
    const bare: SessionRow = { kind: "sent", id: "r4", at: "2026-10-12T10:00:00.000Z", text: "x", target: TARGET };
    const [row] = rowsOfThread("01H", [bare], () => undefined, repository);
    expect(row && "files" in row).toBe(false);
    expect(row && "tags" in row).toBe(false);
  });
});
