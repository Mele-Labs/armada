import { GitBranch, KeyRound, SquareTerminal } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Prose } from "../../primitives/Prose/Prose";

/**
 * A Session's conversation, drawn on Helm's own thread rows
 * (`HelmThread.css`) with the two things Helm's thread has no room for.
 *
 * **A message another Session wrote is set apart**: its own band, with the
 * Session mark and the sender's title, over the words, and the band is a press
 * that opens that Session. It is never folded into the agent's voice, which
 * the owner found too easy to mistake it for (6 Oct 2026).
 *
 * **The first write is a row of its own**, filled in the running hue, because
 * it is the one moment a Session stops being blank: the slot leased and the
 * branch cut, in the thread where it happened.
 */
export type SessionThreadRow =
  | {
      id: string;
      at: string;
      kind: "message";
      from: "you" | "agent";
      text: string;
      /** What was sent with it. Pictures are drawn; other files are chips. */
      files?: readonly { id: string; name: string; src?: string }[];
      sketches?: readonly { id: string; title: string }[];
      tags?: readonly { kind: "session" | "job" | "pull_request" | "branch"; id: string; title: string }[];
    }
  | { id: string; at: string; kind: "message"; from: "session"; sender: { id: string; title: string }; text: string }
  /** A tool call, mono. */
  | { id: string; at: string; kind: "tool"; text: string }
  | { id: string; at: string; kind: "lease"; slot: number; branch: string };

export type SessionThreadProps = {
  rows: readonly SessionThreadRow[];
  /** The permission the agent is held on. */
  asked?: { command: string };
  onAnswer: () => void;
  /** Opens the Session a message came from. */
  onOpenSession: (sessionId: string) => void;
};

const TAG_KIND = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch" } as const;

function Sent({ row }: { row: Extract<SessionThreadRow, { from: "you" | "agent" }> }) {
  const files = row.files ?? [];
  const sketches = row.sketches ?? [];
  const tags = row.tags ?? [];
  if (files.length + sketches.length + tags.length === 0) return null;
  return (
    <div className="armada-session-sent">
      {files.map((file) =>
        file.src === undefined ? (
          <AttachmentChip key={file.id} filename={file.name} />
        ) : (
          <img key={file.id} className="armada-session-sent__picture" src={file.src} alt={file.name} />
        ),
      )}
      {sketches.map((one) => (
        <AttachmentChip key={one.id} filename={one.title} from="Sketch" />
      ))}
      {tags.map((one) => (
        <AttachmentChip key={`${one.kind}${one.id}`} filename={one.title} from={TAG_KIND[one.kind]} />
      ))}
    </div>
  );
}

function Row({ row, onOpenSession }: { row: SessionThreadRow; onOpenSession: (id: string) => void }) {
  if (row.kind === "lease") {
    return (
      <li className="armada-session-lease" role="region" aria-label="Leased on first write">
        <span className="armada-session-lease__eyebrow">First write</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
            <KeyRound size={12} strokeWidth={2} aria-hidden />
            {row.slot}
          </span>
          <span className="armada-session-lease__chip" role="img" aria-label={`Branch ${row.branch}`}>
            <GitBranch size={12} strokeWidth={2} aria-hidden />
            {row.branch}
          </span>
        </span>
        <span className="armada-session-lease__at">{row.at}</span>
      </li>
    );
  }
  if (row.kind === "tool") {
    return (
      <li className="armada-helm-thread__row" data-actor="helm">
        <p className="armada-helm-thread__message" data-mono="true">
          {row.text}
        </p>
      </li>
    );
  }
  if (row.from === "session") {
    const { sender } = row;
    return (
      <li className="armada-session-from" role="region" aria-label={`Message from ${sender.title}`}>
        <button type="button" className="armada-session-from__band" aria-label={`Open Session ${sender.title}`} onClick={() => onOpenSession(sender.id)}>
          <SquareTerminal size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-from__id">{sender.id}</span>
          <span className="armada-session-from__title">{sender.title}</span>
          <span className="armada-session-from__at">{row.at}</span>
        </button>
        <div className="armada-session-from__body">
          <Prose text={row.text} />
        </div>
      </li>
    );
  }
  return (
    <li className="armada-helm-thread__row" data-actor={row.from === "you" ? "you" : "helm"} data-from={row.from}>
      <div className="armada-helm-thread__head">
        <span className="armada-helm-thread__who">{row.from === "you" ? "You" : "Agent"}</span>
        <span className="armada-helm-thread__at">{row.at}</span>
      </div>
      <div className="armada-helm-thread__message">
        <Prose text={row.text} />
      </div>
      <Sent row={row} />
    </li>
  );
}

export function SessionThread({ rows, asked, onAnswer, onOpenSession }: SessionThreadProps) {
  return (
    <div className="armada-session-thread">
      <div className="armada-session-thread__rows" role="region" aria-label="Thread">
        <ol className="armada-helm-thread__rows">
          {rows.map((row) => (
            <Row key={row.id} row={row} onOpenSession={onOpenSession} />
          ))}
        </ol>
      </div>
      {asked === undefined ? null : (
        <Card flat className="armada-session-thread__ask" role="article" aria-label="Waiting on you">
          <span className="armada-session-thread__eyebrow">Permission</span>
          <p className="armada-session-thread__command">{asked.command}</p>
          <div className="armada-session-thread__answers" role="group" aria-label="Answers">
            <Button size="sm" variant="secondary" onClick={onAnswer}>
              Allow once
            </Button>
            <Button size="sm" variant="secondary" onClick={onAnswer}>
              Deny
            </Button>
          </div>
        </Card>
      )}
    </div>
  );
}
