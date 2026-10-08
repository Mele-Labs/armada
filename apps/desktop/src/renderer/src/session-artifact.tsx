// What an Artifacts row of a Session's ledger opens in the side panel: a picture, a file's text, or
// a page in a web view. The panel itself is `ReadingSheet`'s; this draws the body.
//
// **A page is never run in the renderer.** Bridge refuses navigation, so a real window asks main for
// a view over this body's rect (`session-page.ts`) and keeps it there while the panel is open. The
// mock has no second view and draws a frame of its own page in its place.

import { useEffect, useRef, useState } from "react";
import { AppWindow, Files, Globe, Image, NotebookText } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Prose } from "@armada/components";
import type { ArtifactRead, PageBounds, SessionsDraft } from "@armada/screens/src/draft/sessions";

type ArtifactForm = "page" | "file" | "image" | "doc" | "window";

export const ARTIFACT_GLYPH: Record<ArtifactForm, LucideIcon> = { page: Globe, file: Files, image: Image, doc: NotebookText, window: AppWindow };

/** Whether a form is shown in a web view and has an address to open in the browser. */
export const isAddress = (form: ArtifactForm): boolean => form === "page" || form === "doc";

const MB = 1024 * 1024;

const refusedSaid = (read: Extract<ArtifactRead, { ok: false }>): string =>
  read.why === "too_big" ? `Larger than ${Math.round((read.limit ?? 0) / MB)} MB` : read.why === "binary" ? "Not text" : "Could not be read";

type Held = { state: "reading" } | { state: "read"; read: ArtifactRead };

/** A file the ledger names, read once for this open. */
function useArtifact(draft: SessionsDraft | undefined, sessionId: string, path: string): Held {
  const [held, setHeld] = useState<{ path: string; read: ArtifactRead } | undefined>();
  const read = draft?.readArtifact;
  useEffect(() => {
    let current = true;
    void (read?.(sessionId, path) ?? Promise.resolve<ArtifactRead>({ ok: false, why: "unreadable" })).then((answer) => {
      if (current) setHeld({ path, read: answer });
    });
    return () => {
      current = false;
    };
  }, [read, sessionId, path]);
  return held?.path === path ? { state: "read", read: held.read } : { state: "reading" };
}

function Picture({ bytes, type, label }: { bytes: Uint8Array; type: string; label: string }) {
  const [src, setSrc] = useState<string | undefined>();
  useEffect(() => {
    const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type }));
    setSrc(url);
    return () => URL.revokeObjectURL(url);
  }, [bytes, type]);
  return src === undefined ? null : <img className="armada-session-artifact__picture" src={src} alt={label} />;
}

function Contents({ read, label }: { read: ArtifactRead; label: string }) {
  if (!read.ok) return <p className="armada-session-artifact__refused">{refusedSaid(read)}</p>;
  if (read.type.startsWith("image/")) return <Picture bytes={read.bytes} type={read.type} label={label} />;
  const text = new TextDecoder().decode(read.bytes);
  return read.type === "text/markdown" ? (
    <div className="armada-session-reading">
      <Prose text={text} />
    </div>
  ) : (
    <pre className="armada-session-artifact__text">{text}</pre>
  );
}

const sameBox = (a: PageBounds | undefined, b: PageBounds): boolean =>
  a !== undefined && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/** The mock itself on another scenario, which is the one address a browser can serve as a page to show. */
function mockPage(): string {
  const url = new URL(window.location.href);
  url.search = "?scenario=every-state&walked";
  return url.toString();
}

/**
 * The body a page's view lies over. **Reported on every frame it moves**, since the panel travels
 * in and is dragged wider and no resize event says so; main is only told when the rect changed.
 */
function PageFrame({ draft, sessionId, address, label, onClose }: { draft: SessionsDraft | undefined; sessionId: string; address: string; label: string; onClose: () => void }) {
  const body = useRef<HTMLDivElement>(null);
  const page = draft?.page;
  useEffect(() => {
    if (page === undefined) return;
    let frame = 0;
    let last: PageBounds | undefined;
    const follow = () => {
      const box = body.current?.getBoundingClientRect();
      if (box !== undefined) {
        const now = { x: box.x, y: box.y, width: box.width, height: box.height };
        if (!sameBox(last, now)) {
          if (last === undefined) page.show(sessionId, address, now);
          else page.move(now);
          last = now;
        }
      }
      frame = requestAnimationFrame(follow);
    };
    frame = requestAnimationFrame(follow);
    return () => {
      cancelAnimationFrame(frame);
      page.hide();
    };
  }, [page, sessionId, address]);
  // Esc in the view, which holds the keyboard where the panel's own binding cannot hear it.
  useEffect(() => page?.onEscape(onClose), [page, onClose]);
  const testing = (import.meta as { env?: { MODE?: string } }).env?.MODE === "test";
  return (
    <div ref={body} className="armada-session-artifact__page" role="region" aria-label="Page">
      {page === undefined ? <iframe title={label} src={testing ? "about:blank" : mockPage()} /> : null}
    </div>
  );
}

/** The panel's body for one artifact of the ledger. */
export function ArtifactBody({
  draft,
  sessionId,
  form,
  id,
  title,
  onClose,
}: {
  draft: SessionsDraft | undefined;
  sessionId: string;
  form: ArtifactForm;
  id: string;
  title: string;
  onClose: () => void;
}) {
  if (isAddress(form)) return <PageFrame draft={draft} sessionId={sessionId} address={id} label={title} onClose={onClose} />;
  return <FileBody draft={draft} sessionId={sessionId} id={id} title={title} />;
}

function FileBody({ draft, sessionId, id, title }: { draft: SessionsDraft | undefined; sessionId: string; id: string; title: string }) {
  const held = useArtifact(draft, sessionId, id);
  return held.state === "reading" ? null : <Contents read={held.read} label={title} />;
}
