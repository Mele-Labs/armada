// The message box draws the markdown it holds: a quote, a list, a link. What is kept and sent stays
// the text it always was, Enter and Backspace in a quote or a list do what a list or a quote does,
// and a Reply puts words into the box kept for a Session.

import { forgetAllKept, quoteIntoDraft, readKept, SessionComposer, writeKept } from "@armada/components";
import type { SentFromComposer } from "@armada/components";
import { readDom, sentText, writeDom } from "@armada/components/src/compositions/SessionComposer/message-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const KEY = "markdown";
const kept = () => readKept<{ text: string; tags: unknown[]; files: unknown[] }>(`session:${KEY}`);

const roots: { root: Root; host: HTMLElement }[] = [];

beforeEach(forgetAllKept);
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  forgetAllKept();
});

function props(onSend: (sent: SentFromComposer) => void) {
  return {
    working: false,
    mode: "auto" as const,
    onMode: () => undefined,
    model: null,
    effort: null,
    models: [],
    efforts: [],
    onTune: () => undefined,
    commands: [],
    taggable: [],
    tags: [],
    onTags: () => undefined,
    drawn: [],
    onDraw: () => undefined,
    onRemoveDrawn: () => undefined,
    onSend,
    draftKey: KEY,
  };
}

/** The box, mounted over what is already kept for it; the caret stands at the end of that. */
function composer(onSend: (sent: SentFromComposer) => void = () => undefined, focusFor: string | { key: string; at: number } = KEY) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const draw = (asked: string | { key: string; at: number }) => root.render(<SessionComposer {...props(onSend)} focusFor={asked} />);
  draw(focusFor);
  return draw;
}

const message = () => page.getByRole("textbox", { name: "Message" });
const box = () => document.querySelector<HTMLElement>('[role="textbox"]')!;
const focused = () => document.activeElement === document.querySelector('[role="textbox"]');

/** Starts a box kept as `text` and draws it. */
function typed(text: string, onSend?: (sent: SentFromComposer) => void) {
  writeKept(`session:${KEY}`, { text, tags: [], files: [] }, { text, tags: [] });
  composer(onSend);
}

/** Keys pressed once the box has taken focus; on a slow runner they would otherwise land before it. */
async function keys(pressed: string) {
  await expect.poll(focused).toBe(true);
  await userEvent.keyboard(pressed);
}

/** The caret `at` characters into the first text node under `selector`. */
async function caretIn(selector: string, at: number) {
  await expect.poll(() => (box()?.querySelector(selector) ?? null) !== null).toBe(true);
  const node = document.createTreeWalker(box().querySelector(selector)!, NodeFilter.SHOW_TEXT).nextNode()!;
  window.getSelection()!.setBaseAndExtent(node, at, node, at);
}

// What the box draws and what it reads back.

test("Markdown with a quote, a list, a web link, a markdown link, a path and a tag draws as such and reads back as the same text", () => {
  const host = document.createElement("div");
  document.body.append(host);
  const tags = [{ kind: "session" as const, id: "s2", title: "Release notes script" }];
  const text = "Ask ￼ about this\n> the store stamps the row\n> from its own read\n- first https://example.com/a here\n- see [the docs](https://example.com/docs) and crates/fleet/src/servers.rs next\nthen words";
  writeDom(host, text, tags);

  expect(readDom(host)).toEqual({ text, tags });

  const quote = host.querySelector("blockquote")!;
  expect(Array.from(quote.children).map((line) => line.textContent)).toEqual(["the store stamps the row", "from its own read"]);
  const list = host.querySelector("ul")!;
  expect(Array.from(list.children).map((line) => line.tagName)).toEqual(["LI", "LI"]);
  expect(list.querySelector("span[title='https://example.com/a']")?.textContent).toBe("https://example.com/a");
  // A markdown link is drawn as its words and holds the whole.
  expect(list.textContent).toContain("the docs");
  expect(list.textContent).not.toContain("](");
  expect(list.querySelector("span[title='crates/fleet/src/servers.rs']")?.textContent).toBe("crates/fleet/src/servers.rs");
  host.remove();
});

test("A quote or a list followed by words is sent with a blank line before them, and the tags written out", () => {
  expect(sentText("> a\nthen words", [])).toBe("> a\n\nthen words");
  expect(sentText("- a\n- b\nthen words", [])).toBe("- a\n- b\n\nthen words");
  expect(sentText("* a\nthen", [])).toBe("* a\n\nthen");
  // Already apart, still a block, or a block that opens the message: nothing is added.
  expect(sentText("> a\n\nthen words", [])).toBe("> a\n\nthen words");
  expect(sentText("> a\n> b", [])).toBe("> a\n> b");
  expect(sentText("words\n> a", [])).toBe("words\n> a");
  expect(sentText("> ask ￼\nthen", [{ kind: "session", id: "s2", title: "Release notes" }])).toBe("> ask @Release notes\n\nthen");
});

test("Lines inside a code fence are not drawn as a list, and are sent as they were", () => {
  const text = "```\n- old\n+ new\n```";
  const host = document.createElement("div");
  document.body.append(host);
  writeDom(host, text, []);

  expect(host.querySelector("ul")).toBeNull();
  expect(host.querySelector("blockquote")).toBeNull();
  expect(readDom(host).text).toBe(text);
  expect(sentText(text, [])).toBe(text);
  // Outside the fence the same lines are a list again.
  expect(sentText("```\n- old\n```\n- item\nafter", [])).toBe("```\n- old\n```\n- item\n\nafter");
  host.remove();
});

// A Reply puts words in the box kept for a Session.

test("A quote goes alone into an empty box", () => {
  quoteIntoDraft(KEY, "> the store stamps the row\n\n");
  expect(kept()?.text).toBe("> the store stamps the row\n");
});

test("A quote goes under what is typed, a blank line between, and the tags stay", () => {
  const tags = [{ kind: "session", id: "s2", title: "Release notes" }];
  writeKept(`session:${KEY}`, { text: "Ask ￼ about this\n", tags, files: [] }, { text: "Ask ￼ about this\n", tags });
  quoteIntoDraft(KEY, "> the store stamps the row\n\n");
  expect(kept()).toEqual({ text: "Ask ￼ about this\n\n> the store stamps the row\n", tags, files: [] });
});

test("A quote goes under a second one, apart from it", () => {
  quoteIntoDraft(KEY, "> one\n\n");
  quoteIntoDraft(KEY, "> two\n\n");
  expect(kept()?.text).toBe("> one\n\n> two\n");
});

test("A draft that only storage holds comes back with no files, with the quote under it", () => {
  window.localStorage.setItem(`armada:kept:session:${KEY}`, JSON.stringify({ text: "typed before a reload", tags: [] }));
  quoteIntoDraft(KEY, "> words\n\n");
  expect(kept()).toEqual({ text: "typed before a reload\n\n> words\n", tags: [], files: [] });
});

test("A quote put in while the box is on screen is drawn as a quote under what is typed", async () => {
  typed("my question");
  await expect.element(message()).toHaveTextContent("my question");
  quoteIntoDraft(KEY, "> the store stamps the row\n\n");
  await expect.poll(() => box().querySelector("blockquote")?.textContent).toBe("the store stamps the row");
  await expect.element(message()).toHaveTextContent("my question");
});

test("Asking again with a new time puts the caret back in a box that lost it", async () => {
  const draw = composer(() => undefined, { key: KEY, at: 1 });
  await expect.poll(focused).toBe(true);
  box().blur();
  expect(focused()).toBe(false);
  draw({ key: KEY, at: 2 });
  await expect.poll(focused).toBe(true);
});

// Keys in a quote and a list.

test("Enter at the end of a quote's last line leaves the quote for a plain line", async () => {
  const sent = vi.fn();
  typed("> one", sent);
  await keys("{Enter}after");
  expect(kept()?.text).toBe("> one\nafter");
  expect(sent).not.toHaveBeenCalled();
});

test("Enter on a quote's empty last line goes back to a plain line", async () => {
  const sent = vi.fn();
  typed("> one\n> ", sent);
  await keys("{Enter}after");
  expect(kept()?.text).toBe("> one\nafter");
  expect(sent).not.toHaveBeenCalled();
});

test("Shift+Enter in a quote adds another quote line", async () => {
  typed("> one");
  await keys("{Shift>}{Enter}{/Shift}two");
  expect(kept()?.text).toBe("> one\n> two");
  expect(box().querySelectorAll("blockquote > *")).toHaveLength(2);
});

test("Enter in a list item starts the next item", async () => {
  const sent = vi.fn();
  typed("- a", sent);
  await keys("{Enter}b");
  expect(kept()?.text).toBe("- a\n- b");
  expect(sent).not.toHaveBeenCalled();
});

test("Enter in the middle of a list item splits it", async () => {
  typed("- abcd");
  await caretIn("li", 2);
  await keys("{Enter}");
  expect(kept()?.text).toBe("- ab\n- cd");
});

test("Enter on an empty list item ends the list", async () => {
  typed("- a\n- ");
  await keys("{Enter}after");
  expect(kept()?.text).toBe("- a\nafter");
});

test("Shift+Enter in a list adds the next item", async () => {
  typed("- a");
  await keys("{Shift>}{Enter}{/Shift}b");
  expect(kept()?.text).toBe("- a\n- b");
});

test("Backspace at the start of a quote line makes it a plain line, and elsewhere in it deletes a letter", async () => {
  typed("> one");
  await caretIn("blockquote > *", 1);
  await keys("{Backspace}");
  expect(kept()?.text).toBe("> ne");
  await caretIn("blockquote > *", 0);
  await keys("{Backspace}");
  expect(kept()?.text).toBe("ne");
  expect(box().querySelector("blockquote")).toBeNull();
});

test("Backspace at the start of a list item makes it a plain line", async () => {
  typed("- a\n- b");
  await caretIn("li:last-child", 0);
  await keys("{Backspace}");
  expect(kept()?.text).toBe("- a\nb");
  expect(box().querySelectorAll("li")).toHaveLength(1);
});

test("Ctrl+Enter sends from a list item rather than starting the next one", async () => {
  const sent = vi.fn();
  typed("- a", sent);
  await keys("{Control>}{Enter}{/Control}");
  expect(sent).toHaveBeenCalledTimes(1);
  expect(sent.mock.calls[0]![0].text).toBe("- a");
});

test("Ctrl+Enter sends from the end of a quote rather than leaving it", async () => {
  const sent = vi.fn();
  typed("> one", sent);
  await keys("{Control>}{Enter}{/Control}");
  expect(sent).toHaveBeenCalledTimes(1);
  expect(sent.mock.calls[0]![0].text).toBe("> one");
});

test("Enter on a plain line sends, with a blank line put after a quote above it", async () => {
  const sent = vi.fn();
  typed("> one\nafter", sent);
  await keys("{Enter}");
  expect(sent).toHaveBeenCalledTimes(1);
  expect(sent.mock.calls[0]![0].text).toBe("> one\n\nafter");
});

test("Enter on a plain line with no block sends as before", async () => {
  const sent = vi.fn();
  typed("hello", sent);
  await keys("{Enter}");
  expect(sent.mock.calls.map((one) => one[0].text)).toEqual(["hello"]);
});
