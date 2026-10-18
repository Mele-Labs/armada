// A Session's pull request, as the panel draws it: the attachment Fleet served and the merge lines
// it is in, read into `@armada/components`' panel. Apart from `sessions.tsx`, which is near its limit.

import { useCallback, useState } from "react";
import type { ReactNode } from "react";
import { PullRequestPanel } from "@armada/components";
import type { PullRequestActsProps } from "@armada/components";
import { placeOf } from "@armada/screens";
import { usePullRequestPage } from "./session-pull-request-page";
import { usePullRequestFile } from "./session-pull-request-file";
import { PrOwnersOnTheWire } from "./pr-owners-wire";
import type { MergeLines } from "@armada/protocol";
import type { PullRequestAct, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

type PullRequest = Extract<SessionAttachment, { kind: "pull_request" }>;

/** `owner/name`, where the address is a forge's `/<owner>/<name>/pull/<number>`. Absent where it is not. */
export const repositoryOf = (address: string): string | undefined => {
  try {
    return /^\/([^/]+\/[^/]+)\/pull\/\d+/.exec(new URL(address).pathname)?.[1];
  } catch {
    return undefined;
  }
};

/** A description without the HTML comments a forge's template leaves for its author: Prose would draw them as characters. */
export const promptsOut = (body: string | undefined): string => (body ?? "").replace(/<!-{2}[\s\S]*?-{2}>/g, "").trim();

/** The failing checks' names: Fleet serves them as one string, joined with a comma and a space. */
export const failingOf = (one: PullRequest): string[] =>
  (one.checks.state === "failed" ? one.checks.failing : (one.checks.red ?? "")).split(", ").filter((name) => name !== "");

const keyOf = (sessionId: string, one: PullRequest) => `${sessionId}#${one.number}:${failingOf(one).join(",")}`;

/**
 * Tells a Session its pull request's checks failed, as a message it is sent the way the composer's
 * is. **Kept per pull request and per set of failing checks**, so a second failure asks again.
 */
export function useSendBack(sessionId: string, send: SessionsDraft["send"] | undefined) {
  const [sent, setSent] = useState<ReadonlySet<string>>(new Set());
  const sendBack = useCallback(
    (one: PullRequest) => {
      const names = failingOf(one);
      const text = names.length === 0 ? `Checks failed on #${one.number}` : `Checks failed on #${one.number}: ${names.join(", ")}`;
      const key = keyOf(sessionId, one);
      void Promise.resolve(send?.(sessionId, { text, files: [], sketches: [], tags: [] })).then((went) => {
        if (went !== false) setSent((was) => new Set(was).add(key));
      });
    },
    [sessionId, send],
  );
  return { sentBack: (one: PullRequest) => sent.has(keyOf(sessionId, one)), sendBack };
}

/** The panel, remounted for each pull request so one's page and draft never show on another. */
export function SessionPullRequest(props: PanelProps) {
  return <SessionPullRequestPanel key={`${props.sessionId}#${props.one.number}`} {...props} />;
}

type PanelProps = {
  sessionId: string;
  draft: SessionsDraft | undefined;
  one: PullRequest;
  /** What Fleet serves of the merge lines, for the place the pull request holds in one. */
  lines: MergeLines | null;
  floor: boolean;
  pending: PullRequestAct | undefined;
  sentBack: boolean;
  onAct: PullRequestActsProps["onAct"];
  onSendBack: () => void;
  onOpenLink: (address: string) => void;
  onClose: () => void;
  children?: ReactNode;
};

function SessionPullRequestPanel({
  sessionId,
  draft,
  one,
  lines,
  floor,
  pending,
  sentBack,
  onAct,
  onSendBack,
  onOpenLink,
  onClose,
  children,
}: PanelProps) {
  const place = placeOf(lines, one);
  const repository = repositoryOf(one.address);
  const description = promptsOut(one.body);
  const { page, pageSaid, composer } = usePullRequestPage(sessionId, one.number, draft);
  const { file, onFile } = usePullRequestFile(sessionId, one.number, draft);
  return (
    <PrOwnersOnTheWire lines={lines} panel={{ repository, number: one.number, owner: page?.owner }}>
      <PullRequestPanel
        number={one.number}
        title={one.title}
        branch={one.branch}
        address={one.address}
        {...(repository === undefined ? {} : { repository })}
        {...(description === "" ? {} : { body: description })}
        {...(page === undefined ? {} : { page })}
        {...(pageSaid === undefined ? {} : { pageSaid })}
        {...(composer === undefined ? {} : { composer })}
        {...(file === undefined ? {} : { file })}
        {...(onFile === undefined ? {} : { onFile })}
        state={one.state}
        // A check red after the gate passed still reads as failed here: Fleet refuses the merge on it.
        checks={one.checks.state === "failed" || (one.checks.state === "passed" && one.checks.red !== undefined) ? { state: "failed", failing: failingOf(one) } : { state: one.checks.state }}
        auto={one.auto}
        queued={one.queued === true}
        {...(place === undefined ? {} : { place })}
        floor={floor}
        {...(pending === undefined ? {} : { pending })}
        onAct={onAct}
        onSendBack={onSendBack}
        sentBack={sentBack}
        onOpenLink={onOpenLink}
        onClose={onClose}
      >
        {children}
      </PullRequestPanel>
    </PrOwnersOnTheWire>
  );
}
