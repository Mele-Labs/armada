// Main's light on the merge queue, when a main is red: the same panel the Merge line draws for the red, on a card the light opens.
// Hovering or focusing the light opens it, a press pins it, and Esc or a press outside puts it away. The card is the Merge line's
// own band with the Merge line's own acts (`useFixChoice`), so who is fixing, who holds the claim and the ways to hand it to a Job
// or a Session are one thing in both places. #2301.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { MainRedBand, Tooltip } from "@armada/components";
import { recentSessionsOf, type MergeLineView } from "@armada/screens";

import { useFixChoice } from "../../merge-line";
import { useSessions } from "../../sessions-draft";
import type { CockpitProps } from "./props";
import { select, selOfSubject } from "./store";
import type { Ship } from "./ship";

/** How long the card waits after the pointer leaves before it closes, so the pointer can cross the gap between the light and it. */
const LEAVE_MS = 150;

type Red = { view: MergeLineView & { hub: NonNullable<MergeLineView["hub"]> }; name: string };

/** One red repository's band, with the Sessions the work can be sent to and the acts the Merge line sends. */
function RedBand({ red, props, ship, named, away }: { red: Red; props: CockpitProps; ship: Ship; named: boolean; away: () => void }) {
  const sessions = useSessions();
  const main = red.view.hub.main;
  const culprit = main?.state === "red" ? main.red.merge?.session : undefined;
  const fix = useFixChoice(red.view.root, props.onFix, props.onSendToSession);
  if (main?.state !== "red") return null;
  // A Job or Session the deck lists opens on its stage; any other opens on its own page.
  const open = (kind: "job" | "session", id: string, elsewhere: (id: string) => void) => {
    const held = ship.subjects.find((one) => one.kind === kind && one.id === id);
    if (held === undefined) elsewhere(id);
    else select(selOfSubject(held));
  };
  return (
    <section className="deck-main-card__repo" aria-label={red.name}>
      {named ? <h3 className="deck-main-card__name">{red.name}</h3> : null}
      <MainRedBand
        main={main}
        recent={red.view.hub.recent}
        recentSessions={recentSessionsOf(sessions, culprit)}
        onOpenLink={props.onOpenLink}
        onOpenJob={(id) => open("job", id, props.onOpen)}
        onOpenSession={(id) => open("session", id, props.onOpenSession)}
        {...(fix === undefined
          ? {}
          : {
              onFix: (choice) => {
                // A new Session is made in the Dispatch panel, which opens over the card.
                if (choice.kind === "newSession") away();
                return fix(choice);
              },
            })}
      />
    </section>
  );
}

export function MainLight({ props, ship, reds, said }: { props: CockpitProps; ship: Ship; reds: readonly Red[]; said: string }) {
  // The light's own ref and handlers are the Tooltip's to replace, so the light is found, and listened to, from the span around it.
  const wrap = useRef<HTMLSpanElement>(null);
  const light = () => wrap.current?.querySelector<HTMLButtonElement>("button") ?? null;
  const card = useRef<HTMLDivElement>(null);
  const timer = useRef<number | undefined>(undefined);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const open = hover || focus || pinned;
  const away = () => {
    window.clearTimeout(timer.current);
    setHover(false);
    setFocus(false);
    setPinned(false);
  };

  useEffect(() => () => window.clearTimeout(timer.current), []);

  // Where the card sits: above the light, which lives low on the deck, and below it where there is no room above.
  useLayoutEffect(() => {
    const layer = card.current;
    const from = light()?.getBoundingClientRect();
    if (!open || layer === null || from === undefined) return;
    const gap = 8;
    const own = layer.getBoundingClientRect();
    const left = Math.max(gap, Math.min(from.left, window.innerWidth - own.width - gap));
    const above = from.top - gap - own.height;
    layer.style.left = `${left}px`;
    layer.style.top = `${above >= gap ? above : from.bottom + gap}px`;
  });

  useEffect(() => {
    if (!open) return;
    // Esc puts the card away and stops there, so it does not also return the stage to what it was.
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      if (card.current?.contains(document.activeElement) === true) light()?.focus({ preventScroll: true });
      away();
    };
    const onDown = (event: MouseEvent) => {
      const hit = event.target as Node;
      if (wrap.current?.contains(hit) !== true && card.current?.contains(hit) !== true) away();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  const enter = () => {
    window.clearTimeout(timer.current);
    setHover(true);
  };
  const leave = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setHover(false), LEAVE_MS);
  };
  const blur = (event: { relatedTarget: EventTarget | null }) => {
    const to = event.relatedTarget as Node | null;
    if (to !== null && (wrap.current?.contains(to) === true || card.current?.contains(to) === true)) return;
    setFocus(false);
    // Focus that moved on to somewhere else puts the card away; a window losing focus names no target and leaves it.
    if (to !== null) setPinned(false);
  };

  return (
    <>
      <span ref={wrap} className="deck-main-light" onMouseEnter={enter} onMouseLeave={leave} onFocus={() => setFocus(true)} onBlur={blur}>
        <Tooltip label={said} asChild>
          <button
            type="button"
            className="deck-chip"
            data-main
            data-card={open || undefined}
            aria-label={said}
            aria-haspopup="dialog"
            aria-expanded={open}
            onClick={(event) => {
              if (pinned) return away();
              setPinned(true);
              // The card is drawn at the end of the page, so a key press carries focus into it: Tab from the light would not reach it.
              if (event.detail === 0) window.requestAnimationFrame(() => card.current?.querySelector<HTMLElement>("button, a[href]")?.focus());
            }}
          >
            <span className="deck-chip__dot" data-tone="bad" aria-hidden="true" />
            <span>main</span>
          </button>
        </Tooltip>
      </span>
      {!open
        ? null
        : createPortal(
            <div
              ref={card}
              className="deck-main-card"
              role="dialog"
              aria-label={said}
              onMouseEnter={enter}
              onMouseLeave={leave}
              onMouseDown={() => setPinned(true)}
              onFocus={() => setFocus(true)}
              onBlur={blur}
            >
              {reds.map((red) => (
                <RedBand key={red.view.root} red={red} props={props} ship={ship} named={reds.length > 1} away={away} />
              ))}
            </div>,
            document.body,
          )}
    </>
  );
}
