// A picture or a video in a Job's Ledger opens in the large view the thread's own opens, as a media reading. Any other file does not.

import { expect, test, vi } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { entriesOf } from "./sessions";

const goes = {
  onGoTo: vi.fn(),
  onOpenLink: vi.fn(),
  onOpenCheck: vi.fn(),
  onOpenLoadout: vi.fn(),
};

const jobWith = (id: string): Session =>
  ({
    id: "job",
    attachments: [
      {
        kind: "artifact",
        form: "file",
        id,
        title: id.slice(id.lastIndexOf("/") + 1),
      },
    ],
    rows: [],
    turn: {
      state: "idle",
    },
  }) as unknown as Session;

function pressed(path: string) {
  const session = jobWith(path);
  const read = vi.fn();
  const rows = entriesOf(
    session,
    goes,
    read,
    vi.fn(),
    [
      session,
    ],
    vi.fn(),
    vi.fn(),
    undefined,
    vi.fn(),
    {
      slots: [],
    },
    undefined,
  );
  rows.find((one) => one.kind === "artifact")!.onOpen();
  return read;
}

test.each([
  "/work/clip.mp4",
  "/work/shot.png",
])("a Ledger row for %s opens as media", (path) => {
  expect(pressed(path)).toHaveBeenCalledExactlyOnceWith({
    kind: "media",
    id: path,
  });
});

test("a Ledger row for a text file is not opened as media", () => {
  expect(pressed("/work/notes.txt")).not.toHaveBeenCalledWith(
    expect.objectContaining({
      kind: "media",
    }),
  );
});
