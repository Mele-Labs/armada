// What Helm's dock does with a thought typed in its box, mounted on its own: the card's keys act only
// while the box is empty, a question or a failed read is asked of Helm at once, and the row for words
// sent on is the Helm Session's thread, drawn when it arrives and not when Send is pressed.

import { useState } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import type { HelmContext } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";
import { MANIFEST_ID, MANIFEST_NAME, repository } from "@armada/screens/src/fixtures/repository";
import { HelmDock } from "./HelmDock";
import type { RouteTarget, ThoughtReading, ThoughtRouting } from "./thought-routing";

afterEach(unmount);

const CONTEXT: HelmContext = { screen: "overview" };
const WORDS = "the sessions page should say they are waiting for work to land";
const MATCH: RouteTarget = {
  kind: "session",
  id: "01SESSION",
  title: "Sessions list views",
  repository: MANIFEST_NAME,
  manifestId: MANIFEST_ID,
  slot: 3,
};

const box = () => page.getByRole("textbox", { name: "Ask Helm" });
const sendCard = () => page.getByRole("group", { name: "Send to", exact: true });
const newCard = () => page.getByRole("group", { name: "Start new", exact: true });
const sentChip = () => page.getByRole("button", { name: /^Sent to / });

type Wiring = {
  read?: ThoughtRouting["read"];
  send?: (target: RouteTarget, text: string, deliver: (row: SessionThreadRow) => void) => Promise<boolean>;
  rows?: SessionThreadRow[];
};

/** Where a test plays Fleet: the dock hands this the way to put a row into the thread it holds. */
type Thread = { deliver?: (row: SessionThreadRow) => void };

/** The dock over the Helm Session's thread, which it holds. */
function Dock({ rows: first, thread, onAsk, routing, onOpenSent }: { rows: SessionThreadRow[]; thread: Thread; onAsk: (text: string, context: HelmContext) => void; routing: ThoughtRouting; onOpenSent: (target: { kind: "session"; id: string }) => void }) {
  const [rows, setRows] = useState<SessionThreadRow[]>(first);
  thread.deliver = (row) => setRows((was) => [...was, row]);
  return (
    <HelmDock
      helm={{ state: "pointed", manifestId: MANIFEST_ID }}
      hosted={{ working: false, thread: <SessionThread sessionId="helm-armada" rows={rows} agentName="Helm" onOpenSent={onOpenSent} onAnswer={() => {}} onOpenSession={() => {}} /> }}
      repositories={[repository()]}
      live
      context={CONTEXT}
      onAsk={onAsk}
      onSwitch={() => {}}
      onReadRecord={async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } })}
      onCopied={() => {}}
      onSaid={() => {}}
      routing={routing}
    />
  );
}

function dock(wiring: Wiring = {}) {
  const onAsk = vi.fn<(text: string, context: HelmContext) => void>();
  const send = vi.fn<ThoughtRouting["send"]>();
  const open = vi.fn<ThoughtRouting["open"]>();
  const startNew = vi.fn<ThoughtRouting["startNew"]>();
  const thread: Thread = {};
  const routing: ThoughtRouting = {
    read: wiring.read ?? (async (): Promise<ThoughtReading> => ({ kind: "thought", match: MATCH })),
    send: (target, text) => {
      send(target, text);
      return wiring.send === undefined ? Promise.resolve(true) : wiring.send(target, text, (item) => thread.deliver?.(item));
    },
    open,
    startNew,
  };
  mount(<Dock rows={wiring.rows ?? []} thread={thread} onAsk={onAsk} routing={routing} onOpenSent={(target) => open({ ...MATCH, id: target.id })} />);
  return { onAsk, send, open, startNew };
}

/** What the box holds. It is a text box that is edited in place, so its text is its content. */
const typed = (): string => box().element().textContent ?? "";

/** Type the words and send them with ↵, leaving the box empty and focused as it is after any send. */
async function say(words = WORDS): Promise<void> {
  await box().fill(words);
  await userEvent.keyboard("{Enter}");
}

const noMatch = async (): Promise<ThoughtReading> => ({ kind: "thought" });

test("with text in the box, N H Esc and ↵ are not answers: the card stays and the letters are typed", async () => {
  const { onAsk, send } = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();

  await box().fill("x");
  await userEvent.keyboard("nh");
  await userEvent.keyboard("{Escape}");
  expect(typed()).toBe("xnh");
  await expect.element(sendCard()).toBeVisible();

  // ↵ with words in the box sends them as a thought of their own, queued behind the card: it does not answer it.
  await userEvent.keyboard("{Enter}");
  expect(typed()).toBe("");
  await expect.element(sendCard()).toBeVisible();
  expect(send).not.toHaveBeenCalled();
  expect(onAsk).not.toHaveBeenCalled();
});

test("with text in the box, S is not an answer either", async () => {
  const { onAsk, startNew } = dock({ read: noMatch });
  await say();
  await expect.element(newCard()).toBeVisible();

  await box().fill("x");
  await userEvent.keyboard("s");
  await userEvent.keyboard("{Escape}");

  expect(typed()).toBe("xs");
  await expect.element(newCard()).toBeVisible();
  expect(startNew).not.toHaveBeenCalled();
  expect(onAsk).not.toHaveBeenCalled();
});

test("on a match, ↵ with the box empty sends the words to the target", async () => {
  const { send, onAsk } = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();

  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(send).toHaveBeenCalledWith(MATCH, WORDS));
  await expect.poll(() => sendCard().query()).toBeNull();
  expect(onAsk).not.toHaveBeenCalled();
});

test("on a match, N moves to the new stage and sends nothing", async () => {
  const { send } = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();

  await userEvent.keyboard("n");
  await expect.element(newCard()).toBeVisible();
  expect(sendCard().query()).toBeNull();
  expect(send).not.toHaveBeenCalled();
});

test("H asks Helm with the words as typed, on a match and on the new stage", async () => {
  const first = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();
  await userEvent.keyboard("h");
  expect(first.onAsk).toHaveBeenCalledExactlyOnceWith(WORDS, CONTEXT);
  expect(sendCard().query()).toBeNull();
  unmount();

  const second = dock({ read: noMatch });
  await say("another thought");
  await expect.element(newCard()).toBeVisible();
  await userEvent.keyboard("h");
  expect(second.onAsk).toHaveBeenCalledExactlyOnceWith("another thought", CONTEXT);
  expect(newCard().query()).toBeNull();
});

test("S starts a Session with the words, and Esc puts the card away", async () => {
  const session = dock({ read: noMatch });
  await say();
  await expect.element(newCard()).toBeVisible();
  await userEvent.keyboard("s");
  expect(session.startNew).toHaveBeenCalledExactlyOnceWith(WORDS);
  expect(newCard().query()).toBeNull();
  unmount();

  const away = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(sendCard().query()).toBeNull();
  expect(away.send).not.toHaveBeenCalled();
  expect(away.onAsk).not.toHaveBeenCalled();
});

test("a button with focus keeps ↵ for itself, so one press answers once", async () => {
  const { send, onAsk } = dock();
  await say();
  await expect.element(sendCard()).toBeVisible();

  (page.getByRole("button", { name: "No" }).element() as HTMLElement).focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(newCard()).toBeVisible();
  expect(send).not.toHaveBeenCalled();

  (page.getByRole("button", { name: "Ask Helm" }).element() as HTMLElement).focus();
  await userEvent.keyboard("{Enter}");
  expect(onAsk).toHaveBeenCalledTimes(1);
  expect(send).not.toHaveBeenCalled();
});

test("a second ↵ while the words are being delivered does not send them twice", async () => {
  let arrived: (delivered: boolean) => void = () => {};
  const { send } = dock({ send: () => new Promise<boolean>((resolve) => (arrived = resolve)) });
  await say();
  await expect.element(sendCard()).toBeVisible();

  await userEvent.keyboard("{Enter}");
  await userEvent.keyboard("{Enter}");
  expect(send).toHaveBeenCalledTimes(1);
  arrived(true);
  await expect.poll(() => sendCard().query()).toBeNull();
});

test("a question goes straight to Helm, with no card", async () => {
  const { onAsk } = dock({ read: async () => ({ kind: "question" }) });
  await say("why did Job 12 fail?");
  await vi.waitFor(() => expect(onAsk).toHaveBeenCalledExactlyOnceWith("why did Job 12 fail?", CONTEXT));
  expect(sendCard().query()).toBeNull();
  expect(newCard().query()).toBeNull();
});

test("a read that was rejected goes to Helm, so the words are not lost", async () => {
  const { onAsk } = dock({ read: () => Promise.reject(new Error("the connection closed")) });
  await say("the merge line page should group PRs by repository");
  await vi.waitFor(() =>
    expect(onAsk).toHaveBeenCalledExactlyOnceWith("the merge line page should group PRs by repository", CONTEXT),
  );
  expect(newCard().query()).toBeNull();
});

test("a read that throws before it returns a promise goes to Helm too", async () => {
  const { onAsk } = dock({
    read: () => {
      throw new Error("no bridge");
    },
  });
  await say();
  await vi.waitFor(() => expect(onAsk).toHaveBeenCalledExactlyOnceWith(WORDS, CONTEXT));
});

test("a refused send draws no row and leaves the card, then the row is drawn when the thread has it", async () => {
  let refuse = true;
  const { send } = dock({
    send: async (target, text, deliver) => {
      if (refuse) return false;
      deliver({ kind: "sent", id: "1", at: "10:00:00", text, target: { kind: target.kind, id: target.id, title: target.title, repository: MANIFEST_NAME, slot: 3 } });
      return true;
    },
  });
  await say();
  await expect.element(sendCard()).toBeVisible();

  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(send).toHaveBeenCalledTimes(1));
  await expect.element(sendCard()).toBeVisible();
  expect(sentChip().query()).toBeNull();

  refuse = false;
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("button", { name: "Sent to Job Sessions list views" })).toBeVisible();
  expect(sendCard().query()).toBeNull();
  expect(send).toHaveBeenCalledTimes(2);
});

test("a sent row in the thread names the repository, and opens the target", async () => {
  const row: SessionThreadRow = {
    id: "4",
    at: "10:00:00",
    kind: "sent",
    text: WORDS,
    target: { kind: "session", id: "01SESSION", title: "Sessions list views", repository: MANIFEST_NAME, slot: 3 },
  };
  const { open } = dock({ rows: [row] });

  // Drawn from the thread alone: nothing was sent from this dock.
  await expect.element(page.getByText(WORDS)).toBeVisible();
  const chip = page.getByRole("button", { name: "Sent to Job Sessions list views" });
  await expect.element(chip).toHaveTextContent(MANIFEST_NAME);
  await chip.click();
  expect(open).toHaveBeenCalledExactlyOnceWith(MATCH);
});

const FIRST = "retries should back off longer after the third failed charge";
const SECOND = "the merge line page should group pull requests by repository";

/** A reading that answers by the words, so two thoughts sent back to back can read differently. */
const byWords =
  (answers: Record<string, ThoughtReading>): ThoughtRouting["read"] =>
  async (text) =>
    answers[text] ?? { kind: "question" };

test("thoughts sent while one is showing queue in the order typed, and answering one shows the next", async () => {
  const { send, onAsk } = dock({ read: byWords({ [FIRST]: { kind: "thought", match: MATCH }, [SECOND]: { kind: "thought" } }) });
  await say(FIRST);
  await say(SECOND);

  await expect.element(sendCard()).toBeVisible();
  await expect.element(page.getByText(FIRST, { exact: true })).toBeVisible();
  expect(newCard().elements()).toHaveLength(0);

  await userEvent.keyboard("{Enter}");
  expect(send).toHaveBeenCalledWith(MATCH, FIRST);
  await expect.element(newCard()).toBeVisible();
  await expect.element(page.getByText(SECOND, { exact: true })).toBeVisible();

  await userEvent.keyboard("h");
  expect(onAsk).toHaveBeenCalledWith(SECOND, CONTEXT);
  expect(newCard().elements()).toHaveLength(0);
});

test("the cards come in the order typed even when the later reading lands first", async () => {
  const slow: { [text: string]: (reading: ThoughtReading) => void } = {};
  const { onAsk } = dock({
    read: (text) => new Promise<ThoughtReading>((resolve) => (slow[text] = resolve)),
  });
  await say(FIRST);
  await say(SECOND);
  slow[SECOND]?.({ kind: "thought" });
  expect(sendCard().elements()).toHaveLength(0);
  expect(newCard().elements()).toHaveLength(0);

  slow[FIRST]?.({ kind: "thought", match: MATCH });
  await expect.element(sendCard()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(newCard()).toBeVisible();
  expect(onAsk).not.toHaveBeenCalled();
});

test("a queued message that reads as a question goes to Helm at once and does not wait behind a card", async () => {
  const { onAsk } = dock({ read: byWords({ [FIRST]: { kind: "thought", match: MATCH } }) });
  await say(FIRST);
  await expect.element(sendCard()).toBeVisible();

  await say("why did Job 12 fail?");

  await vi.waitFor(() => expect(onAsk).toHaveBeenCalledWith("why did Job 12 fail?", CONTEXT));
  await expect.element(sendCard()).toBeVisible();
});

test("nothing marks how many are waiting behind the card", async () => {
  dock({ read: byWords({ [FIRST]: { kind: "thought", match: MATCH }, [SECOND]: { kind: "thought" } }) });
  await say(FIRST);
  await say(SECOND);
  await expect.element(sendCard()).toBeVisible();

  expect(sendCard().element().textContent).not.toMatch(/\b2\b|\+1|more|waiting/i);
});
