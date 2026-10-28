// The Job browser's pane: the page the Job's agent drives with Armada's tools, which the person can take over and annotate. Docked
// in the Cockpit between the Job and the merge line, as the Job's shell is, or in a window of its own. The store says what it holds.

import { useRef, useState, type MouseEvent, type ReactNode } from "react";
import { AppWindowMac, Bot, PanelBottom, PanelBottomClose, UserCheck } from "lucide-react";
import { Button, Input, Kbd, Textarea, Tooltip } from "@armada/components";
import type { ThreadCard } from "@armada/components";

import { useSessionsDraft } from "../sessions-draft";
import { browserToWindow, dockBrowser, handBack, pinNote, showBrowser, takeOver, useJobBrowser, type BrowserPin, type JobBrowser } from "./store";
import "./job-browser.css";

const DRIVER = { agent: { said: "Agent driving", glyph: <Bot size={16} strokeWidth={2} aria-hidden /> }, you: { said: "You're driving", glyph: <UserCheck size={16} strokeWidth={2} aria-hidden /> } } as const;

function IconAct({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return (
    <Tooltip label={label}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} onClick={onClick}>
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

type Picked = Omit<BrowserPin, "id" | "note">;

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

/** The page, and the notes pinned on it. While Annotate is on, a press on a part of the page picks it instead of pressing it. */
function Page({ sessionId, browser, annotating, onPicked }: { sessionId: string; browser: JobBrowser; annotating: boolean; onPicked: () => void }) {
  const frame = useRef<HTMLDivElement>(null);
  const [picked, setPicked] = useState<Picked>();
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
      {browser.page}
      {browser.pins.map((pin) => (
        <Tooltip key={pin.id} label={pin.note} asChild>
          <span className="job-browser__pin" role="img" aria-label={`Note on ${pin.on}`} style={{ top: pin.box.top, left: pin.box.left, width: pin.box.width, height: pin.box.height }} />
        </Tooltip>
      ))}
      {picked === undefined ? null : (
        <>
          <span className="job-browser__pin" data-writing="" aria-hidden style={{ top: picked.box.top, left: picked.box.left, width: picked.box.width, height: picked.box.height }} />
          <NoteCard
            sessionId={sessionId}
            url={browser.url}
            picked={picked}
            onDone={(note) => {
              if (note !== undefined) pinNote(sessionId, { ...picked, id: `${picked.on}:${browser.pins.length}`, note });
              setPicked(undefined);
            }}
          />
        </>
      )}
    </div>
  );
}

/**
 * The pane: who is driving, the page's address, Take over or Hand back, Annotate, and the moves. Docked, it moves to a window and
 * hides; in its window, Dock brings it back and the window's own close puts it away.
 */
export function JobBrowserPane({ sessionId, browser, placed }: { sessionId: string; browser: JobBrowser; placed: "docked" | "window" }) {
  const [annotating, setAnnotating] = useState(false);
  const driver = DRIVER[browser.driver];
  return (
    <section className="job-browser" role="group" aria-label="Job browser" data-placed={placed}>
      <header className="job-browser__head">
        <Tooltip label={driver.said}>
          <span className="job-browser__driver" data-driver={browser.driver} role="img" aria-label={driver.said}>
            {driver.glyph}
          </span>
        </Tooltip>
        <Input aria-label="Address" mono readOnly value={browser.url} />
        <span className="job-browser__acts">
          {browser.driver === "agent" ? (
            <Button variant="secondary" size="sm" onClick={() => takeOver(sessionId)}>
              Take over
            </Button>
          ) : (
            <Button variant="secondary" size="sm" onClick={() => handBack(sessionId)}>
              Hand back
            </Button>
          )}
          <Button variant="ghost" size="sm" aria-pressed={annotating} onClick={() => setAnnotating((was) => !was)}>
            Annotate
          </Button>
          {placed === "docked" ? (
            <>
              <IconAct label="Open in a window" onClick={() => browserToWindow(sessionId)}>
                <AppWindowMac size={16} strokeWidth={2} aria-hidden />
              </IconAct>
              <IconAct label="Hide" onClick={() => showBrowser(sessionId, false)}>
                <PanelBottomClose size={16} strokeWidth={2} aria-hidden />
              </IconAct>
            </>
          ) : (
            <IconAct label="Dock" onClick={() => dockBrowser(sessionId)}>
              <PanelBottom size={16} strokeWidth={2} aria-hidden />
            </IconAct>
          )}
        </span>
      </header>
      <Page sessionId={sessionId} browser={browser} annotating={annotating} onPicked={() => setAnnotating(false)} />
    </section>
  );
}

/** The Job browser docked in the Cockpit, between the Job and the merge line. Null where the Job has none, or it is put away. */
export function BrowserPanel({ sessionId }: { sessionId: string }) {
  const browser = useJobBrowser(sessionId);
  if (browser === undefined || !browser.shown) return null;
  return (
    <section className="deck-panel deck-browser" aria-label="Job browser">
      <JobBrowserPane sessionId={sessionId} browser={browser} placed="docked" />
    </section>
  );
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
        <Button variant="secondary" size="sm" onClick={() => showBrowser(sessionId, true)}>
          Open Job browser
        </Button>
      </div>
    ),
  }));
}
