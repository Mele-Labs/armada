import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { InlineTag } from "./InlineTag";
import type { ComposerTag } from "./InlineTag";

/** Where a tag stands in a message's text. The tags themselves are listed beside it, in order. */
export const CHIP = "￼";

/** The message as the box holds it: its text with a CHIP at each tag, and the tags in that order. */
export function readDom(root: Node): { text: string; tags: ComposerTag[] } {
  const tags: ComposerTag[] = [];
  let text = "";
  const walk = (node: Node) => {
    node.childNodes.forEach((child) => {
      if (child.nodeType === Node.TEXT_NODE) {
        text += (child as Text).data;
      } else if (child instanceof HTMLElement) {
        if (child.dataset.tagKind !== undefined) {
          text += CHIP;
          tags.push({ kind: child.dataset.tagKind as ComposerTag["kind"], id: child.dataset.tagId ?? "", title: child.dataset.tagTitle ?? "" });
        } else if (child.tagName === "BR") {
          text += "\n";
        } else {
          if (text !== "" && !text.endsWith("\n") && (child.tagName === "DIV" || child.tagName === "P")) text += "\n";
          walk(child);
        }
      }
    });
  };
  walk(root);
  return { text, tags };
}

/** The message as it is sent: each tag written out as `@title` at its place. */
export function sentText(text: string, tags: readonly ComposerTag[]): string {
  let at = 0;
  return text.replace(new RegExp(CHIP, "g"), () => `@${tags[at++]?.title ?? ""}`);
}

function chipNode(tag: ComposerTag): Node {
  const holder = document.createElement("template");
  holder.innerHTML = renderToStaticMarkup(createElement(InlineTag, { tag, atomic: true }));
  return holder.content.firstChild!;
}

/** Draws a message into the box, replacing what was there. */
export function writeDom(root: HTMLElement, text: string, tags: readonly ComposerTag[]): void {
  root.replaceChildren();
  let at = 0;
  text.split(CHIP).forEach((piece, index) => {
    if (index > 0) {
      const tag = tags[at++];
      if (tag !== undefined) root.append(chipNode(tag));
    }
    if (piece !== "") root.append(document.createTextNode(piece));
  });
}

/** A tag, as a node the box can hold. */
export const tagNode = chipNode;

/** What stands before the caret, tags as spaces, and the text node the caret is in. */
export function beforeCaret(root: HTMLElement): { head: string; node: Text | null; offset: number } | undefined {
  const selection = window.getSelection();
  if (selection === null || selection.rangeCount === 0) return undefined;
  const range = selection.getRangeAt(0);
  if (!range.collapsed || !root.contains(range.startContainer)) return undefined;
  const before = document.createRange();
  before.selectNodeContents(root);
  before.setEnd(range.startContainer, range.startOffset);
  const holder = document.createElement("div");
  holder.append(before.cloneContents());
  const head = readDom(holder).text.replace(new RegExp(CHIP, "g"), " ");
  if (range.startContainer.nodeType === Node.TEXT_NODE) return { head, node: range.startContainer as Text, offset: range.startOffset };
  // A caret at the end of a line the box was drawn with sits between nodes, after the text node it follows.
  const prior = range.startContainer.childNodes[range.startOffset - 1];
  return prior?.nodeType === Node.TEXT_NODE ? { head, node: prior as Text, offset: (prior as Text).length } : { head, node: null, offset: 0 };
}

/** The caret at the end of the box. */
export function caretToEnd(root: HTMLElement): void {
  const range = document.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}
