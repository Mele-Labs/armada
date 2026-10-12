// The failures standing as toasts, and the one corner every toast is drawn in.
//
// **A press that failed pops up over everything, and stays.** It used to draw
// as a banner above the surface, which a panel's dim covered — so Copy debug
// info, from a press made inside a panel, could not be reached without closing
// the panel. A toast stands over every layer but the palette, and its acts are
// pressable where it appears. Which failures are toasts, and which stay
// banners, is `failing.ts`.
//
// **Several stack, and none goes on a timer.** A failure is evidence; one that
// expired while somebody read the panel it came from is one they cannot get
// back. A copy confirmation still goes on its own, below them.
// A press the form would not send is one toast per kind, replaced, not five.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import type { BridgeIdentity, Outcome } from "@armada/protocol";
import { CopiedToast, FailureToast, GuidanceToast, SaidToast, ToastRegion, watchUncaught } from "@armada/shell";
import type { Raised } from "./failing";
import { failedCommand, guidanceOf, raisedFailure } from "./failing";

/**
 * The toasts standing, oldest first. Each command answer that failed raises
 * one, and so does each throw no boundary saw; guidance replaces its own kind.
 */
export function useRaised(outcome: Outcome | null): { raised: Raised[]; lower: (key: number) => void; tell: (sentence: string) => void } {
  const [raised, setRaised] = useState<Raised[]>([]);
  const next = useRef(0);
  const raise = useCallback((one: Omit<Raised, "key">) => {
    const key = next.current++;
    setRaised((held) => [...held, { ...one, key } as Raised]);
  }, []);
  // Each answer is a new object, so a second failed press is a second toast.
  useEffect(() => {
    if (outcome === null) return;
    if (failedCommand(outcome)) raise({ outcome });
    else if (guidanceOf(outcome) !== null) {
      setRaised((held) => held.filter((one) => !sameGuidance(one, outcome)));
      raise({ outcome });
    }
  }, [outcome, raise]);
  useEffect(() => watchUncaught((uncaught) => raise({ uncaught })), [raise]);
  const lower = useCallback((key: number) => setRaised((held) => held.filter((one) => one.key !== key)), []);
  // A sentence a surface with no command of its own has to say, raised as the refusal it is.
  const tell = useCallback(
    (message: string) =>
      raise({ outcome: { ok: false, why: "refused", error: { code: "bridge.not_sent", message, run_id: "", fields: {}, chain: [] } } }),
    [raise],
  );
  return { raised, lower, tell };
}

/** Whether a toast is guidance of the same kind as this answer. */
function sameGuidance(one: Raised, outcome: Outcome): boolean {
  if (!("outcome" in one) || guidanceOf(one.outcome) === null) return false;
  return !one.outcome.ok && !outcome.ok && one.outcome.why === outcome.why;
}

export type ToastsProps = {
  raised: Raised[];
  bridge: BridgeIdentity;
  onLower: (key: number) => void;
  copied: string | null;
  said: string | null;
  onCopied: (value: string) => void;
  /** Toasts of another owner's that stand in this column too, so two never sit on one spot. */
  children?: ReactNode;
};

/** Every toast in the window, in one column. */
export function Toasts({ raised, bridge, onLower, copied, said, onCopied, children }: ToastsProps) {
  return (
    <ToastRegion>
      {raised.map((one) => {
        const failure = raisedFailure(one, bridge);
        const guidance = "outcome" in one ? guidanceOf(one.outcome) : null;
        if (guidance !== null) return <GuidanceToast key={one.key} said={guidance} onDismiss={() => onLower(one.key)} />;
        return failure === null ? null : (
          <FailureToast
            key={one.key}
            failure={failure}
            onCopied={onCopied}
            onDismiss={() => onLower(one.key)}
            // A throw is re-run by a redraw; a command is not.
            reloadable={"uncaught" in one}
          />
        );
      })}
      <CopiedToast copied={copied} />
      <SaidToast said={said} />
      {children}
    </ToastRegion>
  );
}
