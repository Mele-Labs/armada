// The Job browser's pane: the page the Job's agent drives with Armada's tools, which the person can take over and annotate. In the
// side panel with the Job's thread floating beside it (`BrowserSide.tsx`), or in a window of its own with the thread beside the page.
// The store says what it holds.

import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { AppWindowMac, Bot, ChevronLeft, ChevronRight, Crosshair, PanelRight, RefreshCw, UserCheck } from "lucide-react";
import { ACTION, Button, Input, Kbd, Textarea, Tooltip } from "@armada/components";
import type { ThreadCard } from "@armada/components";

import { useSessionsDraft } from "../sessions-draft";
import { closePanel, usePanel } from "../shared-panel/store";
import { browserToWindow, goBack, goForward, navigate, openBrowser, pinNote, reload, toggleDriver, urlOf, useJobBrowser, type BrowserPin, type JobBrowser } from "./store";
import "./job-browser.css";

/** Who drives, and what a press on the mark does: the mark is the toggle. */
const DRIVER = {
  agent: { said: "Agent driving. Take over", glyph: <Bot size={16} strokeWidth={2} aria-hidden /> },
  you: { said: "You're driving. Hand back to the agent", glyph: <UserCheck size={16} strokeWidth={2} aria-hidden /> },
} as const;

/** The walk window's note key, from the action registry rather than retyped: `⌥⌘C`. */
const CAPTURE = ACTION.capture_note?.shortcut ?? "";
/** The walk window's reload key, and the history act's. */
const KEYS = { reload: "⌘R", back: "⌘[", forward: "⌘]" } as const;

function IconAct({ label, shortcut, disabled, onClick, children }: { label: string; shortcut?: string; disabled?: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <Tooltip label={label} {...(shortcut === undefined ? {} : { shortcut })}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} disabled={disabled === true} onClick={onClick}>
        {children}
      </Button>
    </Tooltip>
  );
}

/** The words a part of the page is known by: its label, else its text, cut to a line. */
function nameOf(part: HTMLElement): string {
  const labelled = part.getAttribute("aria-label") ?? (part instanceof HTMLInputElement ? (part.labels?.[0]?.textContent ?? part.placeholder) : null);
  const words = (labelled ?? part.textContent ?? "").replace(/\s+/g, " ").trim();
  return words.length > 60 ? `${words.slice(0, 60)}…` : words;
}

/** The nearest part of the page a note can be pinned on. */
const PART = "button, input, label, h1, h2, h3, li, p, dt, dd";

type Picked = Omit<BrowserPin, "id" | "note" | "url">;

/** The note being written on a part, under it: Send puts it to the Job as a message. */
function NoteCard({ sessionId, url, picked, onDone }: { sessionId: string; url: string; picked: Picked; onDone: (note?: string) => void }) {
  const draft = useSessionsDraft();
  const [note, setNote] = useState("");
  const send = () => {
    const words = note.trim();
    if (words === "") return;
    void draft?.send(sessionId, { text: `Note on ${picked.on} at ${url}:\n\n${words}`, files: [], sketches: [], tags: [] });
    setNote("");
    onDone(words);
  };
  return (
    <div className="job-browser__note" role="dialog" aria-label={`Note on ${picked.on}`} style={{ top: picked.box.top + picked.box.height, left: picked.box.left }}>
      <Textarea
        aria-label="Note"
        keep={`job-browser:${sessionId}:${picked.on}`}
        rows={2}
        autoFocus
        value={note}
        onChange={(event) => setNote(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && event.metaKey) {
            event.preventDefault();
            send();
          } else if (event.key === "Escape") onDone();
        }}
      />
      <div className="job-browser__note-acts">
        <Button variant="ghost" size="sm" onClick={() => onDone()}>
          Cancel
          <Kbd>esc</Kbd>
        </Button>
        <Button variant="primary" size="sm" disabled={note.trim() === ""} onClick={send}>
          Send
          <Kbd>⌘↵</Kbd>
        </Button>
      </div>
    </div>
  );
}

/** The page, and the notes pinned on it. While Capture is on, a press on a part of the page picks it instead of pressing it. */
function Page({ sessionId, browser, annotating, onPicked }: { sessionId: string; browser: JobBrowser; annotating: boolean; onPicked: () => void }) {
  const frame = useRef<HTMLDivElement>(null);
  const [picked, setPicked] = useState<Picked>();
  const url = urlOf(browser);
  const pick = (event: MouseEvent<HTMLDivElement>) => {
    const into = frame.current;
    const part = event.target instanceof HTMLElement ? event.target.closest<HTMLElement>(PART) : null;
    if (!annotating && browser.driver === "you") return;
    // The agent's page takes no press but a note's.
    event.preventDefault();
    event.stopPropagation();
    if (!annotating || into === null || part === null || !into.contains(part)) return;
    const at = into.getBoundingClientRect();
    const box = part.getBoundingClientRect();
    setPicked({ on: nameOf(part), box: { top: box.top - at.top + into.scrollTop, left: box.left - at.left + into.scrollLeft, width: box.width, height: box.height } });
    onPicked();
  };
  return (
    <div ref={frame} className="job-browser__page" data-annotating={annotating ? "" : undefined} data-driver={browser.driver} onClickCapture={pick}>
      <div key={`${browser.at}:${browser.loads}`} className="job-browser__document">
        {browser.page?.(url)}
      </div>
      {browser.pins.filter((pin) => pin.url === url).map((pin) => (
        <Tooltip key={pin.id} label={pin.note} asChild>
          <span className="job-browser__pin" role="img" aria-label={`Note on ${pin.on}`} style={{ top: pin.box.top, left: pin.box.left, width: pin.box.width, height: pin.box.height }} />
        </Tooltip>
      ))}
      {picked === undefined ? null : (
        <>
          <span className="job-browser__pin" data-writing="" aria-hidden style={{ top: picked.box.top, left: picked.box.left, width: picked.box.width, height: picked.box.height }} />
          <NoteCard
            sessionId={sessionId}
            url={url}
            picked={picked}
            onDone={(note) => {
              if (note !== undefined) pinNote(sessionId, { ...picked, url, id: `${picked.on}:${browser.pins.length}`, note });
              setPicked(undefined);
            }}
          />
        </>
      )}
    </div>
  );
}

/** The address: the page's, and while the person drives, one he types, which Enter goes to. */
function Address({ sessionId, browser }: { sessionId: string; browser: JobBrowser }) {
  const url = urlOf(browser);
  const [typed, setTyped] = useState(url);
  useEffect(() => setTyped(url), [url]);
  const driving = browser.driver === "you";
  return (
    <Input
      aria-label="Address"
      mono
      readOnly={!driving}
      value={driving ? typed : url}
      onChange={(event) => setTyped(event.target.value)}
      onKeyDown={(event) => {
        if (event.key !== "Enter" || !driving) return;
        event.preventDefault();
        const to = typed.trim();
        if (to !== "" && to !== url) navigate(sessionId, to);
      }}
    />
  );
}

/**
 * The pane: who is driving, which a press on its mark changes; Back, Forward and Reload; the address; Capture, the walk window's
 * note control; and its move. In the side panel it moves to a window; in its window it moves back to the side panel.
 */
export function JobBrowserPane({ sessionId, browser, placed }: { sessionId: string; browser: JobBrowser; placed: "panel" | "window" }) {
  const [capturing, setCapturing] = useState(false);
  const driver = DRIVER[browser.driver];
  const keys = (event: KeyboardEvent<HTMLElement>) => {
    // The letter from its key's place, since ⌥ changes the character a key types.
    const key = event.code.startsWith("Key") ? event.code.slice(3) : event.key.length === 1 ? event.key.toUpperCase() : event.key;
    const chord = `${event.altKey ? "⌥" : ""}${event.metaKey ? "⌘" : ""}${key}`;
    const act = chord === CAPTURE ? () => setCapturing((was) => !was) : chord === KEYS.reload ? () => reload(sessionId) : chord === KEYS.back ? () => goBack(sessionId) : chord === KEYS.forward ? () => goForward(sessionId) : undefined;
    if (act === undefined) return;
    event.preventDefault();
    act();
  };
  return (
    <section className="job-browser" role="group" aria-label="Job browser" data-placed={placed} onKeyDown={keys}>
      <header className="job-browser__head">
        <Tooltip label={driver.said}>
          <Button variant="ghost" size="sm" iconOnly data-driver={browser.driver} aria-label={driver.said} onClick={() => toggleDriver(sessionId)}>
            {driver.glyph}
          </Button>
        </Tooltip>
        <span className="job-browser__acts">
          <IconAct label="Back" shortcut={KEYS.back} disabled={browser.at === 0} onClick={() => goBack(sessionId)}>
            <ChevronLeft size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct label="Forward" shortcut={KEYS.forward} disabled={browser.at >= browser.visited.length - 1} onClick={() => goForward(sessionId)}>
            <ChevronRight size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct label="Reload" shortcut={KEYS.reload} onClick={() => reload(sessionId)}>
            <RefreshCw size={16} strokeWidth={2} aria-hidden />
          </IconAct>
        </span>
        <Address sessionId={sessionId} browser={browser} />
        <span className="job-browser__acts">
          <Tooltip label={capturing ? "Capturing" : "Capture"} shortcut={CAPTURE}>
            <Button variant={capturing ? "primary" : "ghost"} size="sm" iconOnly aria-label="Capture" aria-pressed={capturing} onClick={() => setCapturing((was) => !was)}>
              <Crosshair size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          {placed === "panel" ? (
            <IconAct
              label="Open in a window"
              onClick={() => {
                closePanel();
                browserToWindow(sessionId);
              }}
            >
              <AppWindowMac size={16} strokeWidth={2} aria-hidden />
            </IconAct>
          ) : (
            <IconAct label="Move to the side panel" onClick={() => openBrowser(sessionId)}>
              <PanelRight size={16} strokeWidth={2} aria-hidden />
            </IconAct>
          )}
        </span>
      </header>
      <Page sessionId={sessionId} browser={browser} annotating={capturing} onPicked={() => setCapturing(false)} />
    </section>
  );
}

/** Flight control's Browser tile: lit while the Job browser shows, and absent where the Job has none. */
export type JobBrowserTile = { shown: boolean; press: () => void };

export function useJobBrowserTile(sessionId: string): JobBrowserTile | undefined {
  const browser = useJobBrowser(sessionId);
  const open = usePanel().open;
  if (browser === undefined) return undefined;
  return { shown: browser.windowed || (open?.kind === "browser" && open.sessionId === sessionId), press: () => openBrowser(sessionId) };
}

/** Each time the Job reached for a browser outside Bridge, a line in its thread where it did, with the press that shows its own. */
export function useBrowserReminders(sessionId: string): readonly ThreadCard[] {
  const browser = useJobBrowser(sessionId);
  return (browser?.reminders ?? []).map((one) => ({
    key: `browser:${one.id}`,
    at: one.at,
    node: (
      <div className="job-browser-reminder" role="note" aria-label={`Browser outside Bridge: ${one.tool}`}>
        <span className="job-browser-reminder__said">{`Opened a browser outside Bridge with ${one.tool}. This job has a Job browser.`}</span>
        <Button variant="secondary" size="sm" onClick={() => openBrowser(sessionId)}>
          Open Job browser
        </Button>
      </div>
    ),
  }));
}
