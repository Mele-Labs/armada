// A `mock` node: the HTML of a part of an app, drawn in a sealed frame. **The frame is `sandbox="allow-same-origin"` and nothing
// else**, so no script runs while Bridge can still reach its document to build it. Bridge's content policy is inherited and is
// not widened: nothing is written as an inline `<style>` or loaded from a `data:` address. The HTML becomes nodes of the frame's
// own document, the sheets are constructed with the frame window's `CSSStyleSheet` and adopted, and a font is added from its bytes.

import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { NoteNumber } from "../ViewerNotes/ViewerNotes";
import { elementOfKey, mockNodes, splitPart } from "./mock-frame";
import type { SceneNode } from "./scene";

export type MockFont = { family: string; weight: string; style?: string; bytes: ArrayBuffer | Uint8Array };

/** What the host read for a `mock`: sheet texts in the order they apply, fonts as bytes, and the files it was drawn with. */
export type MockStyles = {
  css: readonly string[];
  fonts: readonly MockFont[];
  /** Repository paths of the stylesheet files `css` came from. */
  drawnWith: readonly string[];
  /** Attributes set on the frame's root element, for a stylesheet that switches on one. */
  root?: Readonly<Record<string, string>>;
};

/** The host's answer for a node's `styles`, given the node's id and the paths the agent named. */
export type ResolveMockStyles = (node: { id: string; styles: readonly string[] }) => Promise<MockStyles>;

const NONE: MockStyles = { css: [], fonts: [], drawnWith: [] };
const SHELL = "<!doctype html><html><head><meta charset=\"utf-8\"></head><body></body></html>";
export const NO_STYLESHEET = "No stylesheet of the app";

/** The elements to ring, by part id, with the notes each carries: none for one that is only picked. */
export type MockNotes = readonly { id: string; notes: readonly { n: number; writing: boolean; said: string }[] }[];

type Ring = MockNotes[number] & { left: number; top: number; width: number; height: number };

const nameOf = (path: string) => path.slice(path.lastIndexOf("/") + 1);

export function MockView({ node, resolve, parts }: { node: SceneNode; resolve: ResolveMockStyles | undefined; parts: MockNotes }) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [doc, setDoc] = useState<Document | undefined>(undefined);
  const [drawn, setDrawn] = useState<MockStyles | undefined>(undefined);
  const [built, setBuilt] = useState(0);
  const [height, setHeight] = useState<number | undefined>(undefined);
  const [rings, setRings] = useState<readonly Ring[]>([]);
  const styles = node.styles;
  const stylesKey = (styles ?? []).join("\n");

  // What the host reads for this node. With no host, the frame is drawn with the node's own rules alone.
  useEffect(() => {
    let live = true;
    if (resolve === undefined) {
      setDrawn(NONE);
      return undefined;
    }
    setDrawn(undefined);
    resolve({ id: node.id, styles: styles ?? [] }).then(
      (read) => live && setDrawn(read),
      () => live && setDrawn(NONE),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `stylesKey` stands for `styles`
  }, [resolve, node.id, stylesKey]);

  // The frame, built: markup, then sheets, then fonts, and its height read once they are in.
  useEffect(() => {
    const win = frame.current?.contentWindow;
    if (doc === undefined || drawn === undefined || win == null) return undefined;
    let live = true;
    doc.body.replaceChildren(...mockNodes(doc, node.html ?? ""));
    for (const [name, value] of Object.entries(drawn.root ?? {})) doc.documentElement.setAttribute(name, value);
    const sheets = [...drawn.css, ...(node.css === undefined ? [] : [node.css])].map((text) => {
      const sheet = new (win as Window & typeof globalThis).CSSStyleSheet();
      sheet.replaceSync(text);
      return sheet;
    });
    doc.adoptedStyleSheets = sheets;
    const Face = (win as Window & typeof globalThis).FontFace;
    void Promise.all(
      drawn.fonts.map(async (font) => {
        const face = new Face(font.family, font.bytes as BufferSource, { weight: font.weight, ...(font.style === undefined ? {} : { style: font.style }) });
        await face.load();
        doc.fonts.add(face);
      }),
    ).then(
      () => live && setBuilt((was) => was + 1),
      () => live && setBuilt((was) => was + 1),
    );
    setBuilt((was) => was + 1);
    return () => {
      live = false;
    };
  }, [doc, drawn, node.html, node.css]);

  // Its height follows its content: read when built and whenever the content resizes, which a font arriving does.
  useEffect(() => {
    const win = frame.current?.contentWindow as (Window & typeof globalThis) | null | undefined;
    if (doc === undefined || win == null) return undefined;
    const read = () => setHeight(Math.ceil(doc.documentElement.getBoundingClientRect().height));
    read();
    const watch = new win.ResizeObserver(read);
    watch.observe(doc.documentElement);
    return () => watch.disconnect();
  }, [doc, built]);

  // Where each noted element stands in the frame, for its ring and number.
  useLayoutEffect(() => {
    if (doc === undefined) return;
    setRings(
      parts.flatMap((one) => {
        const key = splitPart(one.id)?.key;
        const el = key === undefined ? undefined : elementOfKey(doc, key);
        if (el === undefined) return [];
        const at = el.getBoundingClientRect();
        return [{ ...one, left: at.left, top: at.top, width: at.width, height: at.height }];
      }),
    );
  }, [doc, built, height, parts]);

  const files = drawn?.drawnWith ?? [];
  return (
    <>
      {node.title === undefined ? null : <span className="armada-scene-node__caption">{node.title}</span>}
      <div className="armada-mock__stage">
        <iframe
          ref={frame}
          className="armada-mock__frame"
          title={node.title ?? "Mock"}
          sandbox="allow-same-origin"
          srcDoc={SHELL}
          tabIndex={-1}
          onLoad={(event) => setDoc(event.currentTarget.contentDocument ?? undefined)}
          {...(height === undefined ? {} : { style: { height } })}
        />
        {rings.map((one) => (
          <span key={one.id} className="armada-mock__ring" style={{ left: one.left, top: one.top, width: one.width, height: one.height }}>
            {one.notes.length === 0 ? null : (
              <span className="armada-mock__numbers">
                {one.notes.map((note) => (
                  <NoteNumber key={note.n} n={note.n} writing={note.writing} said={note.said} />
                ))}
              </span>
            )}
          </span>
        ))}
      </div>
      {drawn === undefined ? null : files.length === 0 ? (
        <span className="armada-mock__with">{NO_STYLESHEET}</span>
      ) : (
        <Tooltip
          label={files.map((path) => (
            <span key={path} className="armada-mock__path">
              {path}
            </span>
          ))}
        >
          <span className="armada-mock__with">{files.map(nameOf).join(", ")}</span>
        </Tooltip>
      )}
    </>
  );
}
