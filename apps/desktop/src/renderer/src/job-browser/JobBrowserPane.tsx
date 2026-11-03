// The Job browser's pane: a Browser tab's page, which the Job's agent works with Armada's tools and the person uses and annotates at any time. In its
// pane of the Job's panes (`job-panes/bodies.tsx`), or in a window of its own with the thread beside the page. The store says what it holds.

import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
} from "react";
import {
  Bell,
  Bot,
  ChevronLeft,
  ChevronRight,
  ClipboardList,
  Crosshair,
  Download,
  ExternalLink,
  Image,
  LockKeyhole,
  MapPin,
  Monitor,
  Network,
  PanelRight,
  Radio,
  RefreshCw,
  type LucideIcon,
} from "lucide-react";
import { ACTION, Button, Dialog, Input, Kbd, Popover, Textarea, Tooltip } from "@armada/components";
import type { ThreadCard } from "@armada/components";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import {
  CHOICES,
  SITE_PERMISSIONS,
  effectiveChoice,
  type Choice,
  type SitePermission,
} from "../../../shared/site-permissions";
import {
  browserToPane,
  goBack,
  goForward,
  jobBrowserApi,
  hostOf,
  navigate,
  openBrowserTab,
  pinNote,
  reload,
  setChoice,
  setDefault,
  urlOf,
  type BrowserPin,
  type JobBrowser,
} from "./store";
import { takesAnnotateChord } from "./chord";
import "./job-browser.css";

/** The agent's mark: at rest dim, and while an act runs lit and pulsing, its tooltip the act. A mark and not a press, held in a slot that does not move. */
function AgentMark({ acting }: { acting: string | undefined }) {
  const said = acting ?? "No agent act running";
  return (
    <Tooltip label={said} asChild>
      <span
        className="job-browser__mark"
        role="img"
        aria-label={said}
        data-acting={acting === undefined ? undefined : ""}
      >
        <Bot size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The walk window's note key, from the action registry rather than retyped: `⌥⌘C`. */
const CAPTURE = ACTION.capture_note?.shortcut ?? "";
/** The walk window's reload key, and the history act's. */
const KEYS = {
  reload: "⌘R",
  back: "⌘[",
  forward: "⌘]",
} as const;

function IconAct({
  label,
  shortcut,
  disabled,
  onClick,
  children,
}: {
  label: string;
  shortcut?: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip
      label={label}
      {...(shortcut === undefined
        ? {}
        : {
            shortcut,
          })}
    >
      <Button
        variant="ghost"
        size="sm"
        iconOnly
        aria-label={label}
        disabled={disabled === true}
        onClick={onClick}
      >
        {children}
      </Button>
    </Tooltip>
  );
}

/** The words a part of the page is known by: its label, else its text, cut to a line. */
function nameOf(part: HTMLElement): string {
  const labelled =
    part.getAttribute("aria-label") ??
    (part instanceof HTMLInputElement ? (part.labels?.[0]?.textContent ?? part.placeholder) : null);
  const words = (labelled ?? part.textContent ?? "").replace(/\s+/g, " ").trim();
  return words.length > 60 ? `${words.slice(0, 60)}…` : words;
}

/** The nearest part of the page a note can be pinned on. */
const PART = "button, input, label, h1, h2, h3, li, p, dt, dd";

type Picked = Omit<BrowserPin, "id" | "note" | "url">;

/** The note being written on a part, under it: Send puts it to the Job as the person's. */
function NoteCard({
  keep,
  picked,
  onSend,
  onDone,
}: {
  keep: string;
  picked: Picked;
  onSend: (words: string) => void;
  onDone: () => void;
}) {
  const [note, setNote] = useState("");
  const send = () => {
    const words = note.trim();
    if (words === "") return;
    setNote("");
    onSend(words);
  };
  return (
    <div
      className="job-browser__note"
      role="dialog"
      aria-label={`Note on ${picked.on}`}
      style={{
        top: picked.box.top + picked.box.height,
        left: picked.box.left,
      }}
    >
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
function FixturePage({
  sessionId,
  tabId,
  browser,
  draw,
  annotating,
  onPicked,
}: {
  sessionId: string;
  tabId: string;
  browser: JobBrowser;
  draw: (url: string, sessionId: string, tabId: string) => ReactNode;
  annotating: boolean;
  onPicked: () => void;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const draft = useSessionsDraft();
  const [picked, setPicked] = useState<Picked>();
  const url = urlOf(browser);
  const pick = (event: MouseEvent<HTMLDivElement>) => {
    const into = frame.current;
    const part =
      event.target instanceof HTMLElement ? event.target.closest<HTMLElement>(PART) : null;
    // The page takes every press but under Capture, which picks a part for a note.
    if (!annotating) return;
    event.preventDefault();
    event.stopPropagation();
    if (into === null || part === null || !into.contains(part)) return;
    const at = into.getBoundingClientRect();
    const box = part.getBoundingClientRect();
    setPicked({
      on: nameOf(part),
      box: {
        top: box.top - at.top + into.scrollTop,
        left: box.left - at.left + into.scrollLeft,
        width: box.width,
        height: box.height,
      },
    });
    onPicked();
  };
  return (
    <div
      ref={frame}
      className="job-browser__page"
      data-annotating={annotating ? "" : undefined}
      onClickCapture={pick}
    >
      <div key={`${url}:${browser.loads}`} className="job-browser__document">
        {draw(url, sessionId, tabId)}
      </div>
      {browser.pins
        .filter((pin) => pin.url === url)
        .map((pin) => (
          <Tooltip key={pin.id} label={pin.note} asChild>
            <span
              className="job-browser__pin"
              role="img"
              aria-label={`Note on ${pin.on}`}
              style={{
                top: pin.box.top,
                left: pin.box.left,
                width: pin.box.width,
                height: pin.box.height,
              }}
            />
          </Tooltip>
        ))}
      {picked === undefined ? null : (
        <>
          <span
            className="job-browser__pin"
            data-writing=""
            aria-hidden
            style={{
              top: picked.box.top,
              left: picked.box.left,
              width: picked.box.width,
              height: picked.box.height,
            }}
          />
          <NoteCard
            keep={`job-browser:${sessionId}:${picked.on}`}
            picked={picked}
            onSend={(note) => {
              void draft?.send(sessionId, {
                text: `Note on ${picked.on} at ${url}:\n\n${note}`,
                files: [],
                sketches: [],
                tags: [],
              });
              pinNote(sessionId, {
                ...picked,
                url,
                id: `${picked.on}:${browser.pins.length}`,
                note,
              });
              setPicked(undefined);
            }}
            onDone={() => setPicked(undefined)}
          />
        </>
      )}
    </div>
  );
}

type Box = {
  x: number;
  y: number;
  width: number;
  height: number;
};
const sameBox = (a: Box | undefined, b: Box): boolean =>
  a !== undefined && a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/**
 * The page main holds, a view it lies over this frame. **The frame is reported on every frame it moves**, as `PageFrame`'s is, since
 * a pane resized, zoomed or swept in by a layout moves it and no resize event of the frame's says so. Under Capture the view is hidden and its picture drawn here instead, so a
 * press picks a part of it: the walk window's own layer reads that part in the page, and the note goes to the Job as the person's.
 */
function LivePage({
  sessionId,
  tabId,
  api,
  capturing,
  onCaptured,
}: {
  sessionId: string;
  tabId: string;
  api: NonNullable<ReturnType<typeof jobBrowserApi>>;
  capturing: boolean;
  onCaptured: () => void;
}) {
  const frame = useRef<HTMLDivElement>(null);
  const [picture, setPicture] = useState<string>();
  const [picked, setPicked] = useState<Picked>();
  const [ring, setRing] = useState<Box | null>(null);
  // The app annotation layer, the site-settings popover, a dialog, a tab's menu and the drop zones of a tab being dragged all lie over the
  // page's native view, which would hide them, so while one is up the page is hidden and its picture shown, as under Capture.
  const [layerOn, setLayerOn] = useState(false);
  const held = capturing || layerOn;
  const aiming = useRef({
    at: null as {
      x: number;
      y: number;
    } | null,
    busy: false,
  });
  useEffect(() => {
    let moving = 0;
    let last: Box | undefined;
    const follow = () => {
      setLayerOn(
        document.querySelector(
          "[data-armada-annotate], .job-browser__site-panel, .armada-dialog-scrim, .armada-dropdown-menu__panel, .job-pane__drops",
        ) !== null,
      );
      const box = frame.current?.getBoundingClientRect();
      if (box !== undefined) {
        const now = {
          x: box.x,
          y: box.y,
          width: box.width,
          height: box.height,
        };
        if (!sameBox(last, now)) void api.place(sessionId, tabId, now);
        last = now;
      }
      moving = requestAnimationFrame(follow);
    };
    moving = requestAnimationFrame(follow);
    return () => {
      cancelAnimationFrame(moving);
      void api.unplace(sessionId, tabId);
    };
  }, [
    api,
    sessionId,
    tabId,
  ]);
  useEffect(() => {
    if (!held) return;
    let shown: string | undefined;
    let live = true;
    void api.freeze(sessionId, tabId).then((png) => {
      if (!live || png === null) return;
      shown = URL.createObjectURL(
        new Blob(
          [
            png as Uint8Array<ArrayBuffer>,
          ],
          {
            type: "image/png",
          },
        ),
      );
      setPicture(shown);
    });
    return () => {
      live = false;
      setPicture(undefined);
      setPicked(undefined);
      setRing(null);
      if (shown !== undefined) URL.revokeObjectURL(shown);
      void api.thaw(sessionId, tabId);
    };
  }, [
    api,
    sessionId,
    tabId,
    held,
  ]);
  const pick = (event: MouseEvent<HTMLDivElement>) => {
    const at = frame.current?.getBoundingClientRect();
    if (!capturing || picture === undefined || picked !== undefined || at === undefined) return;
    void api
      .hold(sessionId, tabId, event.clientX - at.left, event.clientY - at.top)
      .then((held) => {
        if (held !== null)
          setPicked({
            on: held.chain,
            box: {
              top: held.box.y,
              left: held.box.x,
              width: held.box.width,
              height: held.box.height,
            },
          });
      });
  };
  // The ring follows the part under the pointer in the live page, one ask in flight and at most one per frame.
  const hover = (event: PointerEvent<HTMLDivElement>) => {
    const at = frame.current?.getBoundingClientRect();
    if (!capturing || picture === undefined || picked !== undefined || at === undefined) return;
    aiming.current.at = {
      x: event.clientX - at.left,
      y: event.clientY - at.top,
    };
    if (aiming.current.busy) return;
    aiming.current.busy = true;
    requestAnimationFrame(() => {
      const to = aiming.current.at;
      if (to === null) {
        aiming.current.busy = false;
        return;
      }
      void api
        .aim(sessionId, tabId, to.x, to.y)
        .then(setRing)
        .finally(() => {
          aiming.current.busy = false;
        });
    });
  };
  return (
    <div
      ref={frame}
      className="job-browser__page"
      role="region"
      aria-label="Page"
      data-annotating={capturing ? "" : undefined}
      onClick={pick}
      onPointerMove={hover}
    >
      {picture === undefined ? null : <img className="job-browser__picture" src={picture} alt="" />}
      {ring === null || picked !== undefined || !capturing ? null : (
        <span
          className="job-browser__ring"
          aria-hidden
          style={{
            top: ring.y,
            left: ring.x,
            width: ring.width,
            height: ring.height,
          }}
        />
      )}
      {picked === undefined ? null : (
        <>
          <span
            className="job-browser__pin"
            data-writing=""
            aria-hidden
            style={{
              top: picked.box.top,
              left: picked.box.left,
              width: picked.box.width,
              height: picked.box.height,
            }}
          />
          <NoteCard
            keep={`job-browser:${sessionId}:${tabId}:${picked.on}`}
            picked={picked}
            onSend={(note) =>
              void api.note(sessionId, tabId, note).then((outcome) => outcome.ok && onCaptured())
            }
            onDone={() => setPicked(undefined)}
          />
        </>
      )}
    </div>
  );
}

/** The glyph each permission is known by. Camera and Microphone borrow Image and Radio, and Fullscreen borrows Monitor: the registry has none of them. */
const PERMISSION_GLYPH: Record<SitePermission, LucideIcon> = {
  local_network: Network,
  camera: Image,
  microphone: Radio,
  notifications: Bell,
  clipboard: ClipboardList,
  location: MapPin,
  downloads: Download,
  popups: ExternalLink,
  fullscreen: Monitor,
};

/** A row of segments of which one is chosen: arrows move the choice, and Tab stops once. */
function Segments<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly {
    value: T;
    label: string;
  }[];
  onChange: (value: T) => void;
}) {
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    const step =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (step === 0) return;
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
          onClick={() => onChange(one.value)}
        >
          {one.label}
        </button>
      ))}
    </div>
  );
}

const VIEWS = [
  {
    value: "site",
    label: "This site",
  },
  {
    value: "all",
    label: "All sites",
  },
] as const;
const CHOICE_OPTIONS = CHOICES.map((one) => ({
  value: one.choice,
  label: one.label,
}));

/** What a press on Allow for Local network asks first: the site's, or every site's. */
type LocalNetworkAsk = "site" | "all";

/** Site settings: what the page may use, for this site and as the default for every one. Allow on Local network is asked about first, by `onAsk`. */
function SiteSettingsPanel({
  sessionId,
  tabId,
  browser,
  onAsk,
}: {
  sessionId: string;
  tabId: string;
  browser: JobBrowser;
  onAsk: (scope: LocalNetworkAsk) => void;
}) {
  const [view, setView] = useState<"site" | "all">("site");
  const host = hostOf(urlOf(browser));
  return (
    <div className="job-browser__site-panel">
      <Segments label="Settings for" value={view} options={VIEWS} onChange={setView} />
      {view === "site" ? <span className="job-browser__site-host">{host}</span> : null}
      <ul className="job-browser__permissions">
        {SITE_PERMISSIONS.map(({ permission, label }) => {
          const Glyph = PERMISSION_GLYPH[permission];
          const value =
            view === "site"
              ? effectiveChoice(browser.site, host, permission)
              : browser.site.defaults[permission];
          return (
            <li key={permission} className="job-browser__permission">
              <Glyph size={16} strokeWidth={2} aria-hidden />
              <span className="job-browser__permission-label">{label}</span>
              <Segments<Choice>
                label={label}
                value={value}
                options={CHOICE_OPTIONS}
                onChange={(choice) => {
                  if (permission === "local_network" && choice === "allow" && value !== "allow")
                    onAsk(view);
                  else if (view === "site") setChoice(sessionId, tabId, host, permission, choice);
                  else setDefault(sessionId, tabId, permission, choice);
                }}
              />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** What each Local network confirmation says: the site's own, and the one for every site. */
const LOCAL_NETWORK = {
  site: (host: string) => ({
    title: `Allow local network for ${host}?`,
    body: "This site can reach apps on this Mac and devices on your network. That includes pages the agent opens on it.",
  }),
  all: () => ({
    title: "Allow local network on all sites?",
    body: "Every site can reach apps on this Mac and devices on your network. That includes pages the agent opens.",
  }),
} as const;

/**
 * The lock before the address: site settings in a popover. Allow on Local network is confirmed in a
 * Dialog first, held here beside the popover, which is put away while it is asked: Esc cancels, Enter confirms.
 */
function SiteLock({
  sessionId,
  tabId,
  browser,
}: {
  sessionId: string;
  tabId: string;
  browser: JobBrowser;
}) {
  const [asking, setAsking] = useState<{
    scope: LocalNetworkAsk;
    host: string;
  } | null>(null);
  const said =
    asking === null
      ? null
      : asking.scope === "site"
        ? LOCAL_NETWORK.site(asking.host)
        : LOCAL_NETWORK.all();
  const done = () => setAsking(null);
  return (
    <span className="job-browser__site">
      <Popover
        label="Site settings"
        trigger={
          <Tooltip label="Site settings">
            <Button variant="ghost" size="sm" iconOnly aria-label="Site settings">
              <LockKeyhole size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        }
      >
        {(close) => (
          <SiteSettingsPanel
            sessionId={sessionId}
            tabId={tabId}
            browser={browser}
            onAsk={(scope) => {
              setAsking({
                scope,
                host: hostOf(urlOf(browser)),
              });
              close();
            }}
          />
        )}
      </Popover>
      <Dialog
        open={asking !== null}
        title={said?.title ?? ""}
        tone="neutral"
        initialFocus="confirm"
        confirmLabel="Allow"
        onCancel={done}
        onConfirm={() => {
          if (asking === null) return;
          if (asking.scope === "site")
            setChoice(sessionId, tabId, asking.host, "local_network", "allow");
          else setDefault(sessionId, tabId, "local_network", "allow");
          done();
        }}
      >
        {said?.body}
      </Dialog>
    </span>
  );
}

/** The address: the page's, or one typed, which Enter goes to. */
function Address({
  sessionId,
  tabId,
  browser,
}: {
  sessionId: string;
  tabId: string;
  browser: JobBrowser;
}) {
  const url = urlOf(browser);
  const [typed, setTyped] = useState(url);
  useEffect(
    () => setTyped(url),
    [
      url,
    ],
  );
  return (
    <Input
      aria-label="Address"
      mono
      value={typed}
      onChange={(event) => setTyped(event.target.value)}
      onKeyDown={(event) => {
        if (event.key !== "Enter") return;
        event.preventDefault();
        const to = typed.trim();
        if (to !== "" && to !== url) navigate(sessionId, tabId, to);
      }}
    />
  );
}

/**
 * The pane: the agent's mark, lit while an act runs; Back, Forward and Reload; the address; Capture, the walk window's
 * note control. A page in a window of its own is moved back to its pane from the window, or from the pane it left, which draws none of it.
 */
export function JobBrowserPane({
  sessionId,
  tabId,
  browser,
  placed,
}: {
  sessionId: string;
  tabId: string;
  browser: JobBrowser;
  placed: "window" | "pane";
}) {
  const [capturing, setCapturing] = useState(false);
  const api = jobBrowserApi();
  // ⌥⌘C pressed in the page, which holds the keyboard where this pane cannot hear it.
  useEffect(
    () =>
      api?.onCapture((id, tab) => id === sessionId && tab === tabId && setCapturing((was) => !was)),
    [
      api,
      sessionId,
      tabId,
    ],
  );
  useEffect(() => takesAnnotateChord(() => setCapturing((was) => !was)), []);
  // The page is in the window the pane gave way to, and only one of the two places it.
  const away = placed === "pane" && browser.windowed;
  const keys = (event: KeyboardEvent<HTMLElement>) => {
    // The letter from its key's place, since ⌥ changes the character a key types.
    const key = event.code.startsWith("Key")
      ? event.code.slice(3)
      : event.key.length === 1
        ? event.key.toUpperCase()
        : event.key;
    const chord = `${event.altKey ? "⌥" : ""}${event.metaKey ? "⌘" : ""}${key}`;
    const act =
      chord === CAPTURE
        ? () => setCapturing((was) => !was)
        : chord === KEYS.reload
          ? () => reload(sessionId, tabId)
          : chord === KEYS.back
            ? () => goBack(sessionId, tabId)
            : chord === KEYS.forward
              ? () => goForward(sessionId, tabId)
              : undefined;
    if (act === undefined) return;
    event.preventDefault();
    act();
  };
  return (
    <section
      className="job-browser"
      role="group"
      aria-label="Job browser"
      data-placed={placed}
      onKeyDown={keys}
    >
      <header className="job-browser__head">
        <AgentMark acting={browser.acting} />
        <span className="job-browser__acts">
          <IconAct
            label="Back"
            shortcut={KEYS.back}
            disabled={!browser.back}
            onClick={() => goBack(sessionId, tabId)}
          >
            <ChevronLeft size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct
            label="Forward"
            shortcut={KEYS.forward}
            disabled={!browser.forward}
            onClick={() => goForward(sessionId, tabId)}
          >
            <ChevronRight size={16} strokeWidth={2} aria-hidden />
          </IconAct>
          <IconAct label="Reload" shortcut={KEYS.reload} onClick={() => reload(sessionId, tabId)}>
            <RefreshCw size={16} strokeWidth={2} aria-hidden />
          </IconAct>
        </span>
        {browser.page !== undefined || (api !== undefined && urlOf(browser) !== "") ? (
          <SiteLock sessionId={sessionId} tabId={tabId} browser={browser} />
        ) : null}
        <Address sessionId={sessionId} tabId={tabId} browser={browser} />
        <span className="job-browser__acts">
          <Tooltip label={capturing ? "Capturing" : "Capture"} shortcut={CAPTURE}>
            <Button
              variant={capturing ? "primary" : "ghost"}
              size="sm"
              iconOnly
              aria-label="Capture"
              aria-pressed={capturing}
              disabled={away}
              onClick={() => setCapturing((was) => !was)}
            >
              <Crosshair size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
          {placed === "window" || away ? (
            <IconAct label="Move to the pane" onClick={() => browserToPane(sessionId, tabId)}>
              <PanelRight size={16} strokeWidth={2} aria-hidden />
            </IconAct>
          ) : null}
        </span>
      </header>
      {away ? null : browser.page !== undefined ? (
        <FixturePage
          sessionId={sessionId}
          tabId={tabId}
          browser={browser}
          draw={browser.page}
          annotating={capturing}
          onPicked={() => setCapturing(false)}
        />
      ) : api !== undefined ? (
        <LivePage
          sessionId={sessionId}
          tabId={tabId}
          api={api}
          capturing={capturing}
          onCaptured={() => setCapturing(false)}
        />
      ) : null}
    </section>
  );
}

/** What the press on a reminder tells the Job, as the person's message. */
export const reminderSaid = (address: string | undefined): string =>
  `Use the Job browser for this.${address === undefined ? "" : ` You were on ${address}.`}`;

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
    const answered =
      pressed.has(one.id) ||
      (rows ?? [])
        .slice(at + 1)
        .some(
          (later) => later.kind === "message" && later.from.kind === "you" && later.text === said,
        );
    return [
      {
        key: `browser:${one.id}`,
        at: one.at,
        node: (
          <div
            className="job-browser-reminder"
            role="note"
            aria-label={`Browser outside Bridge: ${one.tool}`}
          >
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
                  openBrowserTab(sessionId);
                  void draft?.send(sessionId, {
                    text: said,
                    files: [],
                    sketches: [],
                    tags: [],
                  });
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
