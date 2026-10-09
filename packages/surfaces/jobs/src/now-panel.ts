// The draft's Now view, wired to the host's presses. **A stub where the host has nowhere to go**:
// a Drone opens its panel, a Check opens its log, a waiting Job opens that Job, and a quick act
// says what it would do (the mock has no Fleet command behind any of them).

import type { NowAct, NowAsk, NowIssue, NowPanelProps, NowRunning, NowWaiting } from "@armada/components";

import type { NowActView, NowView } from "./draft/now";
import type { JobCheckLog } from "./check-log-sheet";

export type NowHost = {
  onOpenDrone?: ((droneId: string) => void) | undefined;
  onOpenCheckLog?: ((log: JobCheckLog) => void) | undefined;
  onOpenJob?: ((jobId: string) => void) | undefined;
  onSaid: (sentence: string) => void;
};

export function nowPanelOf(view: NowView | undefined, host: NowHost): Omit<NowPanelProps, "onHide"> | undefined {
  if (view === undefined) return undefined;
  const opens = (of: string, target: string | undefined, state?: string) => () => {
    if (target === undefined) return;
    if (of === "drone") host.onOpenDrone?.(target);
    if (of === "check") host.onOpenCheckLog?.({ name: target, kept: `${target}.log`, live: state === "running" });
  };
  const acts = (name: string, given: readonly NowActView[] | undefined): NowAct[] =>
    (given ?? []).map((act) => ({ ...act, onAct: () => host.onSaid(`${act.said}: ${name}`) }));
  const asks: NowAsk[] = (view.asks ?? []).map((ask) =>
    ask.kind === "plan"
      ? { ...ask, onAnswer: () => host.onSaid("Answer sent") }
      : { key: ask.key, kind: ask.kind, name: ask.name, text: ask.text, ...(ask.sketch === undefined ? {} : { sketch: ask.sketch }), ...(ask.asker === undefined ? {} : { asker: ask.asker }), onOpen: opens(ask.kind, ask.target) },
  );
  const issues: NowIssue[] = (view.issues ?? []).map(({ target, acts: given, ...one }) => ({
    ...one,
    ...(given === undefined ? {} : { acts: acts(one.name, given) }),
    onOpen: opens(one.of, target),
  }));
  const running: NowRunning[] = (view.running ?? []).map(({ target, acts: given, ...one }) => ({
    ...one,
    ...(given === undefined ? {} : { acts: acts(one.name, given) }),
    onOpen: opens(one.of, target, one.state),
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
    ...(running.some((one) => one.of === "check") ? { onSkipAll: () => host.onSaid("Skip all checks") } : {}),
  };
}
