// A Job's call card, from the Now Fleet serves for it: what asks the owner and what went wrong. The
// request and where the Job lives are the draft's, which a real Fleet does not serve here.

import type { CallAskView, CallIssueView, CallView } from "./draft/calls";
import type { NowView } from "./draft/now";

export function callViewOf(now: NowView | undefined): CallView | undefined {
  if (now === undefined) return undefined;
  const asks = (now.asks ?? []).flatMap((ask): CallAskView[] =>
    ask.kind === "plan" || ask.kind === "commander"
      ? []
      : [{ key: ask.key, kind: ask.kind, name: ask.name, text: ask.text, ...(ask.target === undefined ? {} : { target: ask.target }), ...(ask.asked === undefined ? {} : { asked: ask.asked }) }],
  );
  const issues = (now.issues ?? []).map(
    (one): CallIssueView => ({ key: one.key, of: one.of, name: one.name, text: one.text, said: one.said, ...(one.target === undefined ? {} : { target: one.target }), ...(one.acts === undefined ? {} : { acts: one.acts.map(({ glyph, said }) => ({ glyph, said })) }) }),
  );
  const running = now.running;
  return { served: true, ...(asks.length === 0 ? {} : { asks }), ...(issues.length === 0 ? {} : { issues }), ...(running === undefined ? {} : { running }) };
}
