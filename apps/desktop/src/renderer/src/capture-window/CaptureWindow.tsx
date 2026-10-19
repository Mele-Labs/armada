// The capture window's own document — #1294, `docs/practices/capture-window.md`.
//
// **Bridge's chrome, drawn outside the page it sits above.** The bar names the
// Run, the address and the Studio; while capture is armed this view covers the
// whole window, so the outline and the note card are drawn where the page
// cannot read them, restyle them or forge them — and a press never reaches the
// app underneath.
//
// Nothing here holds a capture. What a person points at is read by main in the
// page's own world and bounded there; this gets a box and one line.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  ACTION,
  CaptureBar,
  StudioCapture as CaptureOverlay,
  WalkGuide,
  WindowQuestion,
  type CaptureBarApproval,
  type CaptureBarQueue,
  type CaptureBox,
} from "@armada/components";
import { said } from "@armada/screens/src/copy";
import { UNTITLED_STUDIO } from "@armada/screens/src/studio";

import type { CaptureAimed, CaptureAnswer, CaptureHeld, CaptureQueueKey, CaptureWindowState } from "../../../shared/capture-window";

/** The binding, from the action registry rather than retyped. `⌥⌘C`. */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

/** What each refusal is called on the bar. A page moves in five ways and each says which. */
const REFUSED_SAID: Record<string, string> = {
  navigation: "Navigation",
  redirect: "Redirect",
  frame: "Frame",
  window: "Popup",
  download: "Download",
};

/**
 * Each of the queue's keys as the bar's own handler reads it, so a key main heard on the page is the press made on the bar.
 * Letters carry their `code`, which the handler goes by.
 */
const QUEUE_PRESS: Record<CaptureQueueKey, KeyboardEventInit> = {
  open: { key: "ArrowDown", code: "ArrowDown", metaKey: true, altKey: true },
  takes: { key: "t", code: "KeyT", metaKey: true, altKey: true },
  next: { key: "ArrowRight", code: "ArrowRight", metaKey: true, altKey: true },
  previous: { key: "ArrowLeft", code: "ArrowLeft", metaKey: true, altKey: true },
  reload: { key: "r", code: "KeyR", metaKey: true },
  approve: { key: "Enter", code: "Enter", metaKey: true },
};

/** How often the pointer is asked about, in frames. One ask per frame is one round trip per frame. */
type Pointer = { x: number; y: number };

export type CaptureWindowApi = {
  read: (onChanged: (state: CaptureWindowState) => void) => () => void;
  arm: (on: boolean) => Promise<CaptureWindowState | null>;
  aim: (x: number, y: number) => Promise<CaptureAimed | null>;
  hold: (x: number, y: number) => Promise<CaptureHeld | null>;
  release: () => Promise<void>;
  save: (said: string) => Promise<{ ok: boolean } & Record<string, unknown>>;
  reload: () => Promise<void>;
  followRefused: () => Promise<void>;
  approve: () => Promise<{ ok: boolean } & Record<string, unknown>>;
  scroll: (wheel: { x: number; y: number; deltaX: number; deltaY: number }) => void;
  showAsk: (on: boolean) => Promise<void>;
  answerAsk: (answer: CaptureAnswer) => Promise<{ ok: boolean } & Record<string, unknown>>;
  pick: (at: number) => Promise<void>;
  panel: (open: boolean) => Promise<void>;
  openOwner: () => Promise<void>;
  walkNext: () => Promise<void>;
  walkBack: () => Promise<void>;
  walkAutoplay: (on: boolean) => Promise<void>;
  walkReplay: () => Promise<void>;
  walkGuide: (shown: boolean) => Promise<void>;
  /** Absent where the bar is not a walk window's, which holds one page. */
  walkTo?: (at: number) => Promise<void>;
  onKey?: (onKey: (key: CaptureQueueKey) => void) => () => void;
};

export function CaptureWindow({ api }: { api: CaptureWindowApi }) {
  const [state, setState] = useState<CaptureWindowState | null>(null);
  const [aimed, setAimed] = useState<CaptureAimed | null>(null);
  const [held, setHeld] = useState<CaptureHeld | null>(null);
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  // Approving one take approves it alone: another on screen is Approve again, and pressing it moves the approval. It is the
  // walk's on screen, so another walk coming on screen offers Approve of its own.
  const [approval, setApproval] = useState<{ walk: string | undefined; at: number; state: CaptureBarApproval } | undefined>(undefined);
  const [reply, setReply] = useState("");
  const [replying, setReplying] = useState(false);
  const [replyRefused, setReplyRefused] = useState<string | undefined>(undefined);
  const bar = useRef<HTMLDivElement>(null);
  const pointer = useRef<Pointer | null>(null);

  useEffect(() => api.read(setState), [api]);
  // A key main heard while the page had the keyboard, pressed on the bar's own root where its handler is.
  useEffect(
    () =>
      api.onKey?.((key) => {
        bar.current?.firstElementChild?.dispatchEvent(new KeyboardEvent("keydown", { ...QUEUE_PRESS[key], bubbles: true, cancelable: true }));
      }),
    [api],
  );

  const armed = state?.armed === true;
  const heldNow = held !== null;
  const ask = state?.ask;
  const askId = ask?.id;
  // Over the page while it is shown and capture is not armed: capture takes the page while it is.
  const asking = ask !== undefined && ask.shown && !armed;
  // The ring of a walk Bridge plays, over the page until it is put away, held on a step or done. A question
  // or an armed capture is drawn instead: each of them takes the page.
  const guide = state?.walk?.guide;
  const guiding = guide !== undefined && !guide.folded && state?.walk?.done !== true && guide.stopped === undefined && guide.signingIn !== true && !armed && !asking;

  // A new question starts with nothing typed against it.
  useEffect(() => {
    setReply("");
    setReplyRefused(undefined);
  }, [askId]);


  /** The page's viewport, which starts where the bar ends. Measured, never assumed. */
  const strip = useCallback((): number => bar.current?.offsetHeight ?? 0, []);
  // The page still scrolls under the outline while the question or the ring is drawn over it, as under capture's.
  useEffect(() => {
    if (!asking && !guiding) return undefined;
    function wheeled(event: WheelEvent): void {
      if (event.clientY < strip()) return;
      api.scroll({ x: event.clientX, y: event.clientY - strip(), deltaX: -event.deltaX, deltaY: -event.deltaY });
    }
    window.addEventListener("wheel", wheeled, { capture: true, passive: true });
    return () => window.removeEventListener("wheel", wheeled, true);
  }, [api, asking, guiding, strip]);

  // The walk's keys answer while this bar has the keyboard, even when nothing in it holds focus: a Next
  // that goes disabled while the step seeks drops focus to the body, and the keys would go quiet. So does
  // the queue's ⌘↵, which main leaves to this document. The press is handed to the bar's own root, where its key handler is.
  const walking = state?.walk !== undefined && !armed;
  const queued = state?.queue !== undefined;
  useEffect(() => {
    if (!walking && !queued) return undefined;
    function keyed(event: KeyboardEvent): void {
      const root = bar.current?.firstElementChild;
      if (root === null || root === undefined || event.target !== document.body || !event.isTrusted) return;
      const plain = !event.metaKey && !event.ctrlKey && !event.altKey && /^(arrowright|arrowleft|enter|a|r)$/i.test(event.key);
      const approving = queued && event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && event.key === "Enter";
      if (!(walking && plain) && !approving) return;
      event.preventDefault();
      event.stopPropagation();
      const { key, code, metaKey, ctrlKey, altKey, shiftKey } = event;
      root.dispatchEvent(new KeyboardEvent("keydown", { key, code, metaKey, ctrlKey, altKey, shiftKey, bubbles: true, cancelable: true }));
    }
    window.addEventListener("keydown", keyed, true);
    return () => window.removeEventListener("keydown", keyed, true);
  }, [walking, queued]);

  const close = useCallback(() => {
    setHeld(null);
    setNote("");
    setRefused(undefined);
    void api.release();
  }, [api]);

  useEffect(() => {
    if (armed) return;
    setAimed(null);
    setHeld(null);
    setNote("");
    setRefused(undefined);
  }, [armed]);

  // The pointer is asked about once a frame while nothing is held, so the
  // outline follows without one round trip per pointer event.
  useEffect(() => {
    if (!armed || heldNow) return undefined;
    let frame = 0;
    let out = false;
    let last = "";
    const tick = (): void => {
      frame = requestAnimationFrame(tick);
      const at = pointer.current;
      if (out || at === null) return;
      const key = `${at.x},${at.y}`;
      if (key === last) return;
      last = key;
      out = true;
      void api.aim(at.x, at.y - strip()).then((answer) => {
        out = false;
        setAimed(answer);
      });
    };
    tick();
    return () => cancelAnimationFrame(frame);
  }, [api, armed, heldNow, strip]);

  useEffect(() => {
    if (!armed) return undefined;
    function moved(event: PointerEvent): void {
      pointer.current = { x: event.clientX, y: event.clientY };
    }
    function pressed(event: MouseEvent): void {
      // The card is Bridge's own surface and takes its own presses.
      if (event.target instanceof Element && event.target.closest("[data-armada-capture] *") !== null) return;
      if (event.clientY < strip()) return;
      event.preventDefault();
      event.stopPropagation();
      if (held !== null || note.trim() !== "") return;
      void api.hold(event.clientX, event.clientY - strip()).then((answer) => {
        if (answer !== null) setHeld(answer);
      });
    }
    // The page still scrolls under the outline: the wheel is taken here and
    // driven into the page by main, which is main moving the page rather than
    // the page reaching anything.
    function wheeled(event: WheelEvent): void {
      if (event.clientY < strip()) return;
      api.scroll({
        x: event.clientX,
        y: event.clientY - strip(),
        deltaX: -event.deltaX,
        deltaY: -event.deltaY,
      });
    }
    window.addEventListener("pointermove", moved, true);
    window.addEventListener("click", pressed, true);
    window.addEventListener("wheel", wheeled, { capture: true, passive: true });
    return () => {
      window.removeEventListener("pointermove", moved, true);
      window.removeEventListener("click", pressed, true);
      window.removeEventListener("wheel", wheeled, true);
    };
  }, [api, armed, held, note, strip]);

  async function save(): Promise<void> {
    setSaving(true);
    const outcome = await api.save(note.trim());
    setSaving(false);
    if (outcome.ok) {
      setNote("");
      setHeld(null);
      return;
    }
    setRefused(`Not captured: ${said(outcome as Parameters<typeof said>[0])}`);
  }

  async function answer(given: CaptureAnswer): Promise<void> {
    setReplying(true);
    const outcome = await api.answerAsk(given);
    setReplying(false);
    if (outcome.ok) {
      setReply("");
      setReplyRefused(undefined);
      return;
    }
    setReplyRefused(`Not sent: ${said(outcome as Parameters<typeof said>[0])}`);
  }

  /** Words typed beside an option go with it. */
  const words = (): { text?: string } => (reply.trim() === "" ? {} : { text: reply.trim() });

  /** The walk on screen, where the bar is a walk window's. */
  const walkOnScreen = state?.queue?.entries[state.queue.at]?.id;

  async function approve(): Promise<void> {
    const pressed = state?.variations?.at ?? 0;
    const walk = walkOnScreen;
    setApproval({ walk, at: pressed, state: { state: "pending" } });
    const outcome = await api.approve();
    setApproval({
      walk,
      at: pressed,
      state: outcome.ok ? { state: "approved" } : { state: "refused", said: said(outcome as Parameters<typeof said>[0]) },
    });
  }

  if (state === null) return null;

  // The page starts below the bar, so a box measured in its viewport is drawn
  // that much lower in this document.
  const lowered = (box: CaptureBox): CaptureBox => ({ ...box, y: box.y + strip() });
  const studio = state.studio === null ? null : (state.studio.name ?? UNTITLED_STUDIO);
  // A pull request's walk with no Session holding it keeps its notes in Armada, which the bar words as a Studio's name.
  const keptIn = state.review !== undefined && state.review.holder === undefined ? "Armada" : null;
  const aim = state.serving ? `Onto ${studio ?? state.session?.title ?? state.review?.holder ?? keptIn ?? ""}` : "The run ended — nothing more is captured";
  const panel = state.panel;

  return (
    <>
      <div ref={bar}>
        <CaptureBar
          {...(state.shown.name === null ? {} : { run: state.shown.name })}
          address={state.shown.address}
          studio={studio ?? keptIn}
          {...(state.session !== undefined ? { session: state.session.title } : state.review?.holder === undefined ? {} : { session: state.review.holder })}
          {...(state.session === undefined ? {} : { onOpenOwner: () => void api.openOwner() })}
          serving={state.serving}
          armed={armed}
          framesRefused={state.framesRefused}
          {...(state.refused === undefined
            ? {}
            : {
                refused: {
                  address: state.refused.address,
                  said: REFUSED_SAID[state.refused.what] ?? "Navigation",
                  offerable: state.refused.offerable,
                },
              })}
          onArm={(on) => void api.arm(on)}
          onReload={() => void api.reload()}
          onFollowRefused={() => void api.followRefused()}
          binding={BINDING}
          // A Studio's window has nobody to tell; a Session's has its owner, and a pull request's walk in the walk window ends.
          {...(state.studio === null && (state.session !== undefined || (state.review !== undefined && state.queue !== undefined)) ? { onApprove: () => void approve() } : {})}
          {...(approval === undefined || approval.walk !== walkOnScreen || approval.at !== (state.variations?.at ?? 0) ? {} : { approval: approval.state })}
          {...(state.readiness === undefined ? {} : { readiness: state.readiness })}
          {...(state.walk === undefined
            ? {}
            : {
                walk: {
                  ...state.walk,
                  onBack: () => void api.walkBack(),
                  onNext: () => void api.walkNext(),
                  onAutoplay: (on: boolean) => void api.walkAutoplay(on),
                  onReplay: () => void api.walkReplay(),
                  guide:
                    state.walk.guide === undefined
                      ? undefined
                      : {
                          shown: !state.walk.guide.folded,
                          onShow: (shown: boolean) => void api.walkGuide(shown),
                          ...(state.walk.guide.stopped === undefined ? {} : { stopped: state.walk.guide.stopped }),
                          ...(state.walk.guide.signingIn === true ? { signingIn: true } : {}),
                          ...(state.walk.guide.waiting === true ? { waiting: true } : {}),
                        },
                },
              })}
          {...(state.variations === undefined ? {} : { variations: { names: state.variations.names, at: state.variations.at, onPick: (at: number) => void api.pick(at) } })}
          {...(ask === undefined || ask.shown ? {} : { question: ask.question, onShowQuestion: () => void api.showAsk(true) })}
          {...(panel === undefined ? {} : { panel: { kind: panel.kind, open: panel.open, onToggle: (open: boolean) => void api.panel(open) } })}
          {...(state.queue === undefined || state.queue.entries.length === 0 ? {} : { queue: queueOf(state.queue, api) })}
        />
      </div>
      {ask === undefined || !asking ? null : (
        <WindowQuestion
          from={ask.from}
          question={ask.question}
          options={ask.options}
          {...(ask.box === undefined ? {} : { box: ask.box === null ? null : lowered(ask.box) })}
          answer={reply}
          onAnswer={setReply}
          onChoose={(choice) => void answer({ choice, ...words() })}
          onSend={() => void answer(words())}
          onFold={() => void api.showAsk(false)}
          sending={replying}
          {...(replyRefused === undefined ? {} : { refused: replyRefused })}
        />
      )}
      {!guiding || state.walk === undefined || guide === undefined ? null : (
        <WalkGuide
          name={state.walk.name}
          at={state.walk.at}
          of={state.walk.of}
          said={state.walk.said}
          box={guide.box === null ? null : lowered(guide.box)}
          top={strip()}
          onFold={() => void api.walkGuide(false)}
        />
      )}
      {armed ? (
        <CaptureOverlay
          aim={aim}
          aimed={state.serving}
          binding={BINDING}
          note={note}
          onNote={setNote}
          onSave={() => void save()}
          onCancel={close}
          saving={saving}
          {...(refused === undefined ? {} : { refused })}
          {...(aimed !== null && held === null ? { hovered: { box: lowered(aimed.box), named: aimed.named } } : {})}
          {...(held === null ? {} : { held: { box: lowered(held.box), chain: held.chain } })}
        />
      ) : null}
    </>
  );
}

/** The walks as the bar draws them, each press a place in the queue that main reads the walk off. ⌥⌘→ past the last is the first. */
function queueOf(queue: NonNullable<CaptureWindowState["queue"]>, api: CaptureWindowApi): CaptureBarQueue {
  const count = queue.entries.length;
  return {
    entries: queue.entries,
    at: queue.at,
    onPick: (at) => void api.walkTo?.(at),
    onStep: (by) => void api.walkTo?.((queue.at + by + count) % count),
  };
}
