// A Job's document drawn in the reading layer. The cleaned page (`document-page.ts`) is copied into one element and its CSS adopted
// under that element's class; Bridge's own components are mounted into it: the real sketch at each `<armada-sketch>`, and the number
// of each note at the words or the row it is about. **The page is not React's**: React draws the element and the portals into it, and
// the page's own nodes are put there by hand, so a note's mark is inserted and taken out without React knowing.

import { useEffect, useLayoutEffect, useState, type KeyboardEvent, type MouseEvent, type RefObject } from "react";
import { createPortal } from "react-dom";
import { NoteNumber, SketchScene, type DocumentTarget, type NotedRef, type ViewerNotes, type ViewerNotesScope } from "@armada/components";

import { placeOf, pointedAt, rangeOf, rowOf } from "./document-find";
import { PAGE_ROOT, sectionOf, type PageSketch } from "./document-page";
import { openLink } from "../commands";

/** A note already sent, as the thread holds it. */
export type SentNote = { n: number; ref: Extract<NotedRef, { kind: "document" }>; body: string };

/** A note the margin lists, held or sent, and where it stands in the page. */
export type PageNote = { id: string; revision: number; target: DocumentTarget };

/** The note whose place is lit: from the margin, which scrolls the page to it, or from the page, which does not move. */
export type LitNote = { id: string; from: "margin" | "page" };

/** Where the layer was asked to open the page: a place a sent note named. */
export type DocumentFocus = { target: DocumentTarget; at: number };

type Mount = { el: HTMLElement; index: number; title: string; section: string };
type Anchor = { n: number; writing: boolean; said: string; el: HTMLElement };

type Props = {
  title: string;
  revision: number;
  /** The page to draw: a revision as it is, or the new one with its changes marked. */
  root: HTMLElement;
  css: string;
  sketches: readonly PageSketch[];
  /** The revision before, whose sketches the new ones are drawn against while Changes is on. */
  against?: readonly PageSketch[] | undefined;
  scope: ViewerNotesScope;
  viewer: ViewerNotes;
  page: RefObject<HTMLDivElement | null>;
  sent: readonly SentNote[];
  focus: DocumentFocus | undefined;
  /** Whether the Notes margin is drawn: the notes stand highlighted in the page only then. */
  margin: boolean;
  notes: readonly PageNote[];
  lit: LitNote | undefined;
  onLit: (to: (was: LitNote | undefined) => LitNote | undefined) => void;
};

const highlights = (): HighlightRegistry | undefined => (typeof CSS !== "undefined" && "highlights" in CSS && typeof Highlight !== "undefined" ? CSS.highlights : undefined);

export function DocumentPage({ title, revision, root, css, sketches, against, scope, viewer, page, sent, focus, margin, notes, lit, onLit }: Props) {
  const [mounts, setMounts] = useState<readonly Mount[]>([]);
  const [anchors, setAnchors] = useState<readonly Anchor[]>([]);

  // The page's CSS reaches only the page: every rule was put under the page's class when it was read.
  useEffect(() => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(css);
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, sheet];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((one) => one !== sheet);
    };
  }, [css]);

  useLayoutEffect(() => {
    const el = page.current;
    if (el === null) return;
    el.replaceChildren(...[...root.childNodes].map((one) => one.cloneNode(true)));
    setMounts(
      [...el.querySelectorAll<HTMLElement>(".armada-doc__sketch-mount")].flatMap((mount) => {
        const figure = mount.parentElement;
        if (figure === null) return [];
        return [{ el: mount, index: Number(figure.dataset.sketch), title: figure.dataset.title ?? "Sketch", section: sectionOf(el, figure, title) }];
      }),
    );
  }, [root, title, page]);

  // A number at the end of the words each held note quotes, and at the head of the row it names. While the margin is shown the words
  // and the rows of held notes are tinted, and those of notes already sent more quietly. The sketch numbers its own parts.
  useLayoutEffect(() => {
    const el = page.current;
    if (el === null) return;
    const made: HTMLElement[] = [];
    const marked: HTMLElement[] = [];
    const held: Range[] = [];
    const gone: Range[] = [];
    const next: Anchor[] = [];
    const anchor = () => {
      const one = document.createElement("span");
      one.className = "armada-doc__anchor";
      made.push(one);
      return one;
    };
    for (const note of margin ? sent : []) {
      if (note.ref.place.revision !== revision) continue;
      if (note.ref.target.kind === "words") {
        const range = rangeOf(el, note.ref.target.quote);
        if (range !== undefined) gone.push(range);
      } else if (note.ref.target.kind === "row") {
        const row = rowOf(el, note.ref.target.row);
        row?.setAttribute("data-sent", "");
        if (row !== undefined) marked.push(row);
      }
    }
    for (const mark of viewer.marks) {
      if (mark.about.kind !== "document") continue;
      const target = mark.about.target;
      if (target.kind === "words") {
        const range = rangeOf(el, target.quote);
        if (range === undefined) continue;
        if (margin) held.push(range.cloneRange());
        const at = anchor();
        range.collapse(false);
        range.insertNode(at);
        next.push({ n: mark.n, writing: mark.writing, said: mark.text, el: at });
      } else if (target.kind === "row") {
        const row = rowOf(el, target.row);
        const cell = row?.firstElementChild;
        if (row === undefined || cell == null) continue;
        if (margin) {
          row.setAttribute("data-noted", "");
          marked.push(row);
        }
        const at = anchor();
        cell.prepend(at);
        next.push({ n: mark.n, writing: mark.writing, said: mark.text, el: at });
      }
    }
    const registry = highlights();
    registry?.set("armada-doc-note", new Highlight(...held));
    registry?.set("armada-doc-sent", new Highlight(...gone));
    setAnchors(next);
    return () => {
      registry?.delete("armada-doc-note");
      registry?.delete("armada-doc-sent");
      for (const one of made) one.remove();
      for (const row of marked) {
        row.removeAttribute("data-noted");
        row.removeAttribute("data-sent");
      }
      el.normalize();
    };
  }, [viewer.marks, sent, revision, root, mounts, page, margin]);

  // Opened at a place a note named: scrolled to, and held out for a few seconds.
  useEffect(() => {
    const el = page.current;
    if (el === null || focus === undefined) return;
    const place = placeOf(el, focus.target);
    place?.scrollIntoView({ block: "center" });
    const registry = highlights();
    const range = focus.target.kind === "words" ? rangeOf(el, focus.target.quote) : undefined;
    if (range !== undefined) registry?.set("armada-doc-focus", new Highlight(range));
    place?.setAttribute("data-focus", "");
    const timer = window.setTimeout(() => {
      registry?.delete("armada-doc-focus");
      place?.removeAttribute("data-focus");
    }, 4000);
    return () => {
      window.clearTimeout(timer);
      registry?.delete("armada-doc-focus");
      place?.removeAttribute("data-focus");
    };
  }, [focus, mounts, page]);

  // The note the pointer or focus is on in the margin: its words or row take the stronger tint, and the page scrolls to it if it is
  // off screen. Its sketch parts are marked on the mounts below.
  const litNote = margin && lit !== undefined ? notes.find((one) => one.id === lit.id && one.revision === revision) : undefined;
  useLayoutEffect(() => {
    const el = page.current;
    if (el === null || litNote === undefined) return;
    const registry = highlights();
    const range = litNote.target.kind === "words" ? rangeOf(el, litNote.target.quote) : undefined;
    if (range !== undefined) registry?.set("armada-doc-lit", new Highlight(range));
    const place = placeOf(el, litNote.target);
    if (litNote.target.kind !== "words") place?.setAttribute("data-lit", "");
    if (lit?.from === "margin") place?.scrollIntoView({ block: "nearest" });
    return () => {
      registry?.delete("armada-doc-lit");
      place?.removeAttribute("data-lit");
    };
  }, [litNote, lit?.from, viewer.marks, sent, mounts, page]);

  // A note held takes the place of the selection that made it.
  useEffect(() => {
    if (viewer.settled > 0) window.getSelection()?.removeAllRanges();
  }, [viewer.settled]);

  // The parts of a sketch that stand marked: the lit note's alone while one of its own is lit, else those a visit named and those of every note
  // the margin lists.
  const markedIn = (sketch: string): readonly string[] => {
    const parts = (target: DocumentTarget) => (target.kind === "part" && target.sketch === sketch ? target.parts.map((part) => part.id) : []);
    const lit = litNote === undefined ? [] : parts(litNote.target);
    if (lit.length > 0) return lit;
    const asked = focus === undefined ? [] : parts(focus.target);
    return [...new Set([...asked, ...(margin ? notes.filter((one) => one.revision === revision).flatMap((one) => parts(one.target)) : [])])];
  };

  const point = (target: EventTarget | null) => {
    const el = page.current;
    return el === null ? undefined : pointedAt(el, target, title, revision);
  };

  const onContextMenu = (event: MouseEvent<HTMLElement>) => {
    if (!viewer.on || (event.target as Element).closest(".armada-doc__sketch") !== null) return;
    event.preventDefault();
    const hit = point(event.target);
    viewer.menu({ x: event.clientX, y: event.clientY }, hit?.about, hit?.anchor);
  };

  // The pointer over a note's place in the page lights that note in the margin.
  const onMouseMove = (event: MouseEvent<HTMLElement>) => {
    const el = page.current;
    if (el === null || !margin) return;
    const target = event.target as Element;
    const here = (): PageNote | undefined => {
      const part = target.closest(".react-flow__node")?.getAttribute("data-id");
      const sketch = target.closest(".armada-doc__sketch")?.getAttribute("data-title");
      const row = target.closest<HTMLElement>("tbody tr");
      return notes.find((one) => {
        if (one.revision !== revision) return false;
        if (one.target.kind === "part") return part != null && one.target.sketch === sketch && one.target.parts.some((p) => p.id === part);
        if (one.target.kind === "row") return row !== null && rowOf(el, one.target.row) === row;
        return [...(rangeOf(el, one.target.quote)?.getClientRects() ?? [])].some((box) => event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom);
      });
    };
    const found = here();
    onLit((was) => (found === undefined ? (was?.from === "page" ? undefined : was) : was?.id === found.id && was.from === "page" ? was : { id: found.id, from: "page" }));
  };
  const onMouseLeave = () => onLit((was) => (was?.from === "page" ? undefined : was));

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    viewer.onKeyDown(event);
    if (event.defaultPrevented || !viewer.on || !(event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey))) return;
    if ((event.target as Element).closest(".armada-doc__sketch") !== null) return;
    event.preventDefault();
    const hit = point(event.target);
    const box = hit?.anchor ?? (event.target as Element).getBoundingClientRect();
    viewer.menu({ x: box.left, y: box.bottom }, hit?.about, hit?.anchor);
  };

  // A link in the page goes where a link goes in Bridge, and an address in the page is scrolled to.
  const onClick = (event: MouseEvent<HTMLElement>) => {
    const link = (event.target as Element).closest("a");
    if (link === null) return;
    event.preventDefault();
    const href = link.getAttribute("href") ?? "";
    if (href.startsWith("#")) page.current?.querySelector(`#${CSS.escape(href.slice(1))}`)?.scrollIntoView({ block: "start" });
    else if (href !== "") void openLink(href);
  };

  return (
    <>
      <div ref={page} className={PAGE_ROOT} role="article" aria-label={title} tabIndex={0} aria-keyshortcuts="Shift+F10 ContextMenu Meta+Enter" onContextMenu={onContextMenu} onMouseMove={onMouseMove} onMouseOver={onMouseMove} onMouseLeave={onMouseLeave} onKeyDown={onKeyDown} onClick={onClick} />
      {mounts.map((one) =>
        createPortal(
          <SketchScene
            scene={sketches[one.index]?.scene}
            {...(against?.[one.index] === undefined ? {} : { against: against[one.index]!.scene })}
            title={one.title}
            notes={scope}
            within={{ page: title, revision, section: one.section }}
            marked={markedIn(one.title)}
          />,
          one.el,
          String(one.index),
        ),
      )}
      {anchors.map((one) => createPortal(<NoteNumber n={one.n} writing={one.writing} said={one.said} />, one.el, `${String(one.n)}:${String(one.writing)}`))}
      {viewer.layer}
    </>
  );
}
