import type { WritePullRequest } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { SendKbd, sendsOn } from "../../send-message";
import { mayWrite } from "./pull-request-format";

/**
 * The comment box pinned to the bottom of a pull request's panel: a textarea, the switch between a
 * comment, an approval and a request for changes, and one send. ⌘↵ sends. **A comment and a request
 * for changes need words; an approval does not.** The host keeps the text, and the textarea keeps
 * the draft across leaving the panel under `keep`.
 */
export type PullRequestComposerProps = {
  kind: WritePullRequest["kind"];
  onKind: (kind: WritePullRequest["kind"]) => void;
  value: string;
  onChange: (value: string) => void;
  /** What the draft is kept under: the Session and the pull request. */
  keep: string;
  /** The write is out and Fleet has not answered. */
  pending?: boolean;
  /** What Fleet refused, in its words. The draft stays. */
  said?: string;
  onSend: () => void;
};

const KINDS = [
  { id: "comment", label: "Comment" },
  { id: "approve", label: "Approve" },
  { id: "request_changes", label: "Request changes" },
];

const UNDERWAY = { comment: "Commenting", approve: "Approving", request_changes: "Requesting changes" } as const;

export function PullRequestComposer({ kind, onKind, value, onChange, keep, pending = false, said, onSend }: PullRequestComposerProps) {
  const ready = mayWrite(kind, value);
  const label = KINDS.find((one) => one.id === kind)!.label;
  return (
    <form
      className="armada-pr-composer"
      aria-label="Comment box"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready && !pending) onSend();
      }}
    >
      <Textarea
        rows={3}
        keep={keep}
        aria-label="Comment"
        value={value}
        disabled={pending}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (sendsOn(event) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            if (ready && !pending) onSend();
          }
        }}
      />
      {said === undefined || said === "" ? null : (
        <p className="armada-pr-composer__said" role="alert">
          {said}
        </p>
      )}
      <div className="armada-pr-composer__bar">
        <Tabs items={KINDS} value={kind} onChange={(id) => onKind(id as WritePullRequest["kind"])} />
        <Button variant="primary" type="submit" pending={pending} disabled={!ready}>
          {pending ? UNDERWAY[kind] : label}
          <SendKbd available={ready && !pending} />
        </Button>
      </div>
    </form>
  );
}
