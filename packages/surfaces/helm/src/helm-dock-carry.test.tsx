// What Helm's dock hands the window with a message, mounted on its own: the files, sketches and tags a message
// went with are given to `onAsk`. How the thread draws them is a Session's (`helm-box-wire.test.ts`).

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { HelmContext } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/build/base";
import { HelmDock } from "./HelmDock";
import type { ThoughtCarry, ThoughtRouting } from "./thought-routing";

afterEach(unmount);

const CONTEXT: HelmContext = { screen: "board" };

const box = () => page.getByRole("textbox", { name: "Ask Helm" });

function Dock({
  onAsk,
  routing,
  box: parts,
}: {
  onAsk: (text: string, context: HelmContext, carry?: ThoughtCarry) => void;
  routing?: ThoughtRouting;
  box?: Parameters<typeof HelmDock>[0]["box"];
}) {
  return (
    <HelmDock
      helm={{ state: "pointed", manifestId: MANIFEST_ID }}
      repositories={[repository()]}
      jobs={[]}
      live
      context={CONTEXT}
      onAsk={onAsk}
      {...(routing === undefined ? {} : { routing })}
      {...(parts === undefined ? {} : { box: parts })}
      onSwitch={() => {}}
      onReadRecord={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onCopied={() => {}}
      onSaid={() => {}}
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
