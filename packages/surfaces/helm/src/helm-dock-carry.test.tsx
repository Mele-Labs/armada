// What Helm's dock hands the window with a message, and how a sent row draws what Fleet kept of it, mounted on
// its own: the files, sketches and tags a message went with are given to `onAsk`; the row is the wire's, with a
// picture's address and the sketch it was drawn from where the window knows them.

import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { HelmContext, HelmThreadItem } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/build/base";
import { HelmDock } from "./HelmDock";
import type { HelmKept } from "./HelmDock";
import type { ThoughtCarry, ThoughtRouting } from "./thought-routing";

afterEach(unmount);

const CONTEXT: HelmContext = { screen: "board" };
const TAG = { kind: "issue", id: "214", title: "#214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/214" } as const;
const PIXEL = "data:image/gif;base64,R0lGODlhAQABAIAAAAUEBAAAACwAAAAAAQABAAACAkQBADs=";

const box = () => page.getByRole("textbox", { name: "Ask Helm" });

function Dock({
  items = [],
  onAsk,
  kept,
  routing,
  box: parts,
}: {
  items?: HelmThreadItem[];
  onAsk: (text: string, context: HelmContext, carry?: ThoughtCarry) => void;
  kept?: HelmKept;
  routing?: ThoughtRouting;
  box?: Parameters<typeof HelmDock>[0]["box"];
}) {
  const [held] = useState(items);
  return (
    <HelmDock
      helm={{ state: "open", manifestId: MANIFEST_ID, replying: false, skipped: 0, missed: 0, items: held, facts: {} }}
      repositories={[repository()]}
      jobs={[]}
      workflows={[]}
      live
      context={CONTEXT}
      onAsk={onAsk}
      {...(kept === undefined ? {} : { kept })}
      {...(routing === undefined ? {} : { routing })}
      {...(parts === undefined ? {} : { box: parts })}
      onSwitch={() => {}}
      onReadRecord={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onCopied={() => {}}
      onSaid={() => {}}
      onApprove={async () => ({ ok: true })}
    />
  );
}

/** A thought router that reads every message as a question for Helm. */
function reading() {
  const read = vi.fn<ThoughtRouting["read"]>(async () => ({ kind: "question" }));
  const routing: ThoughtRouting = { read, send: async () => true, open: () => {}, startNew: () => {} };
  return { read, routing };
}

async function pasteAPicture(): Promise<void> {
  await expect.element(box()).toBeVisible();
  const clipboard = new DataTransfer();
  clipboard.items.add(new File(["png"], "Screenshot.png", { type: "image/png" }));
  box().element().dispatchEvent(new ClipboardEvent("paste", { clipboardData: clipboard, bubbles: true, cancelable: true }));
  await expect.element(page.getByText("Screenshot.png")).toBeVisible();
}

test("words alone are asked as they always were: no third argument", async () => {
  const onAsk = vi.fn();
  mount(<Dock onAsk={onAsk} />);
  await box().fill("why did 12 stop");
  await userEvent.keyboard("{Enter}");
  expect(onAsk).toHaveBeenCalledExactlyOnceWith("why did 12 stop", CONTEXT);
});

test("a message's files and sketches go to the window with its words, and the box is given nothing back", async () => {
  const onAsk = vi.fn<(text: string, context: HelmContext, carry?: ThoughtCarry) => void>();
  const removed = vi.fn();
  mount(<Dock onAsk={onAsk} box={{ drawn: [{ id: "k1", title: "pin the clock" }], onRemoveDrawn: removed }} />);

  await pasteAPicture();
  await box().fill("do these explain it?");
  await userEvent.keyboard("{Enter}");

  const [text, context, carry] = onAsk.mock.calls[0]!;
  expect([text, context]).toEqual(["do these explain it?", CONTEXT]);
  expect(carry?.files.map((one) => [one.name, one.file?.type])).toEqual([["Screenshot.png", "image/png"]]);
  expect(carry?.sketches).toEqual([{ id: "k1", title: "pin the clock" }]);
  expect(carry?.tags).toEqual([]);
  // The sketch went with this message and does not wait for the next.
  expect(removed).toHaveBeenCalledWith("k1");
});

test("a message of only a picture goes straight to Helm, and its words are not read for where they belong", async () => {
  const { read, routing } = reading();
  const onAsk = vi.fn<(text: string, context: HelmContext, carry?: ThoughtCarry) => void>();
  mount(<Dock onAsk={onAsk} routing={routing} />);

  await pasteAPicture();
  await page.getByRole("button", { name: "Send" }).click();

  await vi.waitFor(() => expect(onAsk).toHaveBeenCalledTimes(1));
  expect(read).not.toHaveBeenCalled();
  expect(onAsk.mock.calls[0]![0]).toBe("");
  expect(onAsk.mock.calls[0]![2]?.files).toHaveLength(1);
});

test("words with a picture are read first, and a question carries the picture to Helm", async () => {
  const { read, routing } = reading();
  const onAsk = vi.fn<(text: string, context: HelmContext, carry?: ThoughtCarry) => void>();
  mount(<Dock onAsk={onAsk} routing={routing} />);

  await pasteAPicture();
  await box().fill("why is this red?");
  await userEvent.keyboard("{Enter}");

  await vi.waitFor(() => expect(onAsk).toHaveBeenCalledTimes(1));
  expect(read).toHaveBeenCalledExactlyOnceWith("why is this red?");
  expect(onAsk.mock.calls[0]![2]?.files.map((one) => one.name)).toEqual(["Screenshot.png"]);
});

test("a blank message with nothing carried sends nothing", async () => {
  const { read, routing } = reading();
  const onAsk = vi.fn();
  mount(<Dock onAsk={onAsk} routing={routing} />);
  await box().fill("   ");
  await userEvent.keyboard("{Enter}");
  expect(onAsk).not.toHaveBeenCalled();
  expect(read).not.toHaveBeenCalled();
});

const ROW: HelmThreadItem = {
  kind: "asked",
  id: "0",
  ts: "2026-10-12T10:00:00.000Z",
  text: "see @#214 Sessions list drops a quiet Session",
  files: [
    { id: "f1", name: "Screenshot.png", media_type: "image/png" },
    { id: "f2", name: "pin the clock.png", media_type: "image/png" },
    { id: "f3", name: "clock.log", media_type: "text/plain" },
  ],
  tags: [TAG],
};

test("a sent row draws what Fleet kept of it: a picture once it is read, a sketch where this window drew it, a file by its name, a tag in the line", async () => {
  const picture = vi.fn((id: string) => (id === "f1" ? PIXEL : undefined));
  const kept: HelmKept = { picture, sketch: (name) => (name === "pin the clock.png" ? { id: "k1", title: "pin the clock" } : undefined) };
  mount(<Dock items={[ROW]} onAsk={() => {}} kept={kept} />);

  await expect.element(page.getByRole("img", { name: /Screenshot\.png/ })).toBeVisible();
  await expect.element(page.getByText("pin the clock", { exact: true })).toBeVisible();
  expect(page.getByText("pin the clock.png").query()).toBeNull();
  await expect.element(page.getByText("clock.log")).toBeVisible();
  await expect.element(page.getByText("#214 Sessions list drops a quiet Session")).toBeVisible();
  // Only a picture is read, and a text file never is.
  expect(picture).not.toHaveBeenCalledWith("f3");
});

test("a sent row from before a reload draws a sketch as the picture it went as", async () => {
  mount(<Dock items={[ROW]} onAsk={() => {}} kept={{ picture: () => PIXEL, sketch: () => undefined }} />);
  await expect.element(page.getByRole("img", { name: /pin the clock\.png/ })).toBeVisible();
});

test("a sent row with nothing to read from draws the names", async () => {
  mount(<Dock items={[ROW]} onAsk={() => {}} />);
  await expect.element(page.getByText("Screenshot.png")).toBeVisible();
  await expect.element(page.getByText("clock.log")).toBeVisible();
});

const SENT: HelmThreadItem = {
  kind: "sent",
  id: "1",
  ts: "2026-10-12T10:00:00.000Z",
  text: "the sessions page should say waiting",
  target: { kind: "session", id: "01SESSION", title: "Sessions list views", manifest_id: MANIFEST_ID, slot: 3 },
  files: [
    { id: "f1", name: "Screenshot.png", media_type: "image/png" },
    { id: "f2", name: "pin the clock.png", media_type: "image/png" },
  ],
  tags: [TAG],
};

test("words sent on to a Session are drawn with what went with them, from the row Fleet kept and with nothing held here", async () => {
  const kept: HelmKept = { picture: (id) => (id === "f1" ? PIXEL : undefined), sketch: (name) => (name === "pin the clock.png" ? { id: "k1", title: "pin the clock" } : undefined) };
  mount(<Dock items={[SENT]} onAsk={() => {}} kept={kept} />);

  await expect.element(page.getByRole("button", { name: "Sent to Session Sessions list views" })).toBeVisible();
  await expect.element(page.getByRole("img", { name: /Screenshot\.png/ })).toBeVisible();
  await expect.element(page.getByText("pin the clock", { exact: true })).toBeVisible();
  await expect.element(page.getByText("#214 Sessions list drops a quiet Session")).toBeVisible();
});

test("a sent row with no files or tags draws none", async () => {
  const { files: _files, tags: _tags, ...plain } = SENT as Extract<HelmThreadItem, { kind: "sent" }>;
  mount(<Dock items={[plain]} onAsk={() => {}} kept={{ picture: () => PIXEL, sketch: () => undefined }} />);
  await expect.element(page.getByRole("button", { name: "Sent to Session Sessions list views" })).toBeVisible();
  expect(page.getByRole("img", { name: /Screenshot/ }).query()).toBeNull();
});
