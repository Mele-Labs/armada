// What Done lists as made: a walk shown again stands once, where it was shown last, and the pictures a test run or a look in a browser leave
// behind are scratch, not things made for him.

import { describe, expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { madeOf } from "./ledger";

const walk = (id: string, title: string) => ({ id, at: "2026-10-27T10:00:00Z", kind: "window", title, url: `http://127.0.0.1/?walk=${id}`, ended: "2026-10-27T10:05:00Z" });
const image = (path: string) => ({ kind: "artifact", form: "image", id: path, title: path.split("/").pop() });

const titles = (one: Session) =>
  madeOf(one, [], { landing: "pull_request", mods: 0, onOpenSketch: () => undefined }).flatMap((made) => (made.shape === "row" ? [made.row.title] : []));

test("a walk shown twice stands once, where it was shown last", () => {
  const one = { id: "j", attachments: [], rows: [walk("a", "First look"), walk("b", "Query picker"), walk("c", "First look")] } as unknown as Session;
  expect(titles(one)).toEqual(["Query picker", "First look"]);
});

test("a test run's screenshots and a browser check's pictures are not made", () => {
  const one = {
    id: "j",
    rows: [],
    attachments: [
      image("docs/images/issues-tab.png"),
      image("apps/desktop/src/renderer/src/mock/__screenshots__/walks-1of4.test.tsx/the-walk-plays-1.png"),
      image(".playwright-mcp/check.png"),
      image("/private/tmp/shot.png"),
    ],
  } as unknown as Session;
  expect(titles(one)).toEqual(["issues-tab.png"]);
});

describe("a Job's document in Done", () => {
  const PAGE = "/work/.armada/documents/script.html";
  const job = (change: Record<string, unknown>) =>
    ({ id: "j", rows: [], attachments: [{ kind: "artifact", form: "document", id: PAGE, title: "Video script", revisions: ["/s/r1.html"], asked: true, revision: "1", ...change }] }) as unknown as Session;

  test("is not made while its newest revision waits on him", () => {
    expect(titles(job({}))).not.toContain("Video script");
  });

  test("is made once he approved it", () => {
    expect(titles(job({ settled: { how: "approved", at: "2026-10-30T10:05:00Z" } }))).toContain("Video script");
  });

  test("is made where Fleet never asked", () => {
    expect(titles(job({ asked: undefined, revision: undefined }))).toContain("Video script");
  });
});
