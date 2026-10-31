// The annotation layer, #1226. Dev only: `main.tsx` loads this chunk only when a
// save path exists, and a packaged build has none.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ClipboardEvent,
  type CSSProperties,
  type DragEvent,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Button, Checkbox, forgetKept, JobCard, KbdChord, SearchField, SplitButton, Tabs, Textarea, Tooltip } from "@armada/components";


import type { BridgeApi } from "../../../shared/api";
import { byCreation, exampleName, sentAs, shotName, type Annotation, type Box, type Sent, type StyleChange } from "../../../shared/annotations";
import { pngSize, ringOn } from "../../../shared/ring";
import { useDebugBridgeAsked } from "../debug-bridge";
import { withState, type StateSource } from "../state-feed";
import { jobFactsOf } from "../job-facts";
import { askToOpenSessionInCockpit, openSessionIn } from "../open-session";
import { IssueJobCard } from "../issues-tab/marks";
import { useReportAsked } from "../report";
import { useSessions } from "../sessions-draft";
import { askToTell } from "../tell";
import { capture, captureWindow, currentScreen, locate, windowBox } from "./capture";
import { contextOf, type ContextState } from "./context";
import { startWithDraft } from "./debug-draft";
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
import { sendToSession, unsendable, type SessionTarget } from "./send";
import { painted, withoutLayer } from "./shoot";
import type { Sink } from "./sink";
import { inLayer, targetOf } from "./target";
import { asPng, Examples, imagesIn, Tools } from "./Tools";
import "./annotate.css";

/** ⌥⌘A. Free in Bridge's key map and in Electron's default menu, beside ⌥⌘I for devtools. */
export function isToggle(event: Pick<globalThis.KeyboardEvent, "code" | "metaKey" | "altKey" | "ctrlKey" | "shiftKey">): boolean {
  return event.code === "KeyA" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
}

/** ⌥⌘S while the layer is on: a note on the whole window, its picture taken as the keys are pressed. */
export function isShootWindow(event: Pick<globalThis.KeyboardEvent, "code" | "metaKey" | "altKey" | "ctrlKey" | "shiftKey">): boolean {
  return event.code === "KeyS" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
}

/** What a Debug Bridge note says: the screen it was taken on, then what is asked of the Session. */
export const debugText = (screen: string | null): string =>
  `Debug Bridge${screen === null ? "" : ` on ${screen}`}\n\nWork out what is going on in Bridge from the screenshot and the state below. I will tell you what looks wrong.`;

/** Survives a reload, so pins are still drawn after one. Per window, not per machine. */
export const ON_KEY = "armada.annotate.on";

/** Pointer events the layer takes from the app while it is on, so a click annotates rather than acts. */
const SWALLOWED = ["pointerdown", "pointerup", "mousedown", "mouseup", "click", "dblclick", "contextmenu"] as const;

/**
 * `png` is a whole-window note's picture, taken as it was begun so it shows the state the person saw. `changes` and
 * `examples` are what he made with the card's tools, kept on the note as it is saved; an example's bytes are in `pictures`.
 */
type Draft = { note: Annotation; element: Element; text: string; png?: ArrayBuffer | null; changes?: StyleChange[]; examples?: string[] };

/** One empty list, so a note with no changes hands the tools the same one each render. */
const NONE: StyleChange[] = [];

/** Whether the person has put anything on the draft: words, a change or a picture. A stray click does not throw it away. */
const begun = (draft: Draft | null): boolean =>
  draft !== null && (draft.text.trim() !== "" || (draft.changes?.length ?? 0) > 0 || (draft.examples?.length ?? 0) > 0);

/** What a text box on note `id` keeps its words under. */
const keptAs = (id: string, box: string): string => `annotate:${box}:${id}`;

/** `note` with these changes, and no field for them where there are none. */
function withChanges(note: Annotation, changes: StyleChange[]): Annotation {
  const { changes: _was, ...rest } = note;
  return changes.length === 0 ? rest : { ...rest, changes };
}

/** `note` with these example pictures, and no field for them where there are none. */
function withExamples(note: Annotation, examples: string[]): Annotation {
  const { examples: _was, ...rest } = note;
  return examples.length === 0 ? rest : { ...rest, examples };
}

/** The number the next of note `id`'s example pictures takes, after the highest it has. */
function nextExample(id: string, names: readonly string[]): number {
  const prefix = `${id}-example-`;
  return names.reduce((most, name) => Math.max(most, Number(name.slice(prefix.length, -".png".length)) || 0), 0) + 1;
}

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

/** What the layer asks of Fleet's seam: the Session send, and the Sessions that are live. */
export type LayerFleet = Pick<BridgeApi, "startSession" | "sendSessionMessage" | "sendSessionNotes"> & StateSource;

/** The Job a note went to, as the shared card. A Job this window does not hold is the card with the title the note kept, in no state. */
function SentJob({ sent, onOpen }: { sent: Sent; onOpen: (id: string) => void }) {
  const session = useSessions().find((one) => one.id === sent.sessionId);
  if (session !== undefined) return <IssueJobCard session={session} onOpen={onOpen} />;
  return <JobCard title={sent.title} state="quiet" said="Not held by this window" onOpen={() => onOpen(sent.sessionId)} />;
}

/** What a Session without a title is called, as the Sessions list calls it. */
const titled = (one: { id: string; title?: string }): string => one.title ?? `s-${one.id.slice(0, 8)}`;

/**
 * What a row tells its send acts when the note kept no picture as it was saved, which only one
 * written before pictures were kept has, and is off this screen, where none can be taken now.
 */
const NO_SHOT = "No screenshot: on another screen";

/** The same fact for a send of several, where only some of them go without one. */
const noShot = (away: number, of: number): string => (away === of ? NO_SHOT : `No screenshot for ${away} of ${of}: on another screen`);

/** The panel's tabs, each a view over the notes: what it holds, and what it is called. */
const TABS = [
  { id: "open", label: "Open", holds: (n: Annotation) => n.status === "open" },
  { id: "unsent", label: "No job", holds: (n: Annotation) => n.status === "open" && n.sent === undefined },
  { id: "sent", label: "Has job", holds: (n: Annotation) => n.status === "open" && n.sent !== undefined },
  { id: "done", label: "Done", holds: (n: Annotation) => n.status === "done" },
] as const;
type Tab = (typeof TABS)[number]["id"];

/** Whether the search's words are in the note's text, its screen, or the component or tag it is pinned on. */
function matches(note: Annotation, search: string): boolean {
  const words = search.trim().toLowerCase();
  if (words === "") return true;
  return [note.text, note.screen ?? "", note.component ?? "", note.element.tag].some((field) => field.toLowerCase().includes(words));
}

export function Layer({ sink, fleet: given, openSession = askToOpenSessionInCockpit, tell = askToTell }: { sink: Sink; fleet?: LayerFleet; openSession?: (id: string) => void; tell?: (sentence: string) => void }) {
  const fleet: LayerFleet | undefined = given ?? (window.armada === undefined ? undefined : withState(window.armada));
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
  // Views over the notes, never written to a file. They start over every time the
  // layer comes on: the moment this exists for is turning the layer on after a
  // batch has been fixed and reading only what is still open. Any other tab is a
  // look aside, so it lasts as long as the look.
  const [tab, setTab] = useState<Tab>("open");
  const [search, setSearch] = useState("");
  // Screens whose group is folded in the panel, by name; "" is no screen.
  const [folded, setFolded] = useState<ReadonlySet<string>>(() => new Set());
  const [frames, setFrames] = useState<Record<string, Box | null>>({});
  // Example pictures' bytes by file name: a draft's, held until it is saved, and a saved note's, read back to be shown.
  const [pictures, setPictures] = useState<Record<string, ArrayBuffer>>({});
  const notesRef = useRef(notes);
  notesRef.current = notes;
  // An edit to a saved note's changes, written a beat after the last one so typing in Text is not a write a key.
  const pendingRef = useRef<number | null>(null);
  // Which send is out: one note's id, or every open note on this screen.
  const [sending, setSending] = useState<string | null>(null);
  // Notes ticked in the panel, by id, for one act on all of them. Read through
  // what the panel lists, so a note another tab or the search hides, or one
  // deleted, is never acted on or counted.
  const [selected, setSelected] = useState<ReadonlySet<string>>(() => new Set());
  const [deletingSelection, setDeletingSelection] = useState(false);
  // A status change or delete over the selection is out; a send is `sending`.
  const [acting, setActing] = useState(false);
  // The last row ticked, which a shift-press ticks the rows between it and.
  const anchorRef = useRef<string | null>(null);
  const shiftRef = useRef(false);
  const cannotSend = unsendable(sink);
  // Bridge's state as Fleet last published it, which a whole-window note records.
  const stateRef = useRef<ContextState | null>(null);
  // The layer is on because Report turned it on and nothing since has kept it, so cancelling the card turns it off again.
  const reportedOffRef = useRef(false);
  const onRef = useRef(on);
  onRef.current = on;
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
      reportedOffRef.current = false;
      setOn((was) => !was);
    }
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  }, []);

  useEffect(() => {
    if (!on) return undefined;
    function pressed(event: globalThis.KeyboardEvent): void {
      if (!isShootWindow(event)) return;
      event.preventDefault();
      event.stopPropagation();
      void shootWindow();
    }
    window.addEventListener("keydown", pressed, true);
    return () => window.removeEventListener("keydown", pressed, true);
  });

  // Read the files again every time the layer comes on, so a note an agent marked done shows as done.
  useEffect(() => {
    sessionStorage.setItem(ON_KEY, on ? "1" : "0");
    if (!on) {
      setHovered(null);
      setDraft(null);
      setOpened(null);
      setTab("open");
      setSearch("");
      setFolded(new Set());
      setListing(false);
      setSelected(new Set());
      setDeletingSelection(false);
      anchorRef.current = null;
      return;
    }
    const read = (): void => void sink.list().then((all) => setNotes([...all].sort(byCreation)), (cause) => fail("Reading notes", cause));
    read();
    // Fleet rewrites a note when its Job closes, marked done or no longer sent; absent in a browser.
    return window.armadaDev?.onChanged?.(read);
  }, [on, sink, fail]);

  // Bridge's state, held with the layer off too, since Debug Bridge records it from anywhere.
  useEffect(() => {
    if (fleet === undefined) return;
    const held = (state: Awaited<ReturnType<LayerFleet["state"]>>): void => {
      stateRef.current = state;
    };
    void fleet.state().then(held, () => undefined);
    return fleet.subscribe(held);
  }, [fleet]);

  // Every Session the Jobs list shows as active: a terminal one that is not listening is quiet there (`sessionOfRecord`), so it is left out here.
  useEffect(() => {
    if (!on || fleet === undefined) return;
    const read = (state: Awaited<ReturnType<LayerFleet["state"]>>): void => {
      setLive(
        state.sessions.state === "read"
          ? state.sessions.sessions.filter((one) => one.state === "live" && (one.hosted !== undefined || one.terminal?.listening === true)).map((one) => ({ id: one.id, title: titled(one) }))
          : [],
      );
    };
    void fleet.state().then(read, () => undefined);
    return fleet.subscribe(read);
  }, [on, fleet]);

  /** Comes on if it is off, and opens the card on `element`: what a click does with the layer on. */
  const reportOn = useCallback((element: Element): void => {
    reportedOffRef.current = !onRef.current;
    setOn(true);
    setOpened(null);
    setDraft({ note: capture(element, new Date()), element, text: "" });
  }, []);
  useReportAsked(reportOn);

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
      if (element === null || begun(draftRef.current)) return;
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
      const screen = currentScreen(document);
      for (const note of notes) {
        // A whole-window note is on this screen where it was left on it, and is the window there.
        if (note.scope === "window") {
          next[note.id] = note.screen === screen ? windowBox() : null;
          continue;
        }
        const element = locate(note);
        next[note.id] = element === null ? null : boxOf(element);
      }
      if (hovered !== null) next["hovered"] = hovered.isConnected ? boxOf(hovered) : null;
      if (draft !== null) next["draft"] = draft.note.scope === "window" ? windowBox() : draft.element.isConnected ? boxOf(draft.element) : draft.note.box;
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

  // The open note's example pictures, read back from beside it once each, to be shown on its card.
  const askedRef = useRef(new Set<string>());
  useEffect(() => {
    const note = notes.find((n) => n.id === opened);
    if (note === undefined) return;
    for (const name of note.examples ?? []) {
      if (askedRef.current.has(name)) continue;
      askedRef.current.add(name);
      sink.example(note.id, name).then(
        (png) => {
          if (png !== null) setPictures((was) => (name in was ? was : { ...was, [name]: png }));
        },
        () => askedRef.current.delete(name),
      );
    }
  }, [opened, notes, sink]);

  /**
   * A picture of the window, kept as note `id`'s, and where `on`, the element's box, is on it; null where none was taken or
   * kept. A whole-window note's `on` is null: nothing on it is ringed. A picture that fails to keep is told, and the note is
   * saved without one rather than lost to it.
   */
  async function keepShot(id: string, on: Box | null, taken?: ArrayBuffer | null): Promise<Pick<Annotation, "screenshot" | "ring"> | null> {
    // In a browser nothing captures, so the layer is not blinked out for a picture that never comes.
    if (sink.via !== "main") return null;
    const window = windowBox();
    const png = taken !== undefined ? taken : await shooter.capture(window).catch(() => null);
    if (png === null) return null;
    try {
      await sink.saveShot(id, png);
    } catch (cause) {
      fail("Saving the screenshot", cause);
      return null;
    }
    const size = on === null ? null : pngSize(new Uint8Array(png));
    return { screenshot: shotName(id), ...(on === null || size === null ? {} : { ring: ringOn(on, window.width, size.width) }) };
  }

  /**
   * A note on the whole window: its picture and Bridge's state taken now, as the person saw them,
   * and the card opened for what they say. A note half written is not thrown away for it.
   */
  async function shootWindow(): Promise<void> {
    if (begun(draftRef.current)) return;
    const now = new Date();
    const png = sink.via === "main" ? await shooter.capture(windowBox()).catch(() => null) : null;
    const note = captureWindow(now);
    let context: Annotation["context"];
    try {
      context = stateRef.current === null ? undefined : contextOf(stateRef.current);
    } catch {
      // A state read half-published is no reason to lose the picture; the note goes without it.
      context = undefined;
    }
    setOpened(null);
    setDraft({ note: context === undefined ? note : { ...note, context }, element: document.body, text: "", png });
  }

  // A Debug Bridge out. A ref, so a second ⇧⌘D before the first has answered starts no second Session.
  const debuggingRef = useRef(false);

  /**
   * Debug Bridge, from the palette or ⇧⌘D, with the layer on or off: shoot the window once the
   * palette has gone from it, keep it as a whole-window note, and start a Session holding the note and
   * the picture as a draft. No turn runs until the owner sends it. The note is written first, so a
   * Fleet that refuses leaves it a file.
   */
  async function debugBridge(): Promise<void> {
    if (debuggingRef.current || fleet === undefined) return;
    debuggingRef.current = true;
    try {
      await painted(requestAnimationFrame);
      const now = new Date();
      const box = windowBox();
      const begun = captureWindow(now);
      let context: Annotation["context"];
      try {
        context = stateRef.current === null ? undefined : contextOf(stateRef.current);
      } catch {
        context = undefined;
      }
      const shot = await keepShot(begun.id, null);
      const note: Annotation = {
        ...begun,
        text: debugText(begun.screen),
        ...(context === undefined ? {} : { context }),
        ...shot,
      };
      await sink.save(note);
      setNotes((was) => [...was.filter((n) => n.id !== note.id), note].sort(byCreation));
      const answer = await startWithDraft(note, box, sink, fleet);
      if (!answer.ok) {
        tell(answer.saying);
        return;
      }
      openSession(answer.sessionId);
    } catch (cause) {
      fail("Debug Bridge", cause);
    } finally {
      debuggingRef.current = false;
    }
  }
  useDebugBridgeAsked(() => void debugBridge());

  async function save(): Promise<void> {
    if (draft === null || draft.text.trim() === "" || savingRef.current) return;
    savingRef.current = true;
    const { screenshot: _none, ring: _unringed, ...begun } = draft.note;
    try {
      // Taken here, once: a draft is always a new note, and nothing rewrites a saved note's text.
      const on = begun.scope === "window" ? null : draft.element.isConnected ? boxOf(draft.element) : draft.note.box;
      const shot = await keepShot(begun.id, on, draft.png);
      const changes = draft.changes ?? [];
      const examples = (draft.examples ?? []).filter((name) => pictures[name] !== undefined);
      for (const name of examples) await sink.saveExample(begun.id, name, pictures[name]!);
      const note: Annotation = {
        ...begun,
        text: draft.text.trim(),
        ...shot,
        ...(changes.length === 0 ? {} : { changes }),
        ...(examples.length === 0 ? {} : { examples }),
        updatedAt: new Date().toISOString(),
      };
      await sink.save(note);
      forgetKept(keptAs(note.id, "note"));
      setNotes((was) => [...was.filter((n) => n.id !== note.id), note].sort(byCreation));
      setDraft(null);
      reportedOffRef.current = false;
      // The saved note stays up, so what is done with it is a press away.
      setDeleting(false);
      setOpened(note.id);
    } catch (cause) {
      fail("Saving", cause);
    } finally {
      savingRef.current = false;
    }
  }

  /**
   * A saved note with `patch` on it, shown at once and written to its file: now, or a beat after the last edit where
   * `soon`, so a run of edits is one write. Read from the notes as they are, so an edit made while a picture was read is kept.
   */
  function edit(id: string, patch: (was: Annotation) => Annotation, soon = false): void {
    const was = notesRef.current.find((n) => n.id === id);
    if (was === undefined) return;
    const next: Annotation = { ...patch(was), updatedAt: new Date().toISOString() };
    notesRef.current = notesRef.current.map((n) => (n.id === id ? next : n));
    setNotes(notesRef.current);
    if (pendingRef.current !== null) window.clearTimeout(pendingRef.current);
    const write = (): void => {
      pendingRef.current = null;
      // Read again as it goes, so the last of a run of edits is what is written.
      const latest = notesRef.current.find((n) => n.id === id);
      if (latest !== undefined) sink.save(latest).catch((cause: unknown) => fail("Saving", cause));
    };
    if (soon) pendingRef.current = window.setTimeout(write, 400);
    else write();
  }

  /** Changes made with the card's tools: a draft's kept on it, a saved note's written to it. */
  function changed(into: Annotation | null, changes: StyleChange[]): void {
    if (into === null) setDraft((was) => (was === null ? was : { ...was, changes }));
    else edit(into.id, (was) => withChanges(was, changes), true);
  }

  /**
   * Pictures pasted or dropped on a card, each kept as a PNG among the note's examples: a draft's held until it is saved,
   * a saved note's written beside it at once. One that is not an image is told and left out.
   */
  async function addExamples(into: Annotation | null, files: readonly File[]): Promise<void> {
    const id = into?.id ?? draftRef.current?.note.id;
    if (id === undefined) return;
    let names = into === null ? draftRef.current?.examples ?? [] : notesRef.current.find((n) => n.id === id)?.examples ?? [];
    for (const file of files) {
      const png = await asPng(file).catch(() => null);
      if (png === null) {
        tell(`Picture not added: ${file.name} is not an image`);
        continue;
      }
      const name = exampleName(id, nextExample(id, names));
      if (into !== null) {
        try {
          await sink.saveExample(id, name, png);
        } catch (cause) {
          fail("Saving the picture", cause);
          continue;
        }
      }
      names = [...names, name];
      setPictures((was) => ({ ...was, [name]: png }));
    }
    if (into === null) setDraft((was) => (was === null || was.note.id !== id ? was : { ...was, examples: names }));
    else edit(id, (was) => withExamples(was, names));
  }

  /** A picture taken off a note: off a draft's list, or out of a saved note's file and its directory. */
  async function removeExample(into: Annotation | null, name: string): Promise<void> {
    if (into === null) {
      setDraft((was) => (was === null ? was : { ...was, examples: (was.examples ?? []).filter((one) => one !== name) }));
      return;
    }
    try {
      await sink.removeExample(into.id, name);
    } catch (cause) {
      fail("Removing the picture", cause);
      return;
    }
    edit(into.id, (was) => withExamples(was, (was.examples ?? []).filter((one) => one !== name)));
  }

  /** What a card does with a paste or a drop: images become the note's examples, anything else goes on as it would. */
  const takesPictures = (into: Annotation | null): Pick<CardProps, "onPaste" | "onDragOver" | "onDrop"> => ({
    onPaste: (event) => {
      const files = imagesIn(event.clipboardData);
      if (files.length === 0) return;
      event.preventDefault();
      void addExamples(into, files);
    },
    onDragOver: (event) => {
      if (event.dataTransfer.types.includes("Files")) event.preventDefault();
    },
    onDrop: (event) => {
      const files = imagesIn(event.dataTransfer);
      if (files.length === 0) return;
      event.preventDefault();
      void addExamples(into, files);
    },
  });

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

  /** These notes, each as a pinned note: to a Session started for them, or to `target`. `key` names the send that is out. */
  async function sendToSessionAs(key: string, list: readonly Annotation[], target: SessionTarget | null): Promise<boolean> {
    if (fleet === undefined || list.length === 0) return false;
    setSending(key);
    try {
      const openId = openSessionIn(document);
      const facts = openId === undefined ? undefined : jobFactsOf(openId);
      const open = openId === undefined || facts === undefined ? undefined : { id: openId, facts };
      const answer = await sendToSession(list.map((note) => ({ note, box: frames[note.id] ?? null })), target, shooter, fleet, { address: window.location.href, window: windowBox() }, new Date(), open);
      // Those Fleet took before it refused are stamped too, so a send again does not repeat them.
      const sent = answer.sent;
      const took = answer.ok ? list : list.filter((note) => answer.sentIds?.includes(note.id) === true);
      const sentNotes = sent === undefined ? [] : took.map((note) => ({ ...note, sent, updatedAt: sent.at }));
      for (const next of sentNotes) await sink.save(next);
      setNotes((was) => was.map((n) => sentNotes.find((next) => next.id === n.id) ?? n));
      if (!answer.ok) {
        tell(answer.saying);
        return false;
      }
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

  /** Puts the card away, and the layer with it where Report was all that turned it on. */
  function cancelDraft(): void {
    if (draftRef.current !== null) forgetKept(keptAs(draftRef.current.note.id, "note"));
    setDraft(null);
    if (!reportedOffRef.current) return;
    reportedOffRef.current = false;
    setOn(false);
  }

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      cancelDraft();
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
  // Pins are the open notes', and the done ones' too while the Done tab is chosen.
  const drawn = tab === "done" ? notes : notes.filter((n) => n.status === "open");
  const openNote = drawn.find((n) => n.id === opened) ?? null;
  const openBox = openNote === null ? null : frames[openNote.id] ?? null;
  const hoveredBox = draft === null ? frames["hovered"] ?? null : null;
  const draftBox = draft === null ? null : frames["draft"] ?? draft.note.box;
  const offScreen = drawn.filter((n) => frames[n.id] == null).length;
  const open = numbers.size;
  const done = notes.length - open;
  const panelBottom = frames["bar"] == null ? 0 : size.height - frames["bar"].y;
  const sendableHere = notes.filter((n) => n.status === "open" && n.sent === undefined && frames[n.id] != null).length;
  const holds = TABS.find((one) => one.id === tab)!.holds;
  const groups = groupsOf(notes.filter((n) => holds(n) && matches(n, search)));
  // Folded groups' notes too: Select all takes what the tab and the search hold.
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
        // A whole-window note has no element to pin; the panel lists it.
        if (box == null || note.scope === "window") return null;
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
          <Card box={draftBox} view={view} label="New note" {...takesPictures(null)}>
            <p className="armada-annotate__meta">{chain(draft.note)}</p>
            <Source note={draft.note} />
            <Textarea
              label="Note"
              autoFocus
              keep={keptAs(draft.note.id, "note")}
              value={draft.text}
              onChange={(event) => setDraft({ ...draft, text: event.target.value })}
            />
            {draft.note.scope !== "window" && (
              <Tools
                element={draft.element.isConnected ? draft.element : locate(draft.note)}
                changes={draft.changes ?? NONE}
                onChanges={(next) => changed(null, next)}
                keep={keptAs(draft.note.id, "text")}
              />
            )}
            <Examples pictures={(draft.examples ?? []).map((name) => ({ name, png: pictures[name] }))} onRemove={(name) => void removeExample(null, name)} />
            <div className="armada-annotate__actions">
              <Button variant="ghost" size="sm" onClick={cancelDraft}>
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
        <Card box={openBox} view={view} label="Note" {...takesPictures(openNote)}>
          <p className="armada-annotate__meta">{chain(openNote)}</p>
          <Source note={openNote} />
          <p className="armada-annotate__text">{openNote.text}</p>
          {openNote.scope !== "window" && (
            <Tools element={locate(openNote)} changes={openNote.changes ?? NONE} onChanges={(next) => changed(openNote, next)} keep={keptAs(openNote.id, "text")} />
          )}
          <Examples pictures={(openNote.examples ?? []).map((name) => ({ name, png: pictures[name] }))} onRemove={(name) => void removeExample(openNote, name)} />
          {openNote.sent !== undefined && (
            <p className="armada-annotate__meta">
              Sent to Job {openNote.sent.title}
            </p>
          )}
          {openNote.sent === undefined && cannotSend !== null && <p className="armada-annotate__meta">{cannotSend}</p>}
          {/* The panel row's acts, so a note is acted on where it is pinned. */}
          <div className="armada-annotate__actions">
            {openNote.sent === undefined && openNote.status === "open" && (
              <>
                {cannotSend === null && (
                  <SplitButton
                    size="sm"
                    menuLabel="Send to a Job"
                    disabled={sending !== null}
                    onAction={() => void sendToSessionAs(openNote.id, [openNote], null)}
                    items={live.map((one) => ({ label: one.title, onSelect: () => void sendToSessionAs(openNote.id, [openNote], one) }))}
                  >
                    Start job
                  </SplitButton>
                )}
              </>
            )}
            <Button variant="ghost" size="sm" onClick={() => void setStatus(openNote, openNote.status === "open" ? "done" : "open")}>
              {openNote.status === "open" ? "Mark done" : "Reopen"}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => (deleting ? void remove(openNote) : setDeleting(true))}
            >
              {deleting ? "Press again to delete" : "Delete"}
            </Button>
          </div>
        </Card>
      )}

      {listing && (
        <aside
          className="armada-annotate__panel"
          aria-label="All notes"
          style={{ "--annotate-panel-bottom": `${panelBottom}px` } as CSSProperties}
        >
          {/* The panel's head stays put over the rows as they scroll, so its acts are in reach of a row ticked at the bottom. */}
          <div className="armada-annotate__head">
            <SearchField label="Search notes" value={search} onValue={setSearch} />
            <Tabs
              items={TABS.map((one) => {
                const count = notes.filter(one.holds).length;
                return { id: one.id, label: count === 0 ? one.label : `${one.label} ${count}` };
              })}
              value={tab}
              onChange={(id) => setTab(id as Tab)}
            />
            {listed.length > 0 && (
              <div className="armada-annotate__selection" role="group" aria-label="Selected notes">
                <Checkbox checked={allChosen} onChange={(event) => (event.target.checked ? select(listed.map((n) => n.id), true) : clearSelection())}>
                  Select all <span className="armada-annotate__count">{listed.length}</span>
                </Checkbox>
                {cannotSend === null && (
                  <SplitButton
                    size="sm"
                    variant="primary"
                    menuLabel="Send to a Job"
                    disabled={busy || chosenSendable.length === 0}
                    {...(chosenShotless > 0 ? { note: noShot(chosenShotless, chosenSendable.length) } : {})}
                    onAction={() => void sessionSelection(chosenSendable, null)}
                    items={live.map((one) => ({ label: one.title, onSelect: () => void sessionSelection(chosenSendable, one) }))}
                  >
                    Start job
                  </SplitButton>
                )}
                {chosen.length > 0 && (
                  <>
                    <span className="armada-annotate__selection-said">
                      {chosen.length} selected
                      {chosenSendable.length > 0 && leftOut.length > 0 ? `. Sends leave out ${chosen.length - chosenSendable.length}: ${leftOut.join(", ")}` : ""}
                    </span>
                    <div className="armada-annotate__actions">
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
          </div>
          {listed.length === 0 && <p className="armada-annotate__meta">No notes</p>}
          {groups.map(({ screen, list }) => {
            const key = screen ?? "";
            const shut = folded.has(key);
            return (
              <section key={key} className="armada-annotate__group" aria-label={screen ?? "No screen"}>
                <h2 className="armada-annotate__screen">
                  <button
                    type="button"
                    className="armada-annotate__fold"
                    aria-expanded={!shut}
                    onClick={() =>
                      setFolded((was) => {
                        const next = new Set(was);
                        if (shut) next.delete(key);
                        else next.add(key);
                        return next;
                      })
                    }
                  >
                    {shut ? <ChevronRight size={12} strokeWidth={2} aria-hidden /> : <ChevronDown size={12} strokeWidth={2} aria-hidden />}
                    <span>{screen ?? "No screen"}</span>
                    <span className="armada-annotate__count">{list.length}</span>
                  </button>
                </h2>
                {!shut && (
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
                              </span>
                            </button>
                          </Tooltip>
                          {note.sent !== undefined && (
                            <div className="armada-annotate__row-job">
                              <SentJob sent={note.sent} onOpen={openSession} />
                            </div>
                          )}
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
                            {/* A row without Start job keeps its column, so the acts after it line up down the list. */}
                            {dispatchable && cannotSend === null ? (
                              <SplitButton
                                size="sm"
                                menuLabel="Send to a Job"
                                disabled={sending !== null}
                                {...(shotless ? { note: NO_SHOT } : {})}
                                onAction={() => void sendToSessionAs(note.id, [note], null)}
                                items={live.map((one) => ({ label: one.title, onSelect: () => void sendToSessionAs(note.id, [note], one) }))}
                              >
                                Start job
                              </SplitButton>
                            ) : (
                              <span aria-hidden />
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
                )}
              </section>
            );
          })}
        </aside>
      )}

      <div ref={barRef} className="armada-annotate__bar" role="status">
        <span>Annotating</span>
        <Tooltip label="A note on the whole window, with the page, the open Job and Doctor's read" shortcut="⌥⌘S" asChild>
          <Button variant="secondary" size="sm" onClick={() => void shootWindow()}>
            Screenshot window
          </Button>
        </Tooltip>
        <button type="button" className="armada-annotate__status" aria-expanded={listing} onClick={() => setListing(!listing)}>
          {open} open, {done} done
          {offScreen > 0 ? `, ${offScreen} not on this screen` : ""}
        </button>
        {cannotSend === null && sendableHere > 0 && (
          <>
            <SplitButton
              size="sm"
              menuLabel="Send to a Job"
              disabled={sending !== null}
              onAction={() => void sendHereToSession(null)}
              items={live.map((one) => ({ label: one.title, onSelect: () => void sendHereToSession(one) }))}
            >
              Start job
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
type CardProps = {
  box: Box;
  view: Viewport;
  label: string;
  children: ReactNode;
  /** Where a paste or a drop on the card goes: a picture becomes one of the note's examples. */
  onPaste?: (event: ClipboardEvent<HTMLDivElement>) => void;
  onDragOver?: (event: DragEvent<HTMLDivElement>) => void;
  onDrop?: (event: DragEvent<HTMLDivElement>) => void;
};

function Card({ box, view, label, children, onPaste, onDragOver, onDrop }: CardProps) {
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
      onPaste={onPaste}
      onDragOver={onDragOver}
      onDrop={onDrop}
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

/** The shown notes by screen, open first, a screen named by its notes and "no screen" last. */
function groupsOf(shown: readonly Annotation[]): { screen: string | null; list: Annotation[] }[] {
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

/** `SessionRow ← SessionList ← Sessions`, then the selector, for the person checking what was picked. */
function chain(note: Annotation): string {
  if (note.scope === "window") return `Whole window · ${note.screen ?? "no screen marked"}`;
  const names = [note.component, ...note.owners].filter((n): n is string => n !== null).slice(0, 4);
  return `${names.length > 0 ? names.join(" ← ") : note.element.tag} · ${note.selector}`;
}
