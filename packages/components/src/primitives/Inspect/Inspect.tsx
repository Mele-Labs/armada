import {
  cloneElement,
  isValidElement,
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent as ReactFocusEvent,
  type HTMLAttributes,
  type MouseEvent as ReactMouseEvent,
  type ReactElement,
  type ReactNode,
  type Ref,
} from "react";
import { createPortal } from "react-dom";
import type { LucideIcon } from "lucide-react";

import { actionOf, keyFor } from "../../actions";
import { KbdBinding } from "../Kbd/Kbd";

/**
 * The inspect card: **the one card a rich hover opens, the same over a Session as over a pull request.** A
 * reader who has seen one has seen them all, and only what is said in it changes with the kind of thing.
 *
 * Two halves. `InspectCard` is the surface and its anatomy, top to bottom:
 *
 * 1. a head: the kind's glyph, the kind in small caps, and on the right the item's id or where it is, in mono;
 * 2. the name, as the title;
 * 3. a state band: the state's glyph and words, **in the item's own status tone, the one place colour carries
 *    meaning** (the card's top edge takes the same tone, as a thin line);
 * 4. the facts, label muted over value, mono where the system made the value;
 * 5. the caller's own sections, where a kind has more to say;
 * 6. the keys that work on this item now, each a key chip and a verb, ending in Esc.
 *
 * `Inspect` is the behaviour, for a card that hangs off an element. **One rule set for every card:**
 * it opens on hover after `--tooltip-delay`, and on keyboard focus; the pointer can cross into it; a press pins
 * it, and a pinned card takes focus and can be pressed; Esc or a press outside closes it; and moving to another
 * item while one is open switches at once, as `--tooltip-grace` does for a tooltip. It is placed by anchor
 * positioning with a flip, in the top layer, so it clears every sheet and is never clipped by what it hangs off.
 *
 * The plain `Tooltip` stays for one line of words. A card is for something with a head, a state and facts.
 */

/**
 * One fact: its label, its value, and whether the system made the value. A `tone` is a CSS colour, a status token. `open` makes the
 * value a press that opens what it names, drawn as a link; it is named "Open <label> <value>" where the value is words.
 */
export type InspectFact = { label: string; value: ReactNode; mono?: boolean; tone?: string; open?: () => void };

/**
 * A key that works on the item now. `act` reads the key from the shortcut registry, so Settings → Keyboard moves
 * it here too, and its verb is the registry's unless one is given. `key` is for a press the browser itself
 * answers, such as Enter on a focused button, which is not an act of the registry's.
 */
export type InspectKey = { act: string; verb?: string } | { key: string; verb: string };

export type InspectCardProps = Omit<HTMLAttributes<HTMLDivElement>, "title"> & {
  /** The kind of thing: its glyph from the icon registry and its name. */
  kind: { Glyph: LucideIcon; label: string };
  /** The item's id, or where it is. Mono, on the head's right. */
  where?: ReactNode;
  /** The item's name. */
  title?: ReactNode;
  /** Its state in the registry's glyph and words, in its status tone. */
  state?: { Glyph?: LucideIcon; said: ReactNode; tone?: string };
  /** The colour of the thin top edge: a status token. Defaults to the state's, else none. */
  tone?: string;
  facts?: readonly InspectFact[];
  /** The keys that work on this item now. Esc, which closes every card, is added after them. */
  keys?: readonly InspectKey[];
  /** Narrow or wide, and nothing between. */
  width?: "narrow" | "wide";
  ref?: Ref<HTMLDivElement>;
  /** The caller's own sections, between the facts and the keys. */
  children?: ReactNode;
};

/** Facts as a two-column grid: label muted, value default, mono where the system made it. None draws nothing. */
export function InspectFacts({ facts }: { facts: readonly InspectFact[] }) {
  if (facts.length === 0) return null;
  return (
    <dl className="armada-inspect-card__facts">
      {facts.map((fact, at) => (
        <div key={`${at}:${fact.label}`}>
          <dt>{fact.label}</dt>
          <dd data-mono={fact.mono || undefined} style={fact.tone === undefined ? undefined : { color: fact.tone }}>
            {fact.open === undefined ? (
              fact.value
            ) : (
              <button type="button" className="armada-inspect-card__open" aria-label={typeof fact.value === "string" ? `Open ${fact.label} ${fact.value}` : undefined} onClick={fact.open}>
                {fact.value}
              </button>
            )}
          </dd>
        </div>
      ))}
    </dl>
  );
}

/** A muted heading over a list of facts, for a section of a card. An empty list draws nothing. */
export function InspectSection({ heading, facts }: { heading: string; facts: readonly InspectFact[] }) {
  if (facts.length === 0) return null;
  return (
    <>
      <span className="armada-inspect-card__heading">{heading}</span>
      <InspectFacts facts={facts} />
    </>
  );
}

function KeyChip({ one }: { one: InspectKey }) {
  const binding = "act" in one ? keyFor(one.act) : one.key;
  if (binding === "") return null;
  const verb = one.verb ?? ("act" in one ? actionOf(one.act).verb : "");
  return (
    <li className="armada-inspect-card__key">
      <KbdBinding binding={binding} />
      {verb}
    </li>
  );
}

export function InspectCard({ kind, where, title, state, tone, facts = [], keys = [], width = "narrow", className, style, children, ref, ...rest }: InspectCardProps) {
  const edge = tone ?? state?.tone;
  return (
    <div
      ref={ref}
      className={["armada-inspect-card", className].filter(Boolean).join(" ")}
      data-width={width}
      style={{ ...(edge === undefined ? {} : { "--inspect-tone": edge }), ...style } as CSSProperties}
      {...rest}
    >
      <div className="armada-inspect-card__head">
        <kind.Glyph size={12} strokeWidth={2} aria-hidden />
        <span className="armada-inspect-card__kind">{kind.label}</span>
        {where === undefined ? null : <span className="armada-inspect-card__where">{where}</span>}
      </div>
      {title === undefined ? null : <div className="armada-inspect-card__title">{title}</div>}
      {state === undefined ? null : (
        <div className="armada-inspect-card__state" style={state.tone === undefined ? undefined : ({ "--inspect-state": state.tone } as CSSProperties)}>
          {state.Glyph === undefined ? null : <state.Glyph size={16} strokeWidth={2} aria-hidden />}
          {state.said}
        </div>
      )}
      <InspectFacts facts={facts} />
      {children}
      <ul className="armada-inspect-card__keys" aria-label="Keys">
        {keys.map((one, at) => (
          <KeyChip key={at} one={one} />
        ))}
        <KeyChip one={{ act: "close" }} />
      </ul>
    </div>
  );
}

/** How long a card waits after the pointer leaves, so it can cross the gap to the card. */
const LEAVE_MS = 150;

/** When a card anywhere last closed, and the one that is open. Module-level: the group is whatever the pointer is crossing. */
let closedAt = 0;
let active: (() => void) | undefined;

function readMs(name: string, fallback: number): number {
  const parsed = Number.parseInt(getComputedStyle(document.documentElement).getPropertyValue(name), 10);
  return Number.isNaN(parsed) ? fallback : parsed;
}

const ANCHORS = typeof CSS !== "undefined" && typeof CSS.supports === "function" && CSS.supports("anchor-name", "--a");

const FOCUSABLE = "button:not([disabled]),a[href],input,select,textarea,[tabindex]:not([tabindex='-1'])";

export type InspectProps = {
  /** The card. A function is given the way to put it away and whether it is pinned, for an act inside it that takes the reader elsewhere. */
  card: ReactNode | ((card: { close: () => void; pinned: boolean }) => ReactNode);
  /** What the card is called, for a reader who cannot see it, once it is pinned. */
  name: string;
  /** The one element it hangs off. It must take `className`, handlers and a ref, as every element and every primitive here does. */
  children: ReactElement;
  /** Below the trigger by default; beside it, on its inline-start side, for a row in a column whose card below would cover the rows under it. Flipped where there is no room. */
  side?: "below" | "beside" | "above";
  /**
   * Whether a press pins the card. **Off where the press is already an act** — a row that opens, a chip that
   * selects — because pinning would take the press from the thing the reader pressed.
   */
  pin?: boolean;
  /** A call asked for the card to open: it is pinned and focus goes to the part `to` names; `onSummoned` lets go of the ask. */
  summoned?: boolean;
  to?: string;
  onSummoned?: () => void;
};

export function Inspect({ card, name, children, side = "below", pin = true, summoned = false, to, onSummoned }: InspectProps) {
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [pinned, setPinned] = useState(false);
  const open = hover || focus || pinned;
  const id = useId();
  const anchor = `--inspect-${id.replace(/\W/g, "")}`;
  const trigger = useRef<HTMLElement | null>(null);
  const shell = useRef<HTMLDivElement | null>(null);
  const waiting = useRef<number | undefined>(undefined);
  const leaving = useRef<number | undefined>(undefined);
  /** Esc closed it and the pointer or focus has not left since, so it stays closed. */
  const dismissed = useRef(false);
  const takeFocus = useRef(false);

  const away = useCallback(() => {
    window.clearTimeout(waiting.current);
    window.clearTimeout(leaving.current);
    setHover(false);
    setFocus(false);
    setPinned(false);
  }, []);
  useEffect(() => () => {
    window.clearTimeout(waiting.current);
    window.clearTimeout(leaving.current);
  }, []);

  // One open at a time, and the next opens at once while the last has only just closed.
  useEffect(() => {
    if (!open) return;
    active?.();
    active = away;
    return () => {
      if (active === away) active = undefined;
      closedAt = Date.now();
    };
  }, [open, away]);

  const show = (via: "hover" | "focus") => {
    window.clearTimeout(leaving.current);
    if (dismissed.current) return;
    const set = via === "hover" ? setHover : setFocus;
    if (open || active !== undefined || Date.now() - closedAt < readMs("--tooltip-grace", 300)) return set(true);
    window.clearTimeout(waiting.current);
    waiting.current = window.setTimeout(() => set(true), readMs("--tooltip-delay", 400));
  };
  const hide = (via: "hover" | "focus") => {
    window.clearTimeout(waiting.current);
    window.clearTimeout(leaving.current);
    if (via === "hover") leaving.current = window.setTimeout(() => setHover(false), LEAVE_MS);
    else setFocus(false);
  };
  const inside = (node: EventTarget | null) => node instanceof Node && (trigger.current?.contains(node) === true || shell.current?.contains(node) === true);

  // Top layer, so the card clears every sheet; and, where anchor positioning is missing, placed by hand.
  useLayoutEffect(() => {
    const el = shell.current;
    if (!open || el === null) return;
    try {
      if (!el.matches(":popover-open")) el.showPopover();
    } catch {
      // Already showing, or no popover in this renderer: the stylesheet's fixed layer stands.
    }
    const from = trigger.current?.getBoundingClientRect();
    if (ANCHORS || from === undefined) return;
    const gap = 8;
    const own = el.getBoundingClientRect();
    const right = side === "beside" ? from.left - gap - own.width : from.left;
    const left = Math.max(gap, Math.min(right < gap ? from.right + gap : right, window.innerWidth - own.width - gap));
    const below = side === "beside" ? from.top : from.bottom + gap;
    const top = side === "above" || below + own.height > window.innerHeight ? Math.max(gap, from.top - gap - own.height) : below;
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
  });

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      const within = shell.current?.contains(document.activeElement) === true;
      // A card the reader holds takes the Esc whole; one only hovered lets it go on to whatever else answers it.
      if (pinned || within || focus) {
        event.preventDefault();
        event.stopPropagation();
      }
      if (within) trigger.current?.focus({ preventScroll: true });
      dismissed.current = true;
      away();
    };
    const onDown = (event: PointerEvent) => {
      if (!inside(event.target)) away();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("pointerdown", onDown, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("pointerdown", onDown, true);
    };
  });

  const pinNow = () => {
    takeFocus.current = true;
    setPinned(true);
  };
  useLayoutEffect(() => {
    if (!pinned || !takeFocus.current) return;
    takeFocus.current = false;
    const el = shell.current;
    if (el === null) return;
    const aim = to === undefined ? null : el.querySelector<HTMLElement>(to);
    (aim ?? el.querySelector<HTMLElement>(FOCUSABLE) ?? el).focus({ preventScroll: true });
  }, [pinned, to]);
  useEffect(() => {
    if (!summoned) return;
    pinNow();
    onSummoned?.();
  }, [summoned]);

  if (!isValidElement(children)) throw new Error("Inspect takes exactly one element");
  const child = children as ReactElement<HTMLAttributes<HTMLElement> & { ref?: Ref<HTMLElement> }>;
  const own = child.props;
  const cardId = `${id}-card`;
  const described = open && !pinned ? [own["aria-describedby"], cardId].filter(Boolean).join(" ") : own["aria-describedby"];

  const trigged = cloneElement(child, {
    className: [own.className, "armada-inspect-on"].filter(Boolean).join(" "),
    style: { ...own.style, anchorName: anchor } as CSSProperties,
    "aria-describedby": described,
    onMouseEnter: (event: ReactMouseEvent<HTMLElement>) => {
      own.onMouseEnter?.(event);
      show("hover");
    },
    onMouseLeave: (event: ReactMouseEvent<HTMLElement>) => {
      own.onMouseLeave?.(event);
      dismissed.current = false;
      hide("hover");
    },
    onFocus: (event: ReactFocusEvent<HTMLElement>) => {
      own.onFocus?.(event);
      // Only a keyboard's arrival: a press focuses a button too, and it is the press that decides then.
      if (event.target.matches(":focus-visible")) show("focus");
    },
    onBlur: (event: ReactFocusEvent<HTMLElement>) => {
      own.onBlur?.(event);
      dismissed.current = false;
      if (inside(event.relatedTarget)) return;
      hide("focus");
      if (event.relatedTarget !== null) setPinned(false);
    },
    onClick: (event: ReactMouseEvent<HTMLElement>) => {
      own.onClick?.(event);
      // The press is the act, so the card gets out of the way of what it opened, and stays shut until the pointer leaves.
      if (!pin) {
        dismissed.current = true;
        away();
        return;
      }
      if (event.defaultPrevented) return;
      if (pinned) {
        dismissed.current = true;
        away();
      } else {
        pinNow();
      }
    },
    ref: (node: HTMLElement | null) => {
      trigger.current = node;
      const held = own.ref;
      if (typeof held === "function") held(node);
      else if (held !== null && held !== undefined) (held as { current: HTMLElement | null }).current = node;
    },
  } as HTMLAttributes<HTMLElement>);

  return (
    <>
      {trigged}
      {open
        ? createPortal(
            <div
              ref={shell}
              id={cardId}
              className="armada-inspect"
              popover="manual"
              tabIndex={-1}
              data-side={side}
              data-pinned={pinned || undefined}
              role={pinned ? "dialog" : "tooltip"}
              aria-label={pinned ? name : undefined}
              style={{ positionAnchor: anchor } as CSSProperties}
              // Drawn in the body but a child of its trigger in the tree, so a press or a key in it would reach a row that opens on one.
              onClick={(event) => event.stopPropagation()}
              onKeyDown={(event) => event.stopPropagation()}
              onMouseEnter={() => show("hover")}
              onMouseLeave={() => hide("hover")}
              onFocus={() => setFocus(true)}
              onBlur={(event) => {
                if (inside(event.relatedTarget)) return;
                setFocus(false);
                if (event.relatedTarget !== null) setPinned(false);
              }}
            >
              {typeof card === "function" ? card({ close: away, pinned }) : card}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
