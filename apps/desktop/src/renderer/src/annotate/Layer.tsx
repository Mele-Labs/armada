// The annotation layer, #1226. Dev only: `main.tsx` loads this chunk only when a
// save path exists, and a packaged build has none.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { Button, Checkbox, KbdChord, SplitButton, Textarea, Tooltip } from "@armada/components";

import { openJobIn } from "@armada/jobs";

import type { BridgeApi } from "../../../shared/api";
import { byCreation, sentAs, shotName, type Annotation, type Box } from "../../../shared/annotations";
import { askToOpenSession } from "../open-session";
import { askToTell } from "../tell";
import { capture, locate } from "./capture";
import { componentsOf, fiberOf } from "./fiber";
import {
  metricsOf,
  placeCard,
  samePlacementInput,
  sizeOf,
  NO_CARD,
  NO_METRICS,
  type CardSize,
  type Metrics,
  type Viewport,
} from "./place";
import { sendToFleet, sendToSession, unsendable, type SessionTarget } from "./send";
import { withoutLayer } from "./shoot";
import type { Sink } from "./sink";
import "./annotate.css";

/** ⌥⌘A. Free in Bridge's key map and in Electron's default menu, beside ⌥⌘I for devtools. */
export function isToggle(event: Pick<globalThis.KeyboardEvent, "code" | "metaKey" | "altKey" | "ctrlKey" | "shiftKey">): boolean {
  return event.code === "KeyA" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
}

/** Survives a reload, so pins are still drawn after one. Per window, not per machine. */
export const ON_KEY = "armada.annotate.on";

/** Pointer events the layer takes from the app while it is on, so a click annotates rather than acts. */
const SWALLOWED = ["pointerdown", "pointerup", "mousedown", "mouseup", "click", "dblclick", "contextmenu"] as const;

type Draft = { note: Annotation; element: Element; text: string };

const boxOf = (element: Element): Box => {
  const r = element.getBoundingClientRect();
  return { x: r.x, y: r.y, width: r.width, height: r.height };
};

const place = (box: Box): CSSProperties =>
  ({
    "--annotate-x": `${box.x}px`,
    "--annotate-y": `${box.y}px`,
    "--annotate-w": `${box.width}px`,
    "--annotate-h": `${box.height}px`,
  }) as CSSProperties;

function inLayer(target: EventTarget | null): boolean {
  const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  // The mock's walk card (`WALK_UI`) is as much the layer's as its bar: a walk over the layer has to be able to go on.
  return element?.closest("[data-armada-annotate], [data-walk-ui]") != null;
}

/** What the pointer is over, taking an SVG as the whole glyph rather than one path in it. */
function targetOf(target: EventTarget | null): Element | null {
  if (!(target instanceof Element)) return null;
  return target.closest("svg") ?? target;
}

/** What the layer asks of Fleet's seam: the Job send, the Session send, and the Sessions that are live. */
export type LayerFleet = Pick<
  BridgeApi,
  "stageAttachment" | "proposeFromRequest" | "startSession" | "sendSessionMessage" | "state" | "subscribe"
>;

/** What a Session without a title is called, as the Sessions list calls it. */
const titled = (one: { id: string; title?: string }): string => one.title ?? `s-${one.id.slice(0, 8)}`;

/**
 * What a row tells its send acts when the note kept no picture as it was saved, which only one
 * written before pictures were kept has, and is off this screen, where none can be taken now.
 */
const NO_SHOT = "No screenshot: on another screen";

/** The same fact for a send of several, where only some of them go without one. */
const noShot = (away: number, of: number): string => (away === of ? NO_SHOT : `No screenshot for ${away} of ${of}: on another screen`);

export function Layer({ sink, fleet: given, openSession = askToOpenSession, tell = askToTell }: { sink: Sink; fleet?: LayerFleet; openSession?: (id: string) => void; tell?: (sentence: string) => void }) {
  const fleet: LayerFleet | undefined = given ?? window.armada;
  const [live, setLive] = useState<SessionTarget[]>([]);
  const [on, setOn] = useState(() => sessionStorage.getItem(ON_KEY) === "1");
  const [notes, setNotes] = useState<Annotation[]>([]);
  const [hovered, setHovered] = useState<Element | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [listing, setListing] = useState(false);
  // The row whose Delete is armed, apart from the card's own.
  const [deletingRow, setDeletingRow] = useState<string | null>(null);
  // A view over the notes, never written to a file. It starts off every time the
  // layer comes on: the moment this exists for is turning the layer on after a
  // batch has been fixed and reading only what is still open. Showing the done
  // ones is a look back, so it lasts as long as the look.
  const [showingDone, setShowingDone] = useState(false);
  const [frames, setFrames] = useState<Record<string, Box | null>>({});
  // Which send is out: one note's id, or every open note on this screen.
  const [sending, setSending] = useState<string | null>(null);
  // Notes ticked in the panel, by id, for one act on all of them. Read through
  // what the panel lists, so a note hidden with the done ones, or deleted, is
  // never acted on or counted.
  const [selected, setSelected] = useState<ReadonlySet<string>>(() => new Set());
  const [deletingSelection, setDeletingSelection] = useState(false);
  // A status change or delete over the selection is out; a send is `sending`.
  const [acting, setActing] = useState(false);
  // The last row ticked, which a shift-press ticks the rows between it and.
  const anchorRef = useRef<string | null>(null);
  const shiftRef = useRef(false);
  const cannotSend = unsendable(sink);
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const barRef = useRef<HTMLDivElement | null>(null);
  const layerRef = useRef<HTMLDivElement | null>(null);
  // The sink, with every capture taken of the app alone and not of the layer drawn over it.
  const shooter = useMemo<Sink>(() => ({ ...sink, capture: (box) => withoutLayer(layerRef.current, () => sink.capture(box)) }), [sink]);
  // A save out on the draft. A ref, not state: a second press can land before
  // the first one's render, and the draft stays up until the save answers.
  const savingRef = useRef(false);

  const fail = useCallback((what: string, cause: unknown) => {
    tell(`${what} failed through ${sink.via}: ${cause instanceof Error ? cause.message : String(cause)}`);
  }, [sink.via, tell]);

  useEffect(() => {
    function pressed(event: globalThis.KeyboardEvent): void {
      if (!isToggle(event)) return;
      event.preventDefault();
      event.stopPropagation();
      setOn((was) => !was);
    }
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, []);

  // Read the files again every time the layer comes on, so a note an agent marked done shows as done.
  useEffect(() => {
    sessionStorage.setItem(ON_KEY, on ? "1" : "0");
    if (!on) {
      setHovered(null);
      setDraft(null);
      setOpened(null);
      setShowingDone(false);
      setListing(false);
      setSelected(new Set());
      setDeletingSelection(false);
      anchorRef.current = null;
      return;
    }
    sink.list().then((read) => setNotes([...read].sort(byCreation)), (cause) => fail("Reading notes", cause));
  }, [on, sink, fail]);

  // Every live Session, hosted and terminal, as the dropdown offers them.
  useEffect(() => {
    if (!on || fleet === undefined) return;
    const read = (state: Awaited<ReturnType<LayerFleet["state"]>>): void =>
      setLive(
        state.sessions.state === "read"
          ? state.sessions.sessions.filter((one) => one.state === "live").map((one) => ({ id: one.id, title: titled(one) }))
          : [],
      );
    void fleet.state().then(read, () => undefined);
    return fleet.subscribe(read);
  }, [on, fleet]);

  useEffect(() => {
    if (!on) return undefined;
    function moved(event: PointerEvent): void {
      if (inLayer(event.target)) return;
      setHovered(targetOf(event.target));
    }
    function swallowed(event: Event): void {
      if (inLayer(event.target)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      if (event.type !== "click") return;
      const element = targetOf(event.target);
      // A note half written is not thrown away by a stray click.
      if (element === null || (draftRef.current !== null && draftRef.current.text.trim() !== "")) return;
      setOpened(null);
      setDraft({ note: capture(element, new Date()), element, text: "" });
    }
    window.addEventListener("pointermove", moved, true);
    for (const type of SWALLOWED) window.addEventListener(type, swallowed, true);
    return () => {
      window.removeEventListener("pointermove", moved, true);
      for (const type of SWALLOWED) window.removeEventListener(type, swallowed, true);
    };
  }, [on]);

  // Pins follow their elements through scrolling, resizing and re-rendering; compared before set, so a still page costs no render.
  useEffect(() => {
    if (!on) return undefined;
    let frame = 0;
    let last = "";
    const tick = (): void => {
      const next: Record<string, Box | null> = {};
      for (const note of notes) {
        const element = locate(note);
        next[note.id] = element === null ? null : boxOf(element);
      }
      if (hovered !== null) next["hovered"] = hovered.isConnected ? boxOf(hovered) : null;
      if (draft !== null) next["draft"] = draft.element.isConnected ? boxOf(draft.element) : draft.note.box;
      // The window is a frame the card is placed against, so a resize that moves
      // no element still has to re-place it. Read here rather than on a listener
      // of its own: this loop already decides when the layer redraws. The bar
      // goes with it: it is pinned over everything, and its height changes with
      // what it has to say, so the card is placed above whatever it measures.
      next["window"] = { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };
      next["bar"] = barRef.current === null ? null : boxOf(barRef.current);
      const key = JSON.stringify(next);
      if (key !== last) {
        last = key;
        setFrames(next);
      }
      frame = requestAnimationFrame(tick);
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [on, notes, hovered, draft]);

  /**
   * A picture of `box`, kept as note `id`'s, or null where none was taken or kept. A picture
   * that fails to keep is told, and the note is saved without one rather than lost to it.
   */
  async function keepShot(id: string, box: Box): Promise<string | null> {
    // In a browser nothing captures, so the layer is not blinked out for a picture that never comes.
    if (sink.via !== "main") return null;
    const png = await shooter.capture(box).catch(() => null);
    if (png === null) return null;
    try {
      await sink.saveShot(id, png);
      return shotName(id);
    } catch (cause) {
      fail("Saving the screenshot", cause);
      return null;
    }
  }

  async function save(): Promise<void> {
    if (draft === null || draft.text.trim() === "" || savingRef.current) return;
    savingRef.current = true;
    // The Job whose detail is open as the note is saved, not as it was begun: the save is the act
    // that says what it is about. Left out where none is, and never kept from an earlier save.
    const { openJobId: _was, screenshot: _none, ...begun } = draft.note;
    const openJobId = openJobIn(document);
    try {
      // Taken here, once: a draft is always a new note, and nothing rewrites a saved note's text.
      const screenshot = await keepShot(begun.id, draft.element.isConnected ? boxOf(draft.element) : draft.note.box);
      const note: Annotation = {
        ...begun,
        text: draft.text.trim(),
        ...(openJobId === undefined ? {} : { openJobId }),
        ...(screenshot === null ? {} : { screenshot }),
        updatedAt: new Date().toISOString(),
      };
      await sink.save(note);
      setNotes((was) => [...was.filter((n) => n.id !== note.id), note].sort(byCreation));
      setDraft(null);
    } catch (cause) {
      fail("Saving", cause);
    } finally {
      savingRef.current = false;
    }
  }

  /** True once the file says it; a failure is told and leaves the note as it was. */
  async function setStatus(note: Annotation, status: Annotation["status"]): Promise<boolean> {
    const next = { ...note, status, updatedAt: new Date().toISOString() };
    try {
      await sink.save(next);
      setNotes((was) => was.map((n) => (n.id === note.id ? next : n)));
      return true;
    } catch (cause) {
      fail("Saving", cause);
      return false;
    }
  }

  /** One note to Fleet. Written back with its Job only once Fleet took it. */
  async function send(note: Annotation): Promise<boolean> {
    // Null where the note's element is not on this screen, so a note with no kept picture is never shot at the wrong one.
    const box = frames[note.id] ?? null;
    if (fleet === undefined) return false;
    try {
      const answer = await sendToFleet(note, box, shooter, fleet, new Date());
      if (!answer.ok) {
        tell(answer.saying);
        return false;
      }
      const next = { ...note, sent: answer.sent, updatedAt: answer.sent.at };
      await sink.save(next);
      setNotes((was) => was.map((n) => (n.id === note.id ? next : n)));
      return true;
    } catch (cause) {
      fail("Sending", cause);
      return false;
    }
  }

  async function sendOne(note: Annotation): Promise<void> {
    setSending(note.id);
    await send(note);
    setSending(null);
  }

  /** Every open, unsent note on this screen, one Job each, stopping at the first Fleet refuses. "Here" is this screen, not which notes have a picture. */
  async function sendHere(): Promise<void> {
    setSending("here");
    for (const note of notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null)) {
      if (!(await send(note))) break;
    }
    setSending(null);
  }

  /** These notes as one message: to a Session started for them, or to `target`. `key` names the send that is out. */
  async function sendToSessionAs(key: string, list: readonly Annotation[], target: SessionTarget | null): Promise<boolean> {
    if (fleet === undefined || list.length === 0) return false;
    setSending(key);
    try {
      const answer = await sendToSession(list.map((note) => ({ note, box: frames[note.id] ?? null })), target, shooter, fleet, new Date());
      if (!answer.ok) {
        tell(answer.saying);
        return false;
      }
      const sentNotes = list.map((note) => ({ ...note, sent: answer.sent, updatedAt: answer.sent.at }));
      for (const next of sentNotes) await sink.save(next);
      setNotes((was) => was.map((n) => sentNotes.find((next) => next.id === n.id) ?? n));
      if ("sessionId" in answer.sent) openSession(answer.sent.sessionId);
      return true;
    } catch (cause) {
      fail("Sending", cause);
      return false;
    } finally {
      setSending(null);
    }
  }

  const sendHereToSession = (target: SessionTarget | null): Promise<boolean> =>
    sendToSessionAs("here", notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null), target);

  async function remove(note: Annotation): Promise<boolean> {
    try {
      await sink.remove(note.id);
      setNotes((was) => was.filter((n) => n.id !== note.id));
      setOpened(null);
      setDeletingRow(null);
      return true;
    } catch (cause) {
      fail("Deleting", cause);
      return false;
    }
  }

  function select(ids: readonly string[], on: boolean): void {
    setDeletingSelection(false);
    setSelected((was) => {
      const next = new Set(was);
      for (const id of ids) {
        if (on) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  }

  function clearSelection(): void {
    setSelected(new Set());
    setDeletingSelection(false);
    anchorRef.current = null;
  }

  /** One Job per selected note, stopping at the first Fleet refuses, as the bar's Dispatch job does. */
  async function dispatchSelection(list: readonly Annotation[]): Promise<void> {
    setSending("selection");
    let all = true;
    for (const note of list) {
      if (!(await send(note))) {
        all = false;
        break;
      }
    }
    setSending(null);
    if (all) clearSelection();
  }

  async function sessionSelection(list: readonly Annotation[], target: SessionTarget | null): Promise<void> {
    if (await sendToSessionAs("selection", list, target)) clearSelection();
  }

  /** `act` on each selected note in turn, stopping at the first that fails; the selection goes once all took. */
  async function eachSelected(list: readonly Annotation[], act: (note: Annotation) => Promise<boolean>): Promise<void> {
    setActing(true);
    let all = true;
    for (const note of list) {
      if (!(await act(note))) {
        all = false;
        break;
      }
    }
    setActing(false);
    setDeletingSelection(false);
    if (all) clearSelection();
  }

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      setDraft(null);
      setOpened(null);
    } else if (event.key === "Enter" && event.metaKey && draft !== null) {
      event.preventDefault();
      event.stopPropagation();
      void save();
    }
  }

  if (!on) return null;

  const measured = frames["window"];
  const size = { width: measured?.width ?? window.innerWidth, height: measured?.height ?? window.innerHeight };
  const view: Viewport = { ...size, floor: frames["bar"]?.y ?? size.height };
  // Numbering runs over the open notes alone, oldest first, so marking one done
  // renumbers nothing above it and a new note takes the next open number rather
  // than counting everything ever written.
  const numbers = new Map(notes.filter((n) => n.status === "open").map((n, i) => [n.id, i + 1]));
  const drawn = showingDone ? notes : notes.filter((n) => n.status === "open");
  const openNote = drawn.find((n) => n.id === opened) ?? null;
  const openBox = openNote === null ? null : frames[openNote.id] ?? null;
  const hoveredBox = draft === null ? frames["hovered"] ?? null : null;
  const draftBox = draft === null ? null : frames["draft"] ?? draft.note.box;
  const offScreen = drawn.filter((n) => frames[n.id] == null).length;
  const open = numbers.size;
  const done = notes.length - open;
  const panelBottom = frames["bar"] == null ? 0 : size.height - frames["bar"].y;
  const sendableHere = notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null).length;
  const groups = groupsOf(notes, showingDone);
  const listed = groups.flatMap(({ list }) => list);
  const listedIds = new Set(listed.map((n) => n.id));
  // In the order they were written, as the bar's sends go.
  const chosen = notes.filter((n) => selected.has(n.id) && listedIds.has(n.id));
  const chosenSendable = chosen.filter((n) => n.status === "open" && n.sent === undefined);
  const chosenSent = chosen.filter((n) => n.sent !== undefined).length;
  const chosenDone = chosen.length - chosenSendable.length - chosenSent;
  const chosenShotless = chosenSendable.filter((n) => n.screenshot === undefined && frames[n.id] == null).length;
  const leftOut = [chosenSent > 0 ? `${chosenSent} already sent` : null, chosenDone > 0 ? `${chosenDone} done` : null].filter((part) => part !== null);
  const allChosen = listed.length > 0 && chosen.length === listed.length;
  const busy = sending !== null || acting;
  const dispatchSays = cannotSend ?? "Send each note as a Job";

  return (
    <div ref={layerRef} className="armada-annotate" data-armada-annotate="" onKeyDown={keyed}>
      {hoveredBox !== null && hovered !== null && (
        <div className="armada-annotate__outline" style={place(hoveredBox)}>
          <span className="armada-annotate__tag">
            {componentsOf(fiberOf(hovered)).component ?? hovered.tagName.toLowerCase()}
          </span>
        </div>
      )}

      {drawn.map((note) => {
        const box = frames[note.id];
        if (box == null) return null;
        // A done note carries no number, here or in its label, so nothing a
        // person reads on the screen counts work that is finished.
        const number = numbers.get(note.id);
        const where = number === undefined ? "Done note" : `Note ${number}, open`;
        return (
          <button
            key={note.id}
            type="button"
            className="armada-annotate__pin"
            data-status={note.status}
            data-sent={note.sent === undefined ? undefined : ""}
            style={place(box)}
            aria-label={`${where}${note.sent === undefined ? "" : `, sent as ${sentAs(note.sent)}`}: ${note.text}`}
            onClick={() => {
              setDraft(null);
              setDeleting(false);
              setOpened(opened === note.id ? null : note.id);
            }}
          >
            {number}
          </button>
        );
      })}

      {draft !== null && draftBox !== null && (
        <>
          <div className="armada-annotate__outline" data-held="" style={place(draftBox)} />
          <Card box={draftBox} view={view} label="New note">
            <p className="armada-annotate__meta">{chain(draft.note)}</p>
            <Source note={draft.note} />
            <Textarea
              label="Note"
              autoFocus
              value={draft.text}
              onChange={(event) => setDraft({ ...draft, text: event.target.value })}
            />
            <div className="armada-annotate__actions">
              <Button variant="ghost" size="sm" onClick={() => setDraft(null)}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" disabled={draft.text.trim() === ""} onClick={() => void save()}>
                Save note
              </Button>
            </div>
          </Card>
        </>
      )}

      {openNote !== null && openBox !== null && (
        <Card box={openBox} view={view} label="Note">
          <p className="armada-annotate__meta">{chain(openNote)}</p>
          <Source note={openNote} />
          <p className="armada-annotate__text">{openNote.text}</p>
          {openNote.sent !== undefined && (
            <p className="armada-annotate__meta">
              {"jobId" in openNote.sent
                ? `Sent to Fleet as ${openNote.sent.handle}, waiting on approval on the Board`
                : `Sent to Session ${openNote.sent.title}`}
            </p>
          )}
          {openNote.sent === undefined && cannotSend !== null && <p className="armada-annotate__meta">{cannotSend}</p>}
          <div className="armada-annotate__actions">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => (deleting ? void remove(openNote) : setDeleting(true))}
            >
              {deleting ? "Press again to delete" : "Delete"}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void setStatus(openNote, openNote.status === "open" ? "done" : "open")}
            >
              {openNote.status === "open" ? "Mark done" : "Reopen"}
            </Button>
            {openNote.sent === undefined && openNote.status === "open" && (
              <Button
                variant="primary"
                size="sm"
                disabled={cannotSend !== null || sending !== null}
                onClick={() => void sendOne(openNote)}
              >
                {sending === openNote.id ? "Sending…" : "Send to Fleet"}
              </Button>
            )}
          </div>
        </Card>
      )}

      {listing && (
        <aside
          className="armada-annotate__panel"
          aria-label="All notes"
          style={{ "--annotate-panel-bottom": `${panelBottom}px` } as CSSProperties}
        >
          {listed.length > 0 && (
            <div className="armada-annotate__selection" role="group" aria-label="Selected notes">
              <Checkbox checked={allChosen} onChange={(event) => (event.target.checked ? select(listed.map((n) => n.id), true) : clearSelection())}>
                Select all
              </Checkbox>
              {chosen.length > 0 && (
                <>
                  <span className="armada-annotate__selection-said">
                    {chosen.length} selected
                    {chosenSendable.length > 0 && leftOut.length > 0 ? `. Sends leave out ${chosen.length - chosenSendable.length}: ${leftOut.join(", ")}` : ""}
                  </span>
                  <div className="armada-annotate__actions">
                    {chosenSendable.length > 0 && (
                      <>
                        <Tooltip label={chosenShotless > 0 ? `${dispatchSays}. ${noShot(chosenShotless, chosenSendable.length)}` : dispatchSays} asChild>
                          <Button variant="secondary" size="sm" disabled={cannotSend !== null || busy} onClick={() => void dispatchSelection(chosenSendable)}>
                            {sending === "selection" ? "Sending…" : "Dispatch job"}
                          </Button>
                        </Tooltip>
                        {cannotSend === null && (
                          <SplitButton
                            size="sm"
                            menuLabel="Send to a Session"
                            disabled={busy}
                            {...(chosenShotless > 0 ? { note: noShot(chosenShotless, chosenSendable.length) } : {})}
                            onAction={() => void sessionSelection(chosenSendable, null)}
                            items={live.map((one) => ({ label: one.title, onSelect: () => void sessionSelection(chosenSendable, one) }))}
                          >
                            Start session
                          </SplitButton>
                        )}
                      </>
                    )}
                    {chosen.some((n) => n.status === "open") && (
                      <Button variant="ghost" size="sm" disabled={busy} onClick={() => void eachSelected(chosen.filter((n) => n.status === "open"), (n) => setStatus(n, "done"))}>
                        Mark done
                      </Button>
                    )}
                    {chosen.some((n) => n.status === "done") && (
                      <Button variant="ghost" size="sm" disabled={busy} onClick={() => void eachSelected(chosen.filter((n) => n.status === "done"), (n) => setStatus(n, "open"))}>
                        Reopen
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={busy}
                      onClick={() => (deletingSelection ? void eachSelected(chosen, remove) : setDeletingSelection(true))}
                    >
                      {deletingSelection ? "Press again to delete" : "Delete"}
                    </Button>
                  </div>
                </>
              )}
            </div>
          )}
          {groups.map(({ screen, list }) => (
            <section key={screen ?? ""} className="armada-annotate__group" aria-label={screen ?? "No screen"}>
              <h2 className="armada-annotate__screen">{screen ?? "No screen"}</h2>
              <ul className="armada-annotate__rows">
                {list.map((note) => {
                  const here = frames[note.id] != null;
                  const shotless = !here && note.screenshot === undefined;
                  const dispatchable = note.status === "open" && note.sent === undefined;
                  return (
                    <li
                      key={note.id}
                      className="armada-annotate__row"
                      data-status={note.status}
                      data-here={here ? "" : undefined}
                      data-selected={selected.has(note.id) ? "" : undefined}
                    >
                      <Tooltip label={here ? "Show on this screen" : "On another screen"} asChild>
                        <button
                          type="button"
                          className="armada-annotate__row-main"
                          aria-disabled={!here}
                          onClick={() => {
                            if (!here) return;
                            setDraft(null);
                            setDeleting(false);
                            setOpened(note.id);
                          }}
                        >
                          <span className="armada-annotate__row-text">{note.text}</span>
                          <span className="armada-annotate__row-meta">
                            <span>{note.component ?? note.element.tag}</span>
                            {note.sent !== undefined && <span className="armada-annotate__sent">{sentAs(note.sent)}</span>}
                          </span>
                        </button>
                      </Tooltip>
                      <div className="armada-annotate__actions">
                        <Checkbox
                          aria-label={`Select ${note.text}`}
                          checked={selected.has(note.id)}
                          onClick={(event) => {
                            shiftRef.current = event.shiftKey;
                          }}
                          onChange={(event) => {
                            const from = anchorRef.current === null ? -1 : listed.findIndex((n) => n.id === anchorRef.current);
                            const to = listed.findIndex((n) => n.id === note.id);
                            const range = shiftRef.current && from !== -1 ? listed.slice(Math.min(from, to), Math.max(from, to) + 1) : [note];
                            shiftRef.current = false;
                            anchorRef.current = note.id;
                            select(range.map((n) => n.id), event.target.checked);
                          }}
                        >
                          Select
                        </Checkbox>
                        {dispatchable && (
                          <>
                            <Tooltip label={shotless ? `${cannotSend ?? "Send this note as a Job"}. ${NO_SHOT}` : (cannotSend ?? "Send this note as a Job")} asChild>
                              <Button variant="secondary" size="sm" disabled={cannotSend !== null || sending !== null} onClick={() => void sendOne(note)}>
                                {sending === note.id ? "Sending…" : "Dispatch job"}
                              </Button>
                            </Tooltip>
                            {cannotSend === null && (
                              <SplitButton
                                size="sm"
                                menuLabel="Send to a Session"
                                disabled={sending !== null}
                                {...(shotless ? { note: NO_SHOT } : {})}
                                onAction={() => void sendToSessionAs(note.id, [note], null)}
                                items={live.map((one) => ({ label: one.title, onSelect: () => void sendToSessionAs(note.id, [note], one) }))}
                              >
                                Start session
                              </SplitButton>
                            )}
                          </>
                        )}
                        <Button variant="ghost" size="sm" onClick={() => void setStatus(note, note.status === "open" ? "done" : "open")}>
                          {note.status === "open" ? "Mark done" : "Reopen"}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => (deletingRow === note.id ? void remove(note) : setDeletingRow(note.id))}
                        >
                          {deletingRow === note.id ? "Press again to delete" : "Delete"}
                        </Button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </aside>
      )}

      <div ref={barRef} className="armada-annotate__bar" role="status">
        <span>Annotating</span>
        <button type="button" className="armada-annotate__status" aria-expanded={listing} onClick={() => setListing(!listing)}>
          {open} open, {done} done
          {offScreen > 0 ? `, ${offScreen} not on this screen` : ""}
        </button>
        {done > 0 && (
          <Button variant="ghost" size="sm" onClick={() => setShowingDone(!showingDone)}>
            {showingDone ? "Hide done notes" : "Show done notes"}
          </Button>
        )}
        {cannotSend === null && sendableHere > 0 && (
          <>
            <Button variant="secondary" size="sm" disabled={sending !== null} onClick={() => void sendHere()}>
              {sending === "here" ? "Sending…" : "Dispatch job"}
            </Button>
            <SplitButton
              size="sm"
              menuLabel="Send to a Session"
              disabled={sending !== null}
              onAction={() => void sendHereToSession(null)}
              items={live.map((one) => ({ label: one.title, onSelect: () => void sendHereToSession(one) }))}
            >
              Start session
            </SplitButton>
          </>
        )}
        <KbdChord keys={["⌥", "⌘", "A"]} aria-label="Option Command A turns annotating off" />
      </div>
    </div>
  );
}

/**
 * The note card, put where all of it is on screen.
 *
 * **It measures itself, because nothing else can.** Where the card fits depends
 * on how tall it is, and the stylesheet never learns that — which is why `left`
 * was clamped into the window and `top` was not, and why picking an element as
 * tall as the window drew the card off the top of it. The measurement is taken
 * in a layout effect, so the first frame a person sees is already placed, and
 * kept current by a `ResizeObserver` for as long as the card is up.
 */
function Card({
  box,
  view,
  label,
  children,
}: {
  box: Box;
  view: Viewport;
  label: string;
  children: ReactNode;
}) {
  const node = useRef<HTMLDivElement | null>(null);
  const [self, setSelf] = useState<{ size: CardSize; metrics: Metrics } | null>(null);

  useLayoutEffect(() => {
    const element = node.current;
    if (element === null) return undefined;
    const read = (): void =>
      setSelf((was) => {
        const next = { size: sizeOf(element), metrics: metricsOf(element) };
        return was !== null && samePlacementInput(was, next) ? was : next;
      });
    read();
    const observer = new ResizeObserver(read);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const at = placeCard(box, self?.size ?? NO_CARD, view, self?.metrics ?? NO_METRICS);
  return (
    <div
      ref={node}
      className="armada-annotate__card"
      role="dialog"
      aria-label={label}
      data-side={at.side}
      style={
        {
          "--annotate-card-x": `${at.x}px`,
          "--annotate-card-y": `${at.y}px`,
          "--annotate-card-max": `${at.maxHeight}px`,
        } as CSSProperties
      }
    >
      {children}
    </div>
  );
}

/** Every screen's notes, open first, a screen named by its notes and "no screen" last; done ones only where asked for. */
function groupsOf(notes: readonly Annotation[], showingDone: boolean): { screen: string | null; list: Annotation[] }[] {
  const shown = notes.filter((n) => showingDone || n.status === "open");
  const screens = [...new Set(shown.map((n) => n.screen))].sort((a, b) => (a === null ? 1 : b === null ? -1 : a.localeCompare(b)));
  return screens.map((screen) => ({
    screen,
    list: shown.filter((n) => n.screen === screen).sort((a, b) => Number(a.status === "done") - Number(b.status === "done")),
  }));
}

/** The file and line the element's JSX is on, so it can be seen to have been captured. #1584. */
function Source({ note }: { note: Annotation }) {
  if (note.source === undefined) return null;
  return <p className="armada-annotate__meta">{`${note.source.file}:${note.source.line}`}</p>;
}

/** `JobRowStacked ← ActiveJobsList ← Board`, then the selector, for the person checking what was picked. */
function chain(note: Annotation): string {
  const names = [note.component, ...note.owners].filter((n): n is string => n !== null).slice(0, 4);
  return `${names.length > 0 ? names.join(" ← ") : note.element.tag} · ${note.selector}`;
}
