// A page's words drawn into one element by hand, its CSS adopted under that element's class and the real sketch drawn at each
// `<armada-sketch>`. The reader (`Document.tsx`) draws it whole; a thread card's well draws it cropped (`PageScreen`).

import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { SketchScene } from "./kit/components/compositions/SketchScene/SketchScene";
import { PAGE_ROOT, readPage, sceneWithKept } from "./kit/screens/document-page";
import { Skeleton } from "./kit/components/primitives/Skeleton/Skeleton";

import { readShown, type Shown } from "./jobState";
import { WithKeptStyles } from "./keptStyles";
import type { Machine } from "./machines";

type Mount = {
  el: HTMLElement;
  index: number;
  title: string;
};

/** One heading of a page, for the reader's contents. */
export type Head = {
  id: string;
  level: number;
  text: string;
};

/** One note on the words of a page: what was selected, the block it sits in, and what he wrote. */
export type Note = {
  id: string;
  quote: string;
  block: number;
  text: string;
};

/** The blocks a note can sit under, in the order the page holds them: its paragraphs, items, headings, quotes and cells. */
const BLOCKS = "p, li, h1, h2, h3, h4, blockquote, td, th, pre";
const blocksOf = (root: HTMLElement): HTMLElement[] =>
  [
    ...root.querySelectorAll<HTMLElement>(BLOCKS),
  ].filter((one) => one.closest(".armada-doc__sketch-mount") === null);

/** The page's text, read from the Mac: the kept revision at `path`. `undefined` until it comes. */
export function usePageText(
  machine: Machine | undefined,
  id: string,
  path: string | undefined,
): Shown | undefined {
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  useEffect(() => {
    if (machine === undefined || path === undefined) return;
    let gone = false;
    setShown(undefined);
    void readShown(machine, id, path).then((got) => !gone && setShown(got));
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    id,
    path,
  ]);
  return shown;
}

/** The page's words. `onHeads` is told the headings it holds, in order, for the reader's contents. */
export function PageBody({
  text,
  title,
  keptStyles,
  onHeads,
  notes,
  renderNote,
  onSelect,
}: {
  text: string;
  title: string;
  keptStyles?: Readonly<Record<string, string>> | undefined;
  onHeads?: (heads: readonly Head[]) => void;
  /** His notes, each drawn by `renderNote` under the block it points at. */
  notes?: readonly Note[];
  renderNote?: (note: Note) => ReactNode;
  /** Told what he has selected in the words, or that nothing is. */
  onSelect?: (picked: Pick<Note, "quote" | "block"> | undefined) => void;
}) {
  const parsed = useMemo(
    () => readPage(text),
    [
      text,
    ],
  );
  const el = useRef<HTMLDivElement>(null);
  const [mounts, setMounts] = useState<readonly Mount[]>([]);
  const scenes = useMemo(
    () => parsed.sketches.map((one) => sceneWithKept(one.title, one.scene, keptStyles)),
    [
      parsed,
      keptStyles,
    ],
  );

  // The page's CSS reaches only the page: every rule was put under the page's class when it was read.
  useEffect(() => {
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(parsed.css);
    document.adoptedStyleSheets = [
      ...document.adoptedStyleSheets,
      sheet,
    ];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((one) => one !== sheet);
    };
  }, [
    parsed,
  ]);

  useLayoutEffect(() => {
    const into = el.current;
    if (into === null) return;
    into.replaceChildren(
      ...[
        ...parsed.root.childNodes,
      ].map((one) => one.cloneNode(true)),
    );
    // An address outside the page opens beside the app and not over it.
    for (const link of into.querySelectorAll("a[href^='http']")) {
      link.setAttribute("target", "_blank");
      link.setAttribute("rel", "noopener noreferrer");
    }
    // A table takes the page's full width, and scrolls inside its own box where it is wider than the screen.
    for (const table of into.querySelectorAll("table")) {
      if (table.closest(".armada-doc__sketch-mount") !== null) continue;
      const box = document.createElement("div");
      box.className = "pk-doc__scroll";
      box.tabIndex = 0;
      box.setAttribute("role", "region");
      box.setAttribute("aria-label", table.querySelector("caption")?.textContent ?? "Table");
      table.replaceWith(box);
      box.append(table);
    }
    onHeads?.(
      [
        ...into.querySelectorAll<HTMLElement>("h1, h2, h3"),
      ].map((head, at) => {
        head.id = head.id === "" ? `pk-head-${String(at)}` : head.id;
        return {
          id: head.id,
          level: Number(head.tagName.slice(1)),
          text: head.textContent ?? "",
        };
      }),
    );
    setMounts(
      [
        ...into.querySelectorAll<HTMLElement>(".armada-doc__sketch-mount"),
      ].flatMap((mount) => {
        const figure = mount.parentElement;
        return figure === null
          ? []
          : [
              {
                el: mount,
                index: Number(figure.dataset.sketch),
                title: figure.dataset.title ?? "Sketch",
              },
            ];
      }),
    );
  }, [
    parsed,
  ]);

  // His notes stand under the blocks they point at, each in a place of its own the note's card is drawn into.
  const [homes, setHomes] = useState<
    readonly {
      id: string;
      el: HTMLElement;
    }[]
  >([]);
  const held = (notes ?? []).map((one) => `${one.id}:${String(one.block)}`).join(",");
  useLayoutEffect(() => {
    const into = el.current;
    if (into === null) return;
    for (const old of into.querySelectorAll(".pk-note-home")) old.remove();
    const blocks = blocksOf(into);
    setHomes(
      (notes ?? []).flatMap((one) => {
        const block = blocks[one.block];
        if (block === undefined) return [];
        const home = document.createElement("div");
        home.className = "pk-note-home";
        (block.closest("tr")?.closest("table") ?? block.closest("li") ?? block).after(home);
        return [
          {
            id: one.id,
            el: home,
          },
        ];
      }),
    );
  }, [
    parsed,
    held,
  ]);

  // What he selects in the words, as a quote and the block it starts in.
  useEffect(() => {
    if (onSelect === undefined) return;
    const read = () => {
      const into = el.current;
      const selection = window.getSelection();
      if (
        into === null ||
        selection === null ||
        selection.rangeCount === 0 ||
        selection.isCollapsed ||
        !into.contains(selection.anchorNode)
      )
        return onSelect(undefined);
      const quote = selection.toString().replace(/\s+/g, " ").trim();
      const start =
        selection.anchorNode instanceof Element
          ? selection.anchorNode
          : selection.anchorNode?.parentElement;
      const block = start?.closest(BLOCKS);
      const at =
        block === null || block === undefined ? -1 : blocksOf(into).indexOf(block as HTMLElement);
      onSelect(
        quote === "" || at < 0
          ? undefined
          : {
              quote,
              block: at,
            },
      );
    };
    document.addEventListener("selectionchange", read);
    return () => document.removeEventListener("selectionchange", read);
  }, [
    onSelect,
  ]);

  // A link in the page scrolls to an address in it. Any other address is left alone.
  const onClick = (event: MouseEvent<HTMLElement>) => {
    const link = (event.target as Element).closest("a");
    if (link === null) return;
    const href = link.getAttribute("href") ?? "";
    if (!href.startsWith("#")) return;
    event.preventDefault();
    el.current?.querySelector(`#${CSS.escape(href.slice(1))}`)?.scrollIntoView({
      block: "start",
    });
  };
  return (
    <>
      <div
        ref={el}
        className={`${PAGE_ROOT} pk-doc__page`}
        role="article"
        aria-label={title}
        onClick={onClick}
      />
      {mounts.map((one) =>
        createPortal(
          <SketchScene scene={scenes[one.index]} title={one.title} />,
          one.el,
          String(one.index),
        ),
      )}
      {renderNote === undefined
        ? null
        : homes.map((home) => {
            const note = notes?.find((one) => one.id === home.id);
            return note === undefined ? null : createPortal(renderNote(note), home.el, home.id);
          })}
    </>
  );
}

/** A page's first screen, for a card's well: its words as they are, cut by the well. Shapes while it comes. */
export function PageScreen({
  machine,
  id,
  path,
  title,
  keptStyles,
}: {
  machine: Machine;
  id: string;
  path: string | undefined;
  title: string;
  keptStyles?: Readonly<Record<string, string>> | undefined;
}) {
  const shown = usePageText(machine, id, path);
  if (shown === undefined)
    return (
      <div className="pk-bc__shapes" role="status" aria-label="Loading">
        <Skeleton width="40%" />
        <Skeleton />
        <Skeleton width="85%" />
        <Skeleton width="60%" />
      </div>
    );
  if (shown.kind !== "text") return null;
  return (
    <WithKeptStyles machine={machine}>
      <PageBody text={shown.text} title={title} keptStyles={keptStyles} />
    </WithKeptStyles>
  );
}
