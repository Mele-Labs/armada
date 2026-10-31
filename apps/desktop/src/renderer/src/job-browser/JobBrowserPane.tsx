// The Job browser's pane: the page the Job's agent drives with Armada's tools, which the person can take over and annotate. In the
// side panel with the Job's thread floating beside it (`BrowserSide.tsx`), or in a window of its own with the thread beside the page.
// The store says what it holds.

import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode } from "react";
import { AppWindowMac, Bell, Bot, ChevronLeft, ChevronRight, ClipboardList, Crosshair, Download, Image, LockKeyhole, MapPin, Network, PanelLeftClose, PanelLeftOpen, PanelRight, Radio, RefreshCw, UserCheck, type LucideIcon } from "lucide-react";
import { ACTION, Button, Input, Kbd, Popover, Textarea, Tooltip } from "@armada/components";
import type { ThreadCard } from "@armada/components";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { closePanel, usePanel } from "../shared-panel/store";
import { CHOICES, SITE_PERMISSIONS, effectiveChoice, type Choice, type SitePermission } from "../../../shared/site-permissions";
import { answerRequest, browserToWindow, dismissRequest, goBack, goForward, jobBrowserApi, hostOf, navigate, openBrowser, pinNote, reload, setChoice, setDefault, showThread, toggleDriver, urlOf, useJobBrowser, type BrowserPin, type JobBrowser } from "./store";
import { takesAnnotateChord } from "./chord";
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

/** The note being written on a part, under it: Send puts it to the Job as the person's. */
function NoteCard({ keep, picked, onSend, onDone }: { keep: string; picked: Picked; onSend: (words: string) => void; onDone: () => void }) {
  const [note, setNote] = useState("");
  const send = () => {
    const words = note.trim();
    if (words === "") return;
    setNote("");
    onSend(words);
  };
  return (
    <div className="job-browser__note" role="dialog" aria-label={`Note on ${picked.on}`} style={{ top: picked.box.top + picked.box.height, left: picked.box.left }}>
      <Textarea
        aria-label="Note"
        keep={keep}
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
        <Button variant="ghost" size="sm" onClick={onDone}>
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

/** **The mock's** page, drawn from fixtures, and the notes pinned on it. While Capture is on, a press on a part of the page picks it instead of pressing it. */
function FixturePage({ sessionId, browser, draw, annotating, onPicked }: { sessionId: string; browser: JobBrowser; draw: (url: string, sessionId: string) => ReactNode; annotating: boolean; onPicked: () => void }) {
  const frame = useRef<HTMLDivElement>(null);
  const draft = useSessionsDraft();
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
      <div key={`${url}:${browser.loads}`} className="job-browser__document">
        {draw(url, sessionId)}
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
            keep={`job-browser:${sessionId}:${picked.on}`}
            picked={picked}
            onSend={(note) => {
              void draft?.send(sessionId, { text: `Note on ${picked.on} at ${url}:\n\n${note}`, files: [], sketches: [], tags: [] });
              pinNote(sessionId, { ...picked, url, id: `${picked.on}:${browser.pins.length}`, note });
              setPicked(undefined);
            }}
            onDone={() => setPicked(undefined)}
          />
        </>
      )}
    </div>
  );
}

type Box = { x: number; y: number; width: number; height: number };
const sameBox = (a: Box | undefined, b: Box): boolean => a !== undefined && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/**
 * The page main holds, a view it lies over this frame. **The frame is reported on every frame it moves**, as `PageFrame`'s is, since
 * the side panel travels in and no resize event says so. Under Capture the view is hidden and its picture drawn here instead, so a
 * press picks a part of it: the walk window's own layer reads that part in the page, and the note goes to the Job as the person's.
 */
function LivePage({ sessionId, api, capturing, onCaptured }: { sessionId: string; api: NonNullable<ReturnType<typeof jobBrowserApi>>; capturing: boolean; onCaptured: () => void }) {
  const frame = useRef<HTMLDivElement>(null);
  const [picture, setPicture] = useState<string>();
  const [picked, setPicked] = useState<Picked>();
  const [ring, setRing] = useState<Box | null>(null);
  // The app annotation layer, the site-settings popover and the page's ask all lie over the page's native view, which would hide them,
  // so while one is up the page is hidden and its picture shown, as under Capture.
  const [layerOn, setLayerOn] = useState(false);
  const held = capturing || layerOn;
  const aiming = useRef({ at: null as { x: number; y: number } | null, busy: false });
  useEffect(() => {
    let moving = 0;
    let last: Box | undefined;
    const follow = () => {
      setLayerOn(document.querySelector("[data-armada-annotate], .job-browser__site-panel, .job-browser__ask") !== null);
      const box = frame.current?.getBoundingClientRect();
      if (box !== undefined) {
        const now = { x: box.x, y: box.y, width: box.width, height: box.height };
        if (!sameBox(last, now)) void api.place(sessionId, now);
        last = now;
      }
      moving = requestAnimationFrame(follow);
    };
    moving = requestAnimationFrame(follow);
    return () => {
      cancelAnimationFrame(moving);
      void api.unplace(sessionId);
    };
  }, [api, sessionId]);
  useEffect(() => {
    if (!held) return;
    let shown: string | undefined;
    let live = true;
    void api.freeze(sessionId).then((png) => {
      if (!live || png === null) return;
      shown = URL.createObjectURL(new Blob([png as Uint8Array<ArrayBuffer>], { type: "image/png" }));
      setPicture(shown);
    });
    return () => {
      live = false;
      setPicture(undefined);
      setPicked(undefined);
      setRing(null);
      if (shown !== undefined) URL.revokeObjectURL(shown);
      void api.thaw(sessionId);
    };
  }, [api, sessionId, held]);
  const pick = (event: MouseEvent<HTMLDivElement>) => {
    const at = frame.current?.getBoundingClientRect();
    if (!capturing || picture === undefined || picked !== undefined || at === undefined) return;
    void api.hold(sessionId, event.clientX - at.left, event.clientY - at.top).then((held) => {
      if (held !== null) setPicked({ on: held.chain, box: { top: held.box.y, left: held.box.x, width: held.box.width, height: held.box.height } });
    });
  };
  // The ring follows the part under the pointer in the live page, one ask in flight and at most one per frame.
  const hover = (event: PointerEvent<HTMLDivElement>) => {
    const at = frame.current?.getBoundingClientRect();
    if (!capturing || picture === undefined || picked !== undefined || at === undefined) return;
    aiming.current.at = { x: event.clientX - at.left, y: event.clientY - at.top };
    if (aiming.current.busy) return;
    aiming.current.busy = true;
    requestAnimationFrame(() => {
      const to = aiming.current.at;
      if (to === null) {
        aiming.current.busy = false;
        return;
      }
      void api
        .aim(sessionId, to.x, to.y)
        .then(setRing)
        .finally(() => {
          aiming.current.busy = false;
        });
    });
  };
  return (
    <div ref={frame} className="job-browser__page" role="region" aria-label="Page" data-annotating={capturing ? "" : undefined} onClick={pick} onPointerMove={hover}>
      {picture === undefined ? null : <img className="job-browser__picture" src={picture} alt="" />}
      {ring === null || picked !== undefined || !capturing ? null : <span className="job-browser__ring" aria-hidden style={{ top: ring.y, left: ring.x, width: ring.width, height: ring.height }} />}
      {picked === undefined ? null : (
        <>
          <span className="job-browser__pin" data-writing="" aria-hidden style={{ top: picked.box.top, left: picked.box.left, width: picked.box.width, height: picked.box.height }} />
          <NoteCard
            keep={`job-browser:${sessionId}:${picked.on}`}
            picked={picked}
            onSend={(note) => void api.note(sessionId, note).then((outcome) => outcome.ok && onCaptured())}
            onDone={() => setPicked(undefined)}
          />
        </>
      )}
    </div>
  );
}

/** The glyph each permission is known by. Camera and Microphone borrow Image and Radio: the registry has no camera or microphone. */
const PERMISSION_GLYPH: Record<SitePermission, LucideIcon> = {
  local_network: Network,
  camera: Image,
  microphone: Radio,
  notifications: Bell,
  clipboard: ClipboardList,
  location: MapPin,
  downloads: Download,
};

/** A row of segments of which one is chosen: arrows move the choice, and Tab stops once. */
function Segments<T extends string>({ label, value, options, disabled, onChange }: { label: string; value: T; options: readonly { value: T; label: string }[]; disabled?: boolean; onChange: (value: T) => void }) {
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (step === 0 || disabled === true) return;
    event.preventDefault();
    const at = options.findIndex((one) => one.value === value);
    const next = options[(at + step + options.length) % options.length]!;
    onChange(next.value);
    event.currentTarget.querySelector<HTMLElement>(`[data-value="${next.value}"]`)?.focus();
  };
  return (
    <div className="job-browser__segments" role="radiogroup" aria-label={label} onKeyDown={move}>
      {options.map((one) => (
        <button
          key={one.value}
          type="button"
          role="radio"
          className="job-browser__segment"
          data-value={one.value}
          aria-checked={one.value === value}
          tabIndex={one.value === value ? 0 : -1}
          disabled={disabled === true}
          onClick={() => onChange(one.value)}
        >
          {one.label}
        </button>
      ))}
    </div>
  );
}

const VIEWS = [
  { value: "site", label: "This site" },
  { value: "all", label: "All sites" },
] as const;
const CHOICE_OPTIONS = CHOICES.map((one) => ({ value: one.choice, label: one.label }));

/** Site settings: what the page may use, for this site and as the default for every one. Changing them takes the page over. */
function SiteSettingsPanel({ sessionId, browser }: { sessionId: string; browser: JobBrowser }) {
  const [view, setView] = useState<"site" | "all">("site");
  const host = hostOf(urlOf(browser));
  const locked = browser.driver !== "you";
  return (
    <div className="job-browser__site-panel">
      <Segments label="Settings for" value={view} options={VIEWS} onChange={setView} />
      {view === "site" ? <span className="job-browser__site-host">{host}</span> : null}
      <ul className="job-browser__permissions">
        {SITE_PERMISSIONS.map(({ permission, label }) => {
          const Glyph = PERMISSION_GLYPH[permission];
          return (
            <li key={permission} className="job-browser__permission">
              <Glyph size={16} strokeWidth={2} aria-hidden />
              <span className="job-browser__permission-label">{label}</span>
              <Segments<Choice>
                label={label}
                value={view === "site" ? effectiveChoice(browser.site, host, permission) : browser.site.defaults[permission]}
                options={CHOICE_OPTIONS}
                disabled={locked}
                onChange={(choice) => (view === "site" ? setChoice(sessionId, host, permission, choice) : setDefault(sessionId, permission, choice))}
              />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** The page's ask, under the lock. Esc leaves the site undecided. */
function AskBubble({ sessionId, browser }: { sessionId: string; browser: JobBrowser }) {
  const asked = browser.site.pending;
  const open = asked !== null;
  useEffect(() => {
    if (!open) return;
    const onKey = (event: globalThis.KeyboardEvent) => event.key === "Escape" && dismissRequest(sessionId);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, sessionId]);
  if (asked === null) return null;
  const label = SITE_PERMISSIONS.find((one) => one.permission === asked.permission)?.label ?? asked.permission;
  return (
    <div className="job-browser__ask" role="alertdialog" aria-label="Site request">
      <span>{`${asked.host} wants to use ${label}`}</span>
      <div className="job-browser__ask-acts">
        <Button variant="ghost" size="sm" onClick={() => answerRequest(sessionId, "block")}>
          Block
        </Button>
        <Button variant="primary" size="sm" onClick={() => answerRequest(sessionId, "allow")}>
          Allow
        </Button>
      </div>
    </div>
  );
}

/** The lock before the address: site settings in a popover, and the page's ask under it. */
function SiteLock({ sessionId, browser }: { sessionId: string; browser: JobBrowser }) {
  const said = browser.driver === "you" ? "Site settings" : "Take over to change site settings";
  return (
    <span className="job-browser__site">
      <Popover
        label="Site settings"
        trigger={
          <Tooltip label={said}>
            <Button variant="ghost" size="sm" iconOnly aria-label={said}>
              <LockKeyhole size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        }
      >
        <SiteSettingsPanel sessionId={sessionId} browser={browser} />
      </Popover>
      <AskBubble sessionId={sessionId} browser={browser} />
    </span>
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
 * The pane: in the side panel, the toggle for the Job's thread; who is driving, which a press on its mark changes; Back, Forward and Reload; the address; Capture, the walk window's
 * note control; and its move. In the side panel it moves to a window; in its window it moves back to the side panel.
 */
export function JobBrowserPane({ sessionId, browser, placed }: { sessionId: string; browser: JobBrowser; placed: "panel" | "window" | "pane" }) {
  const [capturing, setCapturing] = useState(false);
  const api = jobBrowserApi();
  // ⌥⌘C pressed in the page, which holds the keyboard where this pane cannot hear it.
  useEffect(() => api?.onCapture((id) => id === sessionId && setCapturing((was) => !was)), [api, sessionId]);
  useEffect(() => takesAnnotateChord(() => setCapturing((was) => !was)), []);
  const driver = DRIVER[browser.driver];
  const threadShown = !browser.threadHidden;
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
        {placed === "panel" ? (
          <Tooltip label={threadShown ? "Hide Job" : "Show Job"}>
            <Button variant="ghost" size="sm" iconOnly aria-label={threadShown ? "Hide Job" : "Show Job"} aria-pressed={threadShown} onClick={() => showThread(sessionId, !threadShown)}>
              {threadShown ? <PanelLeftClose size={16} strokeWidth={2} aria-hidden /> : <PanelLeftOpen size={16} strokeWidth={2} aria-hidden />}
            </Button>
          </Tooltip>
        ) : null}
        <Tooltip label={driver.said}>
          <Button variant="ghost" size="sm" iconOnly data-driver={browser.driver} aria-label={driver.said} onClick={() => toggleDriver(sessionId)}>
            {driver.glyph}
          </Button>
        </Tooltip>
        <span className="job-browser__acts">
          <IconAct label="Back" shortcut={KEYS.back} disabled={!browser.back} onClick={() => goBack(sessionId)}>
            <ChevronLeft size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct label="Forward" shortcut={KEYS.forward} disabled={!browser.forward} onClick={() => goForward(sessionId)}>
            <ChevronRight size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct label="Reload" shortcut={KEYS.reload} onClick={() => reload(sessionId)}>
            <RefreshCw size={16} strokeWidth={2} aria-hidden />
          </IconAct>
        </span>
        {browser.page !== undefined || (api !== undefined && urlOf(browser) !== "") ? <SiteLock sessionId={sessionId} browser={browser} /> : null}
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
          ) : placed === "window" ? (
            <IconAct label="Move to the side panel" onClick={() => openBrowser(sessionId)}>
              <PanelRight size={16} strokeWidth={2} aria-hidden />
            </IconAct>
          ) : null}
        </span>
      </header>
      {browser.page !== undefined ? (
        <FixturePage sessionId={sessionId} browser={browser} draw={browser.page} annotating={capturing} onPicked={() => setCapturing(false)} />
      ) : api !== undefined ? (
        <LivePage sessionId={sessionId} api={api} capturing={capturing} onCaptured={() => setCapturing(false)} />
      ) : null}
    </section>
  );
}

/** Flight control's Browser tile: lit while the Job browser shows. A press where the Job has none makes an empty one. */
export type JobBrowserTile = { shown: boolean; press: () => void };

export function useJobBrowserTile(sessionId: string): JobBrowserTile {
  const browser = useJobBrowser(sessionId);
  const open = usePanel().open;
  const shown = browser !== undefined && (browser.windowed || (open?.kind === "browser" && open.sessionId === sessionId));
  return { shown, press: () => openBrowser(sessionId) };
}

/** What the press on a reminder tells the Job, as the person's message. */
export const reminderSaid = (address: string | undefined): string => `Use the Job browser for this.${address === undefined ? "" : ` You were on ${address}.`}`;

/**
 * Each time the Job reached for a browser outside Bridge, a line in its thread where it did, with the press that shows its own and
 * tells the Job to use it. **Answered is read off the thread**, the person's message after the row, since the row carries no answer.
 */
export function useBrowserReminders(sessionId: string): readonly ThreadCard[] {
  const rows = useSessions().find((one) => one.id === sessionId)?.rows;
  const draft = useSessionsDraft();
  // Held in a ref as well as state: a second press before the line redraws would otherwise send it again.
  const sending = useRef(new Set<string>());
  const [pressed, setPressed] = useState<ReadonlySet<string>>(() => new Set());
  return (rows ?? []).flatMap((one, at) => {
    if (one.kind !== "browser_reminder") return [];
    const said = reminderSaid(one.address);
    const answered = pressed.has(one.id) || (rows ?? []).slice(at + 1).some((later) => later.kind === "message" && later.from.kind === "you" && later.text === said);
    return [
      {
        key: `browser:${one.id}`,
        at: one.at,
        node: (
          <div className="job-browser-reminder" role="note" aria-label={`Browser outside Bridge: ${one.tool}`}>
            <span className="job-browser-reminder__said">{`Opened a browser outside Bridge with ${one.tool}. This job has a Job browser.`}</span>
            {answered ? (
              <span className="job-browser-reminder__sent">Sent to the job</span>
            ) : (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  if (sending.current.has(one.id)) return;
                  sending.current.add(one.id);
                  setPressed(new Set(sending.current));
                  openBrowser(sessionId);
                  void draft?.send(sessionId, { text: said, files: [], sketches: [], tags: [] });
                }}
              >
                Open Job browser
              </Button>
            )}
          </div>
        ),
      },
    ];
  });
}
