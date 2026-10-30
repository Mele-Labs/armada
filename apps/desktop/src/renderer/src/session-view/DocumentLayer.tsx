// A Job's document, read in a floating sheet over the Job. Contents (the page's h2s) and the Notes stand as two cards at the left, the
// page beside them, and the sheet's head holds the revision, Changes, the Notes toggle, Share and Close. **Reading and replying are one
// place**: a note is made on the page itself (the shared note box), held with Enter, and sent with ⌘Enter from the page or the Notes as
// one message to the Job. The Job's next revision is the reply: Changes marks what moved on, and each sent note shows the change that
// answers it. The thread and the Landing under the sheet are not unmounted, so Esc gives the Job back exactly as it was.
// **While the newest revision waits on him**, Approve and Send back head the left column, the thread card's own.
// `docs/concepts/session/noting-what-you-read.md`.

import { useContext, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { ExternalLink, FileDiff, PanelRightClose, PanelRightOpen, X } from "lucide-react";
import { ApprovalAnswers, Button, DropdownMenu, KbdBinding, NoteNumber, RevisionMenu, Sheet, SketchShareContext, Tooltip, aboutSaid, notedOf, savedSaid, setHeldOn, sketchFileName, useHeldOn, useViewerNotes, type DropdownMenuEntry, type NotedRef } from "@armada/components";
import { useAtFloor } from "@armada/shell";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useRevisionReads } from "../revision-reads";
import { useSessionsDraft } from "../sessions-draft";
import { documentAnswerOf } from "../session-thread-rows";
import { DocumentPage, type DocumentFocus, type LitNote, type PageNote, type SentNote } from "./DocumentPage";
import { diffPages } from "./document-diff";
import { placeOf } from "./document-find";
import { readPage, type Page } from "./document-page";
import { savedPage, valuesOf } from "./document-save";
import { clearVisit, documentOf, useDocumentVisit } from "./document-state";
import { closeLayer, useLayerPage } from "./layers";
import "./document.css";

/** The pages of a document's revisions, oldest first. A revision not read yet is not in the list. */
function useRevisions(draft: SessionsDraft | undefined, session: string, paths: readonly string[]): Page[] {
  const read = useRevisionReads(draft, session, paths, readPage);
  return useMemo(() => read.flatMap((one) => one ?? []), [read]);
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

/** Whether the Notes margin is shown: hidden once, it stays hidden each time a page opens, until shown again or the window reloads. */
let marginShown = true;

export function DocumentLayer({ session }: { session: Session }) {
  const draft = useSessionsDraft();
  const doc = documentOf(session, useLayerPage(session.id));
  const pages = useRevisions(draft, session.id, doc?.revisions ?? []);
  const latest = pages.length;
  const [pinned, setPinned] = useState<number | undefined>(undefined);
  const [changes, setChanges] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [revising, setRevising] = useState(false);
  // The head's menus open into the sheet: the head's controls clip a panel drawn in place, and one drawn on the body lies under the scrim.
  const [menus, setMenus] = useState<HTMLElement | null>(null);
  const floor = useAtFloor();
  const [focus, setFocus] = useState<DocumentFocus | undefined>(undefined);
  const [current, setCurrent] = useState<string | undefined>(undefined);
  const [wanted, setWanted] = useState<number | undefined>(undefined);
  const [margin, setMargin] = useState(marginShown);
  const [lit, setLit] = useState<LitNote | undefined>(undefined);
  const layer = useRef<HTMLElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const pageRef = useRef<HTMLDivElement>(null);
  const focused = useRef(false);
  const exporters = useRef<{ current: (() => string | undefined) | null }[]>([]);
  const shared = useContext(SketchShareContext);

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
  // Every note the margin lists, held then sent, by the id its row and its place in the page share.
  const notes = useMemo<PageNote[]>(
    () => [
      ...held.flatMap((note, index): PageNote[] => (note.about.kind === "document" ? [{ id: `held:${String(index)}`, revision: note.about.place.revision, target: note.about.target }] : [])),
      ...sent.map((note, index): PageNote => ({ id: `sent:${String(index)}`, revision: note.ref.place.revision, target: note.ref.target })),
    ],
    [held, sent],
  );
  const lights = (id: string) => ({
    onMouseEnter: () => setLit({ id, from: "margin" }),
    onMouseLeave: () => setLit((was) => (was?.id === id ? undefined : was)),
    onFocus: () => setLit({ id, from: "margin" }),
    onBlur: () => setLit((was) => (was?.id === id ? undefined : was)),
  });
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
  // Hiding the Notes while focus is in them hands focus to the page, so the sheet's keys keep working.
  const toggleMargin = () => {
    if (layer.current?.querySelector(".armada-doc__notes")?.contains(document.activeElement) === true) pageRef.current?.focus();
    setLit(undefined);
    marginShown = !margin;
    setMargin(marginShown);
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
    else if (key === "m") toggleMargin();
    else if (key === "s") setSharing(true);
    else return;
    event.preventDefault();
  };

  // The page as one file, with each sketch drawn into it as the sketch pane draws it, saved where the person says.
  const save = async (kind: "html" | "pdf") => {
    if (page === undefined || pageRef.current === null) return;
    const name = sketchFileName(title, kind);
    // The sketches are read as drawn, so Changes is put away for the save, and the file holds the revision without its marks.
    const marked = showing;
    if (marked) {
      setChanges(false);
      await new Promise((drawn) => requestAnimationFrame(() => requestAnimationFrame(drawn)));
    }
    try {
      const html = savedPage(page, exporters.current.map((one) => one?.current?.()), valuesOf(page, pageRef.current));
      const saved = await window.armada.saveDocument({ kind, name, html });
      const said = savedSaid(name, saved);
      if (said !== undefined) shared.said?.(said);
    } catch {
      shared.said?.(`${name} was not saved.`);
    }
    if (marked) setChanges(true);
  };

  const share = (id: string) => {
    if (id === "web") void save("html");
    else if (id === "pdf") void save("pdf");
    // Adding to the repo asks the Job, as a person would. The page is ignored by git, so the commit forces it in.
    else if (id === "repo" && doc !== undefined) void draft?.send(session.id, { text: `Commit ${doc.id} to the repo. It is gitignored, so add it with git add -f.`, files: [], sketches: [], tags: [] });
  };

  const tools = (
    <div className="armada-doc__tools">
      {diff === undefined ? null : (
        <Tooltip label="Changes" shortcut="C">
          <Button variant="ghost" size="sm" iconOnly aria-label="Changes" aria-pressed={showing} onClick={() => setChanges((was) => !was)}>
            <FileDiff size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {page === undefined ? null : (
        <Tooltip label={margin ? "Hide notes" : "Show notes"} shortcut="M">
          <Button variant="ghost" size="sm" iconOnly aria-label={margin ? "Hide notes" : "Show notes"} onClick={toggleMargin}>
            {margin ? <PanelRightClose size={16} strokeWidth={2} aria-hidden /> : <PanelRightOpen size={16} strokeWidth={2} aria-hidden />}
          </Button>
        </Tooltip>
      )}
      <DropdownMenu icon={ExternalLink} triggerLabel="Share" align="end" portal={menus ?? true} entries={SHARE} open={sharing} onOpenChange={setSharing} onSelect={share} />
    </div>
  );
  const revisionMenu =
    page === undefined ? undefined : (
      <RevisionMenu count={latest} revision={revision} portal={menus ?? true} open={revising} onOpenChange={setRevising} onPick={(picked) => setPinned(picked === latest ? undefined : picked)} />
    );

  // Keys are heard around the whole sheet, its head included. A menu open in the head is `under`, so its Esc closes it and not the sheet.
  return (
    <div className="armada-doc" onKeyDown={onKeyDown}>
      <Sheet kind="document" open floating floor={floor} size="widest" bleed title={title} subtitle={revisionMenu} controls={tools} closeLabel="Close" closeBinding="Esc" under={sharing || revising} onClose={closeLayer}>
        <section ref={layer} className="armada-doc__body" aria-label="Document" data-notes={margin ? "" : undefined}>
          {page === undefined ? null : (
            <>
              <div className="armada-doc__side">
                {doc?.waits !== true || draft?.answerDocument === undefined ? null : (
                  <div className="armada-doc__approve">
                    <ApprovalAnswers id={doc.id} keep={`document:${session.id}:${doc.id}:layer`} open onAnswer={(id, answer) => void draft.answerDocument?.(documentAnswerOf(session, id, answer))} />
                  </div>
                )}
                <nav className="armada-doc__rail" aria-label="Contents" data-rail="">
                  {page.sections.map((one) => (
                    <button key={one.id} type="button" className="armada-doc__section" aria-current={current === one.id ? "location" : undefined} onClick={() => toSection(one.id)}>
                      {one.title}
                    </button>
                  ))}
                </nav>
                <aside className="armada-doc__notes" aria-label="Notes on the page" hidden={!margin}>
                  <header className="armada-doc__notes-head">
                    <h4 className="armada-doc__label">Notes</h4>
                    <Button variant="primary" size="sm" disabled={held.length === 0} onClick={viewer.send}>
                      Send to Job
                      <KbdBinding binding="⌘↵" />
                    </Button>
                  </header>
                  {held.length === 0 ? null : (
                    <ol className="armada-doc__list" aria-label="Notes held">
                      {held.map((note, index) => (
                        <li key={`${String(index)}:${note.text}`} className="armada-doc__note" data-lit={lit?.id === `held:${String(index)}` ? "" : undefined} {...lights(`held:${String(index)}`)}>
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
                          <li key={`${String(index)}:${note.body}`} className="armada-doc__note" data-sent="" data-lit={lit?.id === `sent:${String(index)}` ? "" : undefined} {...lights(`sent:${String(index)}`)}>
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
                  margin={margin}
                  notes={notes}
                  lit={lit}
                  onLit={setLit}
                  exporters={exporters}
                />
              </div>
            </>
          )}
          <div ref={setMenus} className="armada-doc__menus" />
        </section>
      </Sheet>
    </div>
  );
}
