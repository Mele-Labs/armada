// What changed between two revisions of a Job's document. The page is cut into its text blocks (a paragraph, a list item, a cell, a
// heading), the blocks of the two revisions are lined up, and a block that moved on is diffed by word: inserted words in `ins`, removed
// ones in `del`. A block with no partner is added, or removed and drawn struck where it stood. Nothing here reads a sketch's scene.

import { onOneLine, type NotedRef } from "@armada/components";

import { rowText } from "./document-page";

const LEAF = "h1,h2,h3,h4,h5,h6,p,li,td,th,figcaption,caption,blockquote,dt,dd,pre,summary";
/** Blocks drawn struck where they stood. A removed cell has no place to stand in a row that still has the others. */
const STRUCK = new Set(["td", "th"]);

type Leaf = { el: Element; tag: string; text: string; section: string; row: string | undefined };

export type Change = {
  id: number;
  kind: "added" | "changed" | "removed";
  /** Where it stands among the new revision's blocks, for telling which change is nearest a note. */
  at: number;
  section: string;
  /** The words that came or went, short. */
  said: string;
};

export type Diff = {
  /** The new revision, every change marked `data-change` and numbered `data-change-id`. A copy; the page it was made from is as it was. */
  root: HTMLElement;
  changes: readonly Change[];
  /** The change that answers a note made on the revision before: in its section, the one nearest the words, the part or the row. */
  answer: (ref: Extract<NotedRef, { kind: "document" }>) => Change | undefined;
};

function leavesOf(root: HTMLElement): { leaves: Leaf[]; sketchAt: Map<string, number> } {
  const leaves: Leaf[] = [];
  const sketchAt = new Map<string, number>();
  let section = "";
  for (const el of Array.from(root.querySelectorAll(`${LEAF},figure.armada-doc__sketch`))) {
    if (el.matches("figure")) {
      sketchAt.set(el.getAttribute("data-title") ?? "", leaves.length);
      continue;
    }
    if (el.querySelector(LEAF) !== null) continue;
    const text = onOneLine(el.textContent ?? "");
    if (el.tagName === "H2") section = text;
    if (text === "") continue;
    const tag = el.tagName.toLowerCase();
    const row = tag === "td" || tag === "th" ? (el.parentElement === null ? undefined : rowText(el.parentElement)) : undefined;
    leaves.push({ el, tag, text, section, row });
  }
  return { leaves, sketchAt };
}

const wordsOf = (text: string) => text.toLowerCase().split(/\W+/).filter((one) => one !== "");

/** Whether two blocks are one block, edited: the same words, or most of the shorter one's. */
function alike(a: Leaf, b: Leaf): boolean {
  if (a.tag !== b.tag) return false;
  if (a.text === b.text) return true;
  const x = new Set(wordsOf(a.text));
  const y = new Set(wordsOf(b.text));
  const shared = [...x].filter((one) => y.has(one)).length;
  const least = Math.min(x.size, y.size);
  if (least === 0) return false;
  return shared / least >= 0.6 && (shared >= 2 || Math.max(a.text.length, b.text.length) <= 24);
}

type Op = { kind: "keep"; a: number; b: number } | { kind: "add"; b: number } | { kind: "drop"; a: number };

function align(a: readonly Leaf[], b: readonly Leaf[]): Op[] {
  const width = b.length + 1;
  const table = new Uint16Array((a.length + 1) * width);
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      table[i * width + j] = alike(a[i]!, b[j]!) ? table[(i + 1) * width + j + 1]! + 1 : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!);
    }
  }
  const ops: Op[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && alike(a[i]!, b[j]!) && table[i * width + j] === table[(i + 1) * width + j + 1]! + 1) {
      ops.push({ kind: "keep", a: i, b: j });
      i += 1;
      j += 1;
    } else if (j < b.length && (i === a.length || table[i * width + j + 1]! >= table[(i + 1) * width + j]!)) {
      ops.push({ kind: "add", b: j });
      j += 1;
    } else {
      ops.push({ kind: "drop", a: i });
      i += 1;
    }
  }
  return ops;
}

type Piece = { op: "same" | "ins" | "del"; text: string };

const tokensOf = (text: string) => text.match(/\s*(?:[\p{L}\p{N}_'’]+|[^\p{L}\p{N}_\s])/gu) ?? [];

/**
 * The words of two texts lined up, as nodes: what both hold as it stands, and each run of change as the words that went in `del` and
 * the words that came in `ins`. A word or two that happens to be in both, between changes, is part of the change and not a landmark.
 */
function wordNodes(before: string, after: string): { nodes: Node[]; came: string; went: string } {
  const a = tokensOf(before);
  const b = tokensOf(after);
  const same = (x: string | undefined, y: string | undefined) => x !== undefined && y !== undefined && x.trim() === y.trim();
  const width = b.length + 1;
  const table = new Uint16Array((a.length + 1) * width);
  for (let i = a.length - 1; i >= 0; i -= 1) {
    for (let j = b.length - 1; j >= 0; j -= 1) {
      table[i * width + j] = same(a[i], b[j]) ? table[(i + 1) * width + j + 1]! + 1 : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!);
    }
  }
  const pieces: Piece[] = [];
  const push = (op: Piece["op"], text: string) => {
    const last = pieces[pieces.length - 1];
    if (last?.op === op) last.text += text;
    else pieces.push({ op, text });
  };
  let i = 0;
  let j = 0;
  while (i < a.length || j < b.length) {
    if (i < a.length && j < b.length && same(a[i], b[j])) {
      push("same", b[j]!);
      i += 1;
      j += 1;
    } else if (j < b.length && (i === a.length || table[i * width + j + 1]! >= table[(i + 1) * width + j]!)) {
      push("ins", b[j]!);
      j += 1;
    } else {
      push("del", a[i]!);
      i += 1;
    }
  }
  const nodes: Node[] = [];
  let came = "";
  let went = "";
  for (let at = 0; at < pieces.length; ) {
    const piece = pieces[at]!;
    if (piece.op === "same") {
      nodes.push(document.createTextNode(piece.text));
      at += 1;
      continue;
    }
    // A run of change: up to the next stretch both texts share that is longer than a word or two.
    let out = "";
    let gone = "";
    let end = at;
    for (; end < pieces.length; end += 1) {
      const one = pieces[end]!;
      const small = one.op === "same" && one.text.trim().length <= 6 && end > at && end < pieces.length - 1;
      if (one.op === "same" && !small) break;
      if (one.op !== "ins") gone += one.text;
      if (one.op !== "del") out += one.text;
    }
    const lead = /^\s*/.exec(gone === "" ? out : gone)?.[0] ?? "";
    if (lead !== "") nodes.push(document.createTextNode(lead));
    if (gone.trim() !== "") {
      const del = document.createElement("del");
      del.textContent = gone.trim();
      nodes.push(del);
    }
    if (out.trim() !== "") {
      if (gone.trim() !== "") nodes.push(document.createTextNode(" "));
      const ins = document.createElement("ins");
      ins.textContent = out.trim();
      nodes.push(ins);
    }
    came += out;
    went += gone;
    at = end;
  }
  return { nodes, came: came.trim(), went: went.trim() };
}

const cut = (words: string, at: number) => (words.length > at ? `${words.slice(0, at).trimEnd()}…` : words);

export function diffPages(before: HTMLElement, after: HTMLElement): Diff {
  const root = after.cloneNode(true) as HTMLElement;
  const old = leavesOf(before);
  const now = leavesOf(root);
  const ops = align(old.leaves, now.leaves);
  const changes: Change[] = [];
  const mark = (el: Element, kind: Change["kind"], at: number, section: string, said: string) => {
    const id = changes.length + 1;
    el.setAttribute("data-change", kind);
    el.setAttribute("data-change-id", String(id));
    changes.push({ id, kind, at, section, said: cut(said, 60) });
  };
  /** The new revision's block that comes next after op `from`, which a struck block is put in front of. */
  const nextB = (from: number): number | undefined => {
    for (let k = from; k < ops.length; k += 1) {
      const op = ops[k]!;
      if (op.kind !== "drop") return op.b;
    }
    return undefined;
  };
  const placeOf = new Array<number>(old.leaves.length).fill(0);
  ops.forEach((op, k) => {
    if (op.kind === "keep") {
      const was = old.leaves[op.a]!;
      const is = now.leaves[op.b]!;
      placeOf[op.a] = op.b;
      if (was.text === is.text) return;
      const { nodes, came, went } = wordNodes(was.text, is.text);
      is.el.replaceChildren(...nodes);
      mark(is.el, "changed", op.b, is.section, came === "" ? went : came);
    } else if (op.kind === "add") {
      const is = now.leaves[op.b]!;
      mark(is.el, "added", op.b, is.section, is.text);
    } else {
      const was = old.leaves[op.a]!;
      const next = nextB(k);
      placeOf[op.a] = next ?? Math.max(0, now.leaves.length - 1);
      if (STRUCK.has(was.tag)) return;
      const struck = was.el.cloneNode(true) as Element;
      const stands = next === undefined ? undefined : now.leaves[next]!.el;
      if (stands?.parentNode != null) stands.parentNode.insertBefore(struck, stands);
      else root.appendChild(struck);
      mark(struck, "removed", next ?? now.leaves.length, was.section, was.text);
    }
  });
  // A row whose every cell is new is a row that is new.
  for (const row of Array.from(root.querySelectorAll("tr"))) {
    const cells = [...row.children];
    if (cells.length > 0 && cells.every((cell) => cell.getAttribute("data-change") === "added")) row.setAttribute("data-change", "added");
  }
  const answer: Diff["answer"] = (ref) => {
    const target = ref.target;
    const at =
      target.kind === "words"
        ? old.leaves.findIndex((one) => one.text.includes(onOneLine(target.quote)))
        : target.kind === "row"
          ? old.leaves.findIndex((one) => one.row === onOneLine(target.row))
          : (old.sketchAt.get(target.sketch) ?? -1);
    if (at < 0) return undefined;
    const near = placeOf[Math.min(at, placeOf.length - 1)] ?? 0;
    return changes.filter((one) => one.section === ref.place.section).sort((x, y) => Math.abs(x.at - near) - Math.abs(y.at - near) || x.at - y.at)[0];
  };
  return { root, changes, answer };
}
