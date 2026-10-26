// A Job's document, read full width over the Job. Contents on the left (the page's h2s), the page in the middle, the Notes margin on the
// right, and a head that holds the revision, Changes, Share and Back. **Reading and replying are one place**: a note is made on the page
// itself (the shared note box), held with Enter, and sent with ⌘Enter from the page or the margin as one message to the Job. The Job's
// next revision is the reply: Changes marks what moved on, and each sent note shows the change that answers it. The thread and the
// Landing under the layer are not unmounted, so Esc gives the Job back exactly as it was. `docs/concepts/session/noting-what-you-read.md`.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { ExternalLink, FileDiff, X } from "lucide-react";
import { Button, DropdownMenu, KbdBinding, NoteNumber, Tooltip, aboutSaid, notedOf, setHeldOn, useHeldOn, useViewerNotes, type DropdownMenuEntry, type NotedRef } from "@armada/components";
import { popupIsOpen } from "@armada/screens/src/list-keyboard";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { DocumentPage, type DocumentFocus, type SentNote } from "./DocumentPage";
import { diffPages } from "./document-diff";
import { placeOf } from "./document-find";
import { readPage, type Page } from "./document-page";
import { clearVisit, documentOf, useDocumentVisit } from "./document-state";
import { closeLayer } from "./layers";
import "./document.css";

/** The pages of a document's revisions, oldest first, each read once as it arrives. A revision not read yet is not in the list. */
function useRevisions(draft: SessionsDraft | undefined, session: string, paths: readonly string[]): Page[] {
  const cache = useRef(new Map<string, Page>());
  const [version, setVersion] = useState(0);
  const key = paths.join("|");
  useEffect(() => {
    let live = true;
    const missing = key.split("|").filter((one) => one !== "" && !cache.current.has(one));
    if (missing.length === 0) return undefined;
    void Promise.all(
      missing.map(async (path) => {
        const read = await draft?.readArtifact?.(session, path);
        if (read?.ok === true) cache.current.set(path, readPage(new TextDecoder().decode(read.bytes)));
      }),
    ).then(() => {
      if (live) setVersion((was) => was + 1);
    });
    return () => {
      live = false;
    };
  }, [draft, session, key]);
  return useMemo(() => key.split("|").flatMap((one) => cache.current.get(one) ?? []), [key, version]);
}

const CHANGE_SAID = { added: "Added", changed: "Changed", removed: "Removed" } as const;

const SHARE: DropdownMenuEntry[] = [
  { kind: "item", id: "web", label: "Save as web page" },
  { kind: "item", id: "pdf", label: "Save as PDF" },
  { kind: "item", id: "repo", label: "Add to the repo" },
];

/** The notes of every message the person sent about this page, in the order sent. */
function sentAbout(session: Session, title: string): SentNote[] {
  return session.rows.flatMap((row) => {
    if (row.kind !== "message" || row.from.kind !== "you") return [];
    return (notedOf(row.text) ?? []).flatMap((block): SentNote[] => {
      const ref: NotedRef = block.ref;
      return ref.kind === "document" && ref.place.page === title ? [{ n: block.n, ref, body: block.body }] : [];
    });
  });
}

export function DocumentLayer({ session }: { session: Session }) {
  const draft = useSessionsDraft();
  const doc = documentOf(session);
  const pages = useRevisions(draft, session.id, doc?.revisions ?? []);
  const latest = pages.length;
  const [pinned, setPinned] = useState<number | undefined>(undefined);
  const [changes, setChanges] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [focus, setFocus] = useState<DocumentFocus | undefined>(undefined);
  const [current, setCurrent] = useState<string | undefined>(undefined);
  const [wanted, setWanted] = useState<number | undefined>(undefined);
  const layer = useRef<HTMLElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const pageRef = useRef<HTMLDivElement>(null);
  const focused = useRef(false);

  const revision = Math.min(pinned ?? latest, latest);
  const page = pages[revision - 1];
  const before = pages[revision - 2];
  const title = page?.title ?? doc?.title ?? "Document";
  const scope = { key: `document:${session.id}:${doc?.id ?? ""}`, job: session.id };
  const viewer = useViewerNotes(scope, pageRef);
  const held = useHeldOn(scope.key).notes;

  const diff = useMemo(() => (page === undefined || before === undefined ? undefined : diffPages(before.root, page.root, title)), [page, before, title]);
  const showing = changes && diff !== undefined;
  const sent = useMemo(() => sentAbout(session, title), [session.rows, title]);
  const answers = useMemo(() => sent.map((note) => (diff !== undefined && note.ref.place.revision === revision - 1 ? diff.answer(note.ref) : undefined)), [sent, diff, revision]);

  // Opened from a sent note in the thread: on the revision the note was made on, at the words, the row or the sketch it named.
  const visit = useDocumentVisit(session.id);
  useEffect(() => {
    if (visit === undefined || latest === 0) return;
    setPinned(visit.place.revision < latest ? visit.place.revision : undefined);
    setFocus({ target: visit.target, at: visit.at });
    clearVisit(session.id);
  }, [visit, latest, session.id]);

  // Focus goes into the layer once there is a page: the first section of the contents.
  useEffect(() => {
    if (page === undefined || focused.current) return;
    focused.current = true;
    layer.current?.querySelector<HTMLElement>("[data-rail] button")?.focus();
  }, [page]);

  // Esc is Back, unless a note box or a menu is open over the layer: that one's Esc is its own.
  useEffect(() => {
    const listen = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || document.querySelector('[role="dialog"], [role="alertdialog"]') !== null || popupIsOpen()) return;
      event.preventDefault();
      closeLayer();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);

  // A change the margin asked for is scrolled to once the page that holds it is drawn.
  useEffect(() => {
    if (wanted === undefined || !showing) return;
    pageRef.current?.querySelector(`[data-change-id="${String(wanted)}"]`)?.scrollIntoView({ block: "center" });
    setWanted(undefined);
  }, [wanted, showing, page]);

  const toSection = (id: string) => {
    pageRef.current?.querySelector(`#${CSS.escape(id)}`)?.scrollIntoView({ block: "start" });
    setCurrent(id);
  };
  const onScroll = () => {
    const top = (scroller.current?.getBoundingClientRect().top ?? 0) + 24;
    let found: string | undefined;
    for (const heading of Array.from(pageRef.current?.querySelectorAll("h2") ?? [])) {
      if (heading.getBoundingClientRect().top > top) break;
      found = heading.id;
    }
    setCurrent(found);
  };
  const toChange = (id: number) => {
    setChanges(true);
    setWanted(id);
  };
  const switchRevision = () => setPinned(revision === latest ? latest - 1 : undefined);

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement || target.isContentEditable || target.closest("[role=menu]") !== null) return;
    if (target.closest("[data-rail]") !== null && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      const stops = [...(layer.current?.querySelectorAll<HTMLElement>("[data-rail] button") ?? [])];
      const to = stops[Math.min(stops.length - 1, Math.max(0, stops.indexOf(target) + (event.key === "ArrowDown" ? 1 : -1)))];
      event.preventDefault();
      to?.focus();
      return;
    }
    viewer.onKeyDown(event);
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
    const key = event.key.toLowerCase();
    if (key === "c" && diff !== undefined) setChanges((was) => !was);
    else if (key === "r" && latest > 1) switchRevision();
    else if (key === "s") setSharing(true);
    else return;
    event.preventDefault();
  };

  const share = (id: string) => {
    // Saving is not wired in the mock. Adding to the repo asks the Job, as a person would.
    if (id === "repo" && doc !== undefined) void draft?.send(session.id, { text: `Commit ${doc.id} to the repo.`, files: [], sketches: [], tags: [] });
  };

  const revisions: DropdownMenuEntry[] = pages.map((_, at) => ({ kind: "item", id: String(at + 1), label: `Revision ${String(at + 1)}`, selected: at + 1 === revision }));

  return (
    <section ref={layer} className="armada-doc" aria-label="Document" data-keeps-escape="" data-owns-keys="c r s" onKeyDown={onKeyDown}>
      <header className="armada-doc__head">
        <h3 className="armada-doc__title">{title}</h3>
        {page === undefined ? null : latest > 1 ? (
          <DropdownMenu plain triggerLabel={`Revision ${String(revision)}`} triggerTooltip="Revision" entries={revisions} onSelect={(id) => setPinned(Number(id) === latest ? undefined : Number(id))} />
        ) : (
          <span className="armada-doc__revision">Revision {revision}</span>
        )}
        <div className="armada-doc__tools">
          {diff === undefined ? null : (
            <Tooltip label="Changes" shortcut="C">
              <Button variant="ghost" size="sm" iconOnly aria-label="Changes" aria-pressed={showing} onClick={() => setChanges((was) => !was)}>
                <FileDiff size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
          <DropdownMenu icon={ExternalLink} triggerLabel="Share" align="end" entries={SHARE} open={sharing} onOpenChange={setSharing} onSelect={share} />
          <Tooltip label="Back" shortcut="Esc">
            <Button variant="ghost" size="sm" iconOnly aria-label="Back" onClick={closeLayer}>
              <X size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        </div>
      </header>
      {page === undefined ? null : (
        <div className="armada-doc__body">
          <nav className="armada-doc__rail" aria-label="Contents" data-rail="">
            {page.sections.map((one) => (
              <button key={one.id} type="button" className="armada-doc__section" aria-current={current === one.id ? "location" : undefined} onClick={() => toSection(one.id)}>
                {one.title}
              </button>
            ))}
          </nav>
          <div ref={scroller} className="armada-doc__scroll" onScroll={onScroll}>
            <DocumentPage
              title={title}
              revision={revision}
              root={showing ? diff.root : page.root}
              css={page.css}
              sketches={page.sketches}
              against={showing ? before?.sketches : undefined}
              scope={scope}
              viewer={viewer}
              page={pageRef}
              sent={sent}
              focus={focus}
            />
          </div>
          <aside className="armada-doc__notes" aria-label="Notes on the page">
            <header className="armada-doc__notes-head">
              <h4 className="armada-doc__label">Notes</h4>
              <Button variant="primary" size="sm" disabled={held.length === 0} onClick={viewer.send}>
                Send
                <KbdBinding binding="⌘↵" />
              </Button>
            </header>
            {held.length === 0 ? null : (
              <ol className="armada-doc__list" aria-label="Notes held">
                {held.map((note, index) => (
                  <li key={`${String(index)}:${note.text}`} className="armada-doc__note">
                    <NoteNumber n={index + 1} />
                    <button type="button" className="armada-doc__note-body" onClick={() => note.about.kind === "document" && pageRef.current !== null && placeOf(pageRef.current, note.about.target)?.scrollIntoView({ block: "center" })}>
                      <span className="armada-doc__about">{aboutSaid(note.about)}</span>
                      <span>{note.text}</span>
                    </button>
                    <Tooltip label="Remove">
                      <Button variant="ghost" size="sm" iconOnly aria-label={`Remove note ${String(index + 1)}`} onClick={() => setHeldOn(scope.key, (was) => ({ ...was, notes: was.notes.filter((_, one) => one !== index) }))}>
                        <X size={16} strokeWidth={2} aria-hidden />
                      </Button>
                    </Tooltip>
                  </li>
                ))}
              </ol>
            )}
            {sent.length === 0 ? null : (
              <ol className="armada-doc__list" aria-label="Notes sent">
                {sent.map((note, index) => {
                  const change = answers[index];
                  return (
                    <li key={`${String(index)}:${note.body}`} className="armada-doc__note" data-sent="">
                      <NoteNumber n={note.n} />
                      <button type="button" className="armada-doc__note-body" onClick={() => pageRef.current !== null && placeOf(pageRef.current, note.ref.target)?.scrollIntoView({ block: "center" })}>
                        <span className="armada-doc__about">{aboutSaid({ kind: "document", place: note.ref.place, target: note.ref.target })}</span>
                        <span>{note.body}</span>
                      </button>
                      {change === undefined ? null : (
                        <button type="button" className="armada-doc__answer" onClick={() => toChange(change.id)}>
                          <Tooltip label={`${CHANGE_SAID[change.kind]} in revision ${String(revision)}`}>
                            <span className="armada-doc__answer-mark" role="img" aria-label={`${CHANGE_SAID[change.kind]} in revision ${String(revision)}`}>
                              <FileDiff size={12} strokeWidth={2} aria-hidden />
                            </span>
                          </Tooltip>
                          <span className="armada-doc__answer-said">{change.said}</span>
                        </button>
                      )}
                    </li>
                  );
                })}
              </ol>
            )}
          </aside>
        </div>
      )}
    </section>
  );
}
