// The call card's acts, moved out to the Helm rail beside it as keycap switches: the decision on top, a
// rule, where to go next, a rule, and Dismiss under a cover. What waits behind the card is a strip on its
// other side. Under about 1100px the rail folds into the split row under the card.

import { useEffect, useRef, useState } from "react";
import { Lock, LockOpen, Radar } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import type { Item } from "../Dashboard";

/** How long a lifted cover stays lifted. */
const COVER_MS = 3000;

/**
 * One act as a keycap switch: the key printed on the cap, the verb under it, heavy at the bottom edge
 * so a press reads as going down. `primary` is the accent one. Pending is the press waiting on Fleet.
 */
export function Keycap({
  label,
  keys,
  primary = false,
  pending = false,
  disabled = false,
  says,
  onPress,
}: {
  label: string;
  keys: string;
  primary?: boolean;
  pending?: boolean;
  disabled?: boolean;
  /** What a hover says, where the cap needs more than its verb. */
  says?: string;
  onPress: () => void;
}) {
  const cap = (
    <span className="armada-keycap" data-primary={primary || undefined}>
      <Button variant={primary ? "primary" : "secondary"} pending={pending} disabled={disabled} onClick={onPress}>
        <Kbd>{keys}</Kbd>
        <span className="armada-keycap__verb">{label}</span>
      </Button>
    </span>
  );
  return says === undefined ? cap : <Tooltip label={says} asChild>{cap}</Tooltip>;
}

/**
 * The act that cannot be struck by accident, under a cover. The first press lifts the cover, which says
 * so in the frame and the glyph; a second press within a few seconds does it. The key does the same,
 * through `press`.
 */
export function CoveredKeycap({
  label,
  keys,
  lifted,
  pending,
  disabled,
  says,
  onPress,
}: {
  label: string;
  keys: string;
  lifted: boolean;
  pending: boolean;
  disabled: boolean;
  says: string;
  onPress: () => void;
}) {
  const Glyph = lifted ? LockOpen : Lock;
  return (
    <Tooltip label={says} asChild>
      <span className="armada-keycap" data-cover={lifted ? "lifted" : "down"}>
        <Button variant="secondary" pending={pending} disabled={disabled} onClick={onPress}>
          <Kbd>{keys}</Kbd>
          <span className="armada-keycap__verb">{label}</span>
          <Glyph size={14} aria-hidden="true" className="armada-keycap__glyph" />
        </Button>
      </span>
    </Tooltip>
  );
}

/** A cover that lifts on one press and lets go on its own: `lifted`, and the press that says whether to do the act. */
export function useCover(act: () => void): { lifted: boolean; press: () => void } {
  const [lifted, setLifted] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const press = () => {
    if (lifted) {
      clearTimeout(timer.current);
      setLifted(false);
      act();
      return;
    }
    setLifted(true);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setLifted(false), COVER_MS);
  };
  return { lifted, press };
}

/**
 * The calls behind the one in front, as a strip along the console's edge: one slim row each, its kind's
 * glyph and its title, the one put off dashed. Pressing one brings it forward.
 */
export function Queue({ edge, put, recall, first }: { edge: readonly Item[]; put: readonly string[]; recall: (key: string) => void; first: string }) {
  if (edge.length === 0) return null;
  return (
    <section className="armada-queue" aria-label="Waiting">
      <header className="armada-queue__band">
        <Tooltip label="Waiting">
          <span role="img" aria-label="Waiting">
            <Radar size={14} aria-hidden="true" />
          </span>
        </Tooltip>
      </header>
      <ol className="armada-queue__list">
        {edge.slice(0, 6).map((one, at) => {
          const Icon = one.icon;
          return (
            <li key={one.key}>
              <Tooltip label={one.title} asChild>
                <button
                  type="button"
                  className="armada-queue__strip"
                  data-hue={one.hue}
                  data-deferred={put.includes(one.key) || undefined}
                  aria-label={`${one.kind}: ${one.title}`}
                  onClick={() => recall(one.key)}
                >
                  <Icon size={12} aria-hidden="true" />
                  <span>{one.title}</span>
                  {at === 0 ? <Kbd>{first}</Kbd> : null}
                </button>
              </Tooltip>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
