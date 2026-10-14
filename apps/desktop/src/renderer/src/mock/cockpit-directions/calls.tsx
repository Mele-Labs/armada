// The calls as the three prototypes keep them: a tray strip of their own along the foot of the glass
// where a call arrives as a transmission, and the centred card it opens into on `w`. The card is the
// real CallCard, answered through the same `useAnswering` the Cockpit's is, so a reply, a pick or a
// done goes where it goes there. The tray is the only new part. Mock only. proto/cockpit-three-directions.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Kbd, Tooltip, isPressed, keyFor, pressedDigit } from "@armada/components";
import { nowPanelOf } from "@armada/jobs";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { Item } from "../../Dashboard";
import { holdFor } from "../../cockpit/Cockpit";
import { CallCard, type CardKeys } from "../../cockpit/CallCard";
import { Queue } from "../../cockpit/consoles";
import type { CockpitProps } from "../../cockpit/body-seam";
import { KEYS } from "./shared";
import type { Ship } from "./ship";

export type Calls = {
  /** Calls not yet answered, the one that arrived last in front. */
  line: readonly Item[];
  front: Item | undefined;
  /** The card is open, and has the keys. */
  open: boolean;
  openFront: () => void;
  /** The tray strip along the foot of the glass. */
  tray: ReactNode;
  /** The centred card over the glass, while it is open. */
  card: ReactNode;
};

/**
 * Calls answered are held out until Fleet stops carrying them, as the Cockpit does. **A call is in the
 * tray from the moment it is first seen**, and one that arrives later stands in front of the ones
 * already there.
 */
export function useCalls(props: CockpitProps, ship: Ship): Calls {
  const [holds, setHolds] = useState<ReadonlyMap<string, number>>(new Map());
  const [open, setOpen] = useState(false);
  const seen = useRef(new Map<string, number>());
  const first = useRef(true);
  const carried = ship.calls;
  const fresh = carried.filter((one) => !seen.current.has(one.key));
  // The ones already there at the first frame share one instant; later ones are later.
  for (const one of fresh) seen.current.set(one.key, first.current ? 0 : Date.now());
  useEffect(() => {
    first.current = false;
  }, []);
  const line = useMemo(
    () =>
      carried
        .filter((one) => !holds.has(one.key))
        .map((one, at) => ({ one, at, when: seen.current.get(one.key) ?? 0 }))
        .sort((a, b) => b.when - a.when || a.at - b.at)
        .map(({ one }) => one),
    [carried, holds],
  );
  useEffect(() => {
    if (holds.size === 0) return;
    const live = new Set(carried.map((one) => one.key));
    const gone = [...holds].filter(([key, since]) => Date.now() - since >= holdFor.ms || !live.has(key));
    if (gone.length > 0) return setHolds((was) => new Map([...was].filter(([key]) => !gone.some(([one]) => one === key))));
    const next = Math.min(...holds.values()) + holdFor.ms - Date.now();
    const timer = window.setTimeout(() => setHolds((was) => new Map(was)), Math.max(0, next));
    return () => window.clearTimeout(timer);
  }, [holds, carried]);

  const front = line[0];
  // The card closes when there is nothing to show.
  useEffect(() => {
    if (open && front === undefined) setOpen(false);
  }, [open, front]);
  const answering = useRef<CardKeys | undefined>(undefined);
  const finish = () => {
    if (front === undefined) return;
    setHolds((was) => new Map(was).set(front.key, Date.now()));
    setOpen(false);
  };
  const later = () => {
    if (front === undefined) return;
    seen.current.set(front.key, -1);
    setOpen(false);
  };

  useListKeydown((event) => {
    if (!open || front === undefined) return;
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    if (event.repeat) return;
    if (holdsText(event.target)) {
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (isPressed("close", event) || (bare && event.key === "ArrowDown"))) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    const card = answering.current;
    if (card === undefined) return;
    const claim = () => event.preventDefault();
    const digit = pressedDigit("call_pick", event);
    if (digit !== null) return claim(), card.pickNumber(digit);
    if (card.pickKey(event)) return claim();
    if (isPressed("call_reply", event) && card.change !== undefined) return claim(), card.change();
    if (isPressed("call_reply", event)) {
      const field = document.querySelector<HTMLTextAreaElement>(".armada-callcard__reply textarea");
      return field === null ? undefined : (claim(), field.focus());
    }
    if (isPressed("call_expand", event)) return card.expand === undefined ? undefined : (claim(), card.expand());
    if (isPressed("call_send", event)) {
      if ((event.target as HTMLElement).tagName === "BUTTON" && document.querySelector(".armada-callcard")?.contains(event.target as Node) === true) return;
      return card.canSend ? (claim(), card.send()) : undefined;
    }
    if (isPressed("call_later", event) || isPressed("close", event)) return claim(), card.pending === undefined ? later() : undefined;
    if (isPressed("call_dismiss", event)) return claim(), card.dismiss();
    if (isPressed("call_open_walk", event)) return card.openWalk === undefined ? undefined : (claim(), card.openWalk());
    if (isPressed("open", event)) return card.open === undefined ? undefined : (claim(), card.open());
  });

  const hosts = props;
  const nowing = front === undefined ? undefined : front.job === undefined ? undefined : nowPanelOf(props.nows?.[front.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} });
  const behind = line.slice(1);

  const tray =
    front === undefined ? null : (
      <div className="proto-tray" data-open={open || undefined}>
        {behind.slice(0, 2).map((one, at) => (
          <span key={one.key} className="proto-tray__behind" data-hue={one.hue} style={{ ["--i" as string]: at + 1 }} aria-hidden="true" />
        ))}
        <TrayStrip key={front.key} item={front} arrived={!first.current || seen.current.get(front.key) !== 0} onOpen={() => setOpen(true)} />
      </div>
    );

  const card =
    !open || front === undefined ? null : (
      <div className="armada-cockpit__scrim proto-scrim" data-hue={front.hue}>
        <div className="armada-stack" data-consoled style={{ ["--behind" as string]: 0 }}>
          <CallCard
            key={front.key}
            queue={<Queue edge={behind} put={[]} recall={(key) => (seen.current.set(key, Date.now()), setHolds((was) => new Map(was)))} first={keyFor("call_recall")} />}
            item={front}
            now={props.now}
            nowing={nowing}
            state={props.state}
            hosts={hosts}
            finish={finish}
            later={later}
            dismiss={finish}
            leaving={undefined}
            from={undefined}
            answering={answering}
          />
        </div>
      </div>
    );

  return { line, front, open, openFront: () => (front === undefined ? undefined : setOpen(true)), tray, card };
}

/** One transmission: the call's kind glyph and its title, and the key that opens it. It arrives from the foot of the glass. */
function TrayStrip({ item, arrived, onOpen }: { item: Item; arrived: boolean; onOpen: () => void }) {
  const Icon = item.icon;
  return (
    <button type="button" className="proto-tray__strip" data-hue={item.hue} data-arrived={arrived || undefined} aria-label={`${item.kind}: ${item.title}`} onClick={onOpen}>
      <Tooltip label={item.kind}>
        <span className="proto-tray__kind" role="img" aria-label={item.kind}>
          <Icon size={16} aria-hidden />
        </span>
      </Tooltip>
      <span className="proto-tray__title">{item.title}</span>
      <Kbd>{KEYS.call}</Kbd>
    </button>
  );
}
