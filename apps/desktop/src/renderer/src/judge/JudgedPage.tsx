// A document's notes, each under the paragraph it names. The page is not React's (`DocumentPage.tsx`): its nodes are put there by hand, so a holder is put
// after the paragraph by hand too, and the note is drawn into it by a portal. **Drawn after the page**, so the holders go into the nodes the page put there.
// A note that names no paragraph is the block at the head of the page (`JudgeNoteBlock`).

import { useLayoutEffect, useState, type RefObject } from "react";
import { createPortal } from "react-dom";

import { JudgeNotes, useJudgeNotes } from "./notes";
import type { JudgeArtifact } from "./store";

/** The paragraphs of the page, in reading order, outside any sketch. A note's `about` is `p:` and the paragraph's place in this list. */
const paragraphs = (page: HTMLElement): HTMLElement[] => [...page.querySelectorAll<HTMLElement>("p")].filter((one) => one.closest(".armada-doc__sketch") === null);

export const paragraphAbout = (index: number): string => `p:${index}`;

/** `root` is the page the layer drew: a new one is a new set of nodes, so the holders are made again. */
export function JudgedPage({ page, root, job, artifact }: { page: RefObject<HTMLDivElement | null>; root: HTMLElement; job: string; artifact: Pick<JudgeArtifact, "kind" | "id"> }) {
  const { notes } = useJudgeNotes(job, artifact);
  const [holders, setHolders] = useState<readonly { about: string; el: HTMLElement }[]>([]);
  const wanted = notes.flatMap((one) => (one.about?.startsWith("p:") === true ? [one.about] : [])).join(" ");
  useLayoutEffect(() => {
    const el = page.current;
    if (el === null || wanted === "") return setHolders([]);
    const all = paragraphs(el);
    const made: HTMLElement[] = [];
    const next = wanted.split(" ").flatMap((about) => {
      const after = all[Number(about.slice(2))];
      if (after === undefined) return [];
      const holder = document.createElement("div");
      holder.className = "judge-page-notes";
      after.after(holder);
      made.push(holder);
      return [{ about, el: holder }];
    });
    setHolders(next);
    return () => made.forEach((one) => one.remove());
  }, [page, root, wanted]);
  return <>{holders.map((one) => createPortal(<JudgeNotes job={job} artifact={artifact} label="Notes on this paragraph" only={(note) => note.about === one.about} />, one.el, one.about))}</>;
}
