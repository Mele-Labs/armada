// What stands in for Bridge's own window on a Job's server, in a browser that
// has no second window to open: the capture window's bar, with no Studio, over
// the page it would load. `prototype-fleet.ts` opens it. Dev-only.
//
// **A window, so the app beside it stays usable**: dragged by its strip and
// resized from its corner, and nothing under it is blocked — the real one is
// a window of its own, and the owner moves between Jobs with it open.
//
// **Capture lands on the Job**, as main's window does since protocol 23.18:
// Capture or ⌥⌘A arms it, a press inside the page picks what is under it, and
// the note is handed to `onNote`. The page is this mock on the same origin, so
// the frame's document is reachable — main asks its page through a script.

import { StrictMode, useEffect, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Button, CaptureBar } from "@armada/components";

/** What a press inside the page picked, as a walk note describes it. */
export type Picked = { element: string; selector: string; location: string };

export type WalkWindowOptions = {
  /** What a note goes to, as the bar names it: a Job's handle, or a Session's title. */
  job: string;
  /** Which of the two it is, as the note's Send says. Absent is a Job. */
  into?: "Job" | "Session";
  onNote: (said: string, picked: Picked) => void;
};

let held: { root: Root; host: HTMLElement } | null = null;

/** The mock itself on another scenario: the one address a browser page can serve as a page to show. */
export function mockPage(): string {
  // The roster is also read in node, by `scenario.test.ts`, where there is no page.
  if (typeof window === "undefined") return "http://localhost:41311/?scenario=every-state&walked";
  const url = new URL(window.location.href);
  url.search = "?scenario=every-state&walked";
  return url.toString();
}

/** Close what is open. */
export function closeWalkWindow(): void {
  document.documentElement.removeAttribute("data-walking");
  held?.root.unmount();
  held?.host.remove();
  held = null;
}

/**
 * Open the stand-in on `url`, replacing one already open — main raises the
 * one window per server, and a page has nothing to raise.
 *
 * **No page under test.** A frame of the tester's own page would start the
 * suite again inside itself, so a test sees the bar and an empty frame.
 */
export function openWalkWindow(run: string, url: string, options: WalkWindowOptions): void {
  closeWalkWindow();
  const host = document.createElement("div");
  host.className = "armada-mock-walk-window";
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <WalkWindow run={run} url={url} host={host} {...options} />
    </StrictMode>,
  );
  held = { root, host };
}

/** ⌥⌘A or ⌥⌘C, the two chords main's window takes before its page does. */
const isChord = (event: KeyboardEvent): boolean =>
  (event.code === "KeyA" || event.code === "KeyC") && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;

/**
 * The chord, claimed while a stand-in is open. **Listened for at import**, so
 * it runs ahead of the dev annotation layer `main.tsx` mounts after the app:
 * a real walk window is a window of its own and Bridge's layer never sees its
 * keys, but here both share one page and the layer would arm as well.
 */
let claimChord: (() => void) | null = null;
if (typeof window !== "undefined") {
  window.addEventListener(
    "keydown",
    (event) => {
      if (claimChord === null || !isChord(event)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      claimChord();
    },
    true,
  );
}

function WalkWindow({ run, url, host, job, into = "Job", onNote }: { run: string; url: string; host: HTMLElement } & WalkWindowOptions) {
  const testing = (import.meta as { env?: { MODE?: string } }).env?.MODE === "test";
  const frame = useRef<HTMLIFrameElement>(null);
  const [armed, setArmed] = useState(false);
  const [picked, setPicked] = useState<Picked | null>(null);
  const [said, setSaid] = useState("");
  const [kept, setKept] = useState(0);

  // **Bridge dims behind the window while it has focus**, as main dims its own
  // windows behind a focused walk window: a press here or in its page takes
  // focus, and a press anywhere on the app behind gives it back.
  useEffect(() => {
    const dim = (on: boolean) => document.documentElement.toggleAttribute("data-walking", on);
    const pressed = (event: PointerEvent) => dim(host.contains(event.target as Node));
    // The page inside the frame takes focus without a press reaching this document.
    const left = () => queueMicrotask(() => dim(document.activeElement === frame.current));
    dim(true);
    document.addEventListener("pointerdown", pressed, true);
    window.addEventListener("blur", left);
    return () => {
      dim(false);
      document.removeEventListener("pointerdown", pressed, true);
      window.removeEventListener("blur", left);
    };
  }, [host]);

  // The chord, from this page and from the page inside the frame — whose own
  // annotation layer it reaches first, being the frame's earliest listener
  // only after load, so it claims it there too.
  useEffect(() => {
    claimChord = () => setArmed((was) => !was);
    const toggle = (event: KeyboardEvent) => {
      if (!isChord(event)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      setArmed((was) => !was);
    };
    const inner = frame.current;
    const watchInner = () => inner?.contentWindow?.addEventListener("keydown", toggle, true);
    watchInner();
    inner?.addEventListener("load", watchInner);
    return () => {
      claimChord = null;
      inner?.contentWindow?.removeEventListener("keydown", toggle, true);
      inner?.removeEventListener("load", watchInner);
    };
  }, []);

  // Armed: the page is held still, the element under the pointer is ringed,
  // and a press picks it rather than acting on it.
  useEffect(() => {
    const page = frame.current?.contentDocument;
    if (!armed || page == null) return;
    let ringed: HTMLElement | null = null;
    const ring = (on: HTMLElement | null) => {
      if (ringed !== null) ringed.style.outline = "";
      ringed = on;
      // The framed page is this mock, so its own tokens draw the ring.
      if (on !== null) on.style.outline = "var(--focus-ring)";
    };
    const over = (event: MouseEvent) => ring(event.target as HTMLElement);
    const press = (event: MouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const target = event.target as HTMLElement;
      setPicked({ element: described(target), selector: selectorOf(target), location: page.location.pathname + page.location.search });
      setArmed(false);
    };
    page.addEventListener("mouseover", over, true);
    page.addEventListener("click", press, true);
    return () => {
      ring(null);
      page.removeEventListener("mouseover", over, true);
      page.removeEventListener("click", press, true);
    };
  }, [armed]);

  function save(): void {
    if (picked === null || said.trim() === "") return;
    onNote(said.trim(), picked);
    setKept((was) => was + 1);
    setPicked(null);
    setSaid("");
  }

  return (
    <div role="dialog" aria-label={`Bridge's window on ${run}`} className="armada-mock-walk-window__frame">
      <div className="armada-mock-walk-window__strip" onPointerDown={dragging(host)}>
        <span>
          {run} — drag to move, corner to resize
          {kept === 0 ? "" : ` · ${kept} ${kept === 1 ? "note" : "notes"} sent to the Job`}
        </span>
        <Button variant="secondary" size="sm" onClick={closeWalkWindow}>
          Close window
        </Button>
      </div>
      <CaptureBar
        run={run}
        address={new URL(url).origin}
        studio={null}
        job={job}
        serving
        armed={armed}
        framesRefused={0}
        onArm={setArmed}
        onReload={() => {
          if (frame.current !== null && !testing) frame.current.src = url;
        }}
        onFollowRefused={() => {}}
        binding={["⌥", "⌘", "A"]}
      />
      {picked === null ? null : (
        <form
          className="armada-mock-walk-window__note"
          aria-label="Note on what you picked"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <span>On {picked.element}</span>
          <textarea
            aria-label="What is wrong here"
            value={said}
            autoFocus
            onChange={(event) => setSaid(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && event.metaKey) save();
              if (event.key === "Escape") setPicked(null);
            }}
          />
          <div>
            <Button variant="ghost" size="sm" type="button" onClick={() => setPicked(null)}>
              Cancel
            </Button>
            <Button size="sm" type="submit" disabled={said.trim() === ""}>
              Send to the {into}
            </Button>
          </div>
        </form>
      )}
      <iframe ref={frame} title={`${run}, served`} src={testing ? "about:blank" : url} data-armed={armed || undefined} />
    </div>
  );
}

/** What was picked, the way a person names it: its role or tag and its words. */
function described(element: HTMLElement): string {
  const role = element.getAttribute("role") ?? element.tagName.toLowerCase();
  const words = (element.getAttribute("aria-label") ?? element.textContent ?? "").trim().replace(/\s+/g, " ");
  return words === "" ? role : `${role} “${words.length > 48 ? `${words.slice(0, 47)}…` : words}”`;
}

/** A path of tags and classes down to it, short enough to read. */
function selectorOf(element: HTMLElement): string {
  const parts: string[] = [];
  for (let at: HTMLElement | null = element; at !== null && parts.length < 4; at = at.parentElement) {
    const first = [...at.classList][0];
    parts.unshift(first === undefined ? at.tagName.toLowerCase() : `${at.tagName.toLowerCase()}.${first}`);
  }
  return parts.join(" > ");
}

/** Move the window by its strip. The frame is under the pointer too, so it is told to let go of it. */
function dragging(host: HTMLElement) {
  return (down: React.PointerEvent<HTMLDivElement>) => {
    if ((down.target as HTMLElement).closest("button") !== null) return;
    const frame = host.querySelector<HTMLElement>(".armada-mock-walk-window__frame");
    if (frame === null) return;
    const from = frame.getBoundingClientRect();
    const start = { x: down.clientX - from.left, y: down.clientY - from.top };
    frame.style.pointerEvents = "none";
    const move = (at: PointerEvent) => {
      frame.style.left = `${Math.max(0, at.clientX - start.x)}px`;
      frame.style.top = `${Math.max(0, at.clientY - start.y)}px`;
      frame.style.right = "auto";
    };
    const up = () => {
      frame.style.pointerEvents = "";
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
}
