// A document's notes, each under the block it names. The page is not React's (`DocumentPage.tsx`): its nodes are put there by hand, so a holder is put after the
// block by hand too, and the note is drawn into it by a portal. **Drawn after the page**, so the holders go into the nodes the page put there. A note that names no
// block, or words no block on the page says (it may be another revision), stands at the head of the page, so none is lost.

import { useEffect, useLayoutEffect, useState, type RefObject } from "react";
import { createPortal } from "react-dom";

import { JudgeNotes, useJudgeNotes } from "./notes";
import { notesBrought, useNotesAsked, type JudgeArtifact } from "./store";

/** The tags Fleet divides a page's text at (`text_of`, `crates/fleet/src/judge_content.rs`). A note's `about` is `p:` and the words of one such block. */
const BLOCKS = "p, h1, h2, h3, h4, h5, h6, li, tr, pre, blockquote, section, div";

const WORDS = "p:";

const wordsOf = (el: Element): string => (el.textContent ?? "").split(/\s+/).filter(Boolean).join(" ");

/** Where a holder goes for the block: inside a list item, after a row's whole table, else straight after it. */
const holderAfter = (block: HTMLElement, holder: HTMLElement) => {
  if (block.tagName === "LI") return block.append(holder);
  (block.tagName === "TR" ? (block.closest("table") ?? block) : block).after(holder);
};

type Holder = { about: string; el: HTMLElement };

/** `root` is the page the layer drew: a new one is a new set of nodes, so the holders are made again. */
export function JudgedPage({ page, root, job, artifact }: { page: RefObject<HTMLDivElement | null>; root: HTMLElement; job: string; artifact: Pick<JudgeArtifact, "kind" | "id"> }) {
  const { notes } = useJudgeNotes(job, artifact);
  const asked = useNotesAsked(job, artifact);
  const [holders, setHolders] = useState<readonly Holder[]>([]);
  const [head, setHead] = useState<HTMLElement | undefined>();
  const [placed, setPlaced] = useState<ReadonlySet<string>>(new Set());
  const wanted = JSON.stringify([...new Set(notes.flatMap((one) => (one.about?.startsWith(WORDS) === true ? [one.about] : [])))]);
  const any = notes.length > 0;
  useLayoutEffect(() => {
    const el = page.current;
    if (el === null || !any) {
      setHolders([]);
      setHead(undefined);
      setPlaced(new Set());
      return;
    }
    const blocks = [...el.querySelectorAll<HTMLElement>(BLOCKS)].filter((one) => one.closest(".armada-doc__sketch") === null).map((one) => ({ one, words: wordsOf(one) }));
    const made: HTMLElement[] = [];
    const holder = () => {
      const put = document.createElement("div");
      put.className = "judge-page-notes";
      made.push(put);
      return put;
    };
    const next = (JSON.parse(wanted) as string[]).flatMap((about) => {
      const said = about.slice(WORDS.length);
      const same = blocks.filter((one) => one.words === said).map((one) => one.one);
      // A block can hold no more than one that says the same: the note goes under the innermost, the one the words are in.
      const block = same.find((one) => !same.some((other) => other !== one && one.contains(other)));
      if (block === undefined) return [];
      const under = holder();
      holderAfter(block, under);
      return [{ about, el: under }];
    });
    const top = holder();
    el.prepend(top);
    setHolders(next);
    setHead(top);
    setPlaced(new Set(next.map((one) => one.about)));
    return () => made.forEach((one) => one.remove());
  }, [page, root, wanted, any]);
  // A press asked for the notes: the first on the page comes into view, and an open page that has none is asked no more.
  useEffect(() => {
    if (!asked || head === undefined) return;
    page.current?.querySelector(".judge-notes")?.scrollIntoView({ block: "center" });
    notesBrought();
  }, [asked, head, holders, page]);
  return (
    <>
      {head === undefined ? null : createPortal(<JudgeNotes job={job} artifact={artifact} label="Notes from the judge" only={(note) => note.about === undefined || !placed.has(note.about)} />, head)}
      {holders.map((one) => createPortal(<JudgeNotes job={job} artifact={artifact} label="Notes on this part" only={(note) => note.about === one.about} />, one.el, one.about))}
    </>
  );
}
