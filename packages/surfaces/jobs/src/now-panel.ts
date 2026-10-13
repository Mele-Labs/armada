// The Now view, wired to the host's presses. A Drone opens its panel, a Check opens its log, a
// waiting Job opens that Job. **A quick act goes to `onAct` where the host has a Fleet command
// behind it**, and says what it would do where it has not (the mock, and the Cockpit's cards).

import type { NowAct, NowAsk, NowIssue, NowPanelProps, NowRunning, NowWaiting } from "@armada/components";

import type { NowActView, NowView } from "./draft/now";
import type { JobCheckLog } from "./check-log-sheet";

/** A quick act pressed, and the Check or Drone its row is about. `skip_all` names neither. */
export type NowActed = { glyph: NowActView["glyph"]; check?: string; drone?: string };

export type NowHost = {
  onOpenDrone?: ((droneId: string) => void) | undefined;
  onOpenCheckLog?: ((log: JobCheckLog) => void) | undefined;
  onOpenJob?: ((jobId: string) => void) | undefined;
  onAct?: ((acted: NowActed) => void) | undefined;
  /** The owner's answer to the Commander's card: the card's key, then the answer. */
  onAnswerCommander?: ((key: string, option: string) => void) | undefined;
  onSaid: (sentence: string) => void;
};

export function nowPanelOf(view: NowView | undefined, host: NowHost): Omit<NowPanelProps, "onHide"> | undefined {
  if (view === undefined) return undefined;
  const opens = (of: string, target: string | undefined, state?: string) => () => {
    if (target === undefined) return;
    if (of === "drone") host.onOpenDrone?.(target);
    if (of === "check") host.onOpenCheckLog?.({ name: target, kept: `${target}.log`, live: state === "running" });
  };
  const acts = (of: string, name: string, target: string | undefined, given: readonly NowActView[] | undefined): NowAct[] =>
    (given ?? []).map((act) => ({
      ...act,
      onAct: () => {
        if (host.onAct === undefined) return host.onSaid(`${act.said}: ${name}`);
        host.onAct({ glyph: act.glyph, ...(of === "check" ? { check: name } : {}), ...(of === "drone" && target !== undefined ? { drone: target } : {}) });
      },
    }));
  const asks: NowAsk[] = (view.asks ?? []).map((ask) =>
    ask.kind === "commander"
      ? { ...ask, onAnswer: (option: string) => host.onAnswerCommander?.(ask.key, option) }
      : ask.kind === "plan"
      ? { ...ask, onAnswer: () => host.onSaid("Answer sent") }
      : { key: ask.key, kind: ask.kind, name: ask.name, text: ask.text, ...(ask.sketch === undefined ? {} : { sketch: ask.sketch }), ...(ask.asker === undefined ? {} : { asker: ask.asker }), onOpen: opens(ask.kind, ask.target) },
  );
  const issues: NowIssue[] = (view.issues ?? []).map(({ target, acts: given, ...one }) => ({
    ...one,
    ...(given === undefined ? {} : { acts: acts(one.of, one.name, target, given) }),
    onOpen: opens(one.of, target),
  }));
  const running: NowRunning[] = (view.running ?? []).map(({ target, log, acts: given, ...one }) => ({
    ...one,
    ...(given === undefined ? {} : { acts: acts(one.of, one.name, target, given) }),
    onOpen: log === undefined ? opens(one.of, target, one.state) : () => host.onOpenCheckLog?.(log),
  }));
  const waiting: NowWaiting[] = (view.waiting ?? []).map(({ target, ...one }) => ({
    ...one,
    ...(one.kind === "job" && target !== undefined && host.onOpenJob !== undefined ? { onOpen: () => host.onOpenJob?.(target) } : {}),
  }));
  return {
    onOpenFile: (path: string) => host.onSaid(`Open ${path}`),
    asks,
    issues,
    running,
    waiting,
    ...(running.some((one) => one.of === "check") ? { onSkipAll: () => (host.onAct === undefined ? host.onSaid("Skip all checks") : host.onAct({ glyph: "skip_all" })) } : {}),
  };
}
