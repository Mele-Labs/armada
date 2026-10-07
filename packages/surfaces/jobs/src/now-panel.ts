// The draft's Now view, wired to the host's presses. **A stub where the host
// has nowhere to go**: only a Drone opens today, from the Drone's id.

import type { NowAsk, NowIssue, NowPanelProps, NowRunning } from "@armada/components";

import type { NowView } from "./draft/now";

export function nowPanelOf(view: NowView | undefined, onOpenDrone: ((droneId: string) => void) | undefined): Omit<NowPanelProps, "onHide"> | undefined {
  if (view === undefined) return undefined;
  const opens = (of: string, target: string | undefined) => () => {
    if (of === "drone" && target !== undefined) onOpenDrone?.(target);
  };
  const asks: NowAsk[] = (view.asks ?? []).map((ask) =>
    ask.kind === "plan"
      ? { ...ask, onAnswer: () => {} }
      : { key: ask.key, kind: ask.kind, name: ask.name, text: ask.text, onOpen: opens(ask.kind, ask.target) },
  );
  const issues: NowIssue[] = (view.issues ?? []).map(({ target, ...one }) => ({ ...one, onOpen: opens(one.of, target) }));
  const running: NowRunning[] = (view.running ?? []).map(({ target, ...one }) => ({ ...one, onOpen: opens(one.of, target) }));
  return { asks, issues, running };
}
