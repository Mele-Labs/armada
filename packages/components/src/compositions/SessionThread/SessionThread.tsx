import { useState } from "react";
import type { FormEvent } from "react";
import { GitBranch, KeyRound, Send } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";
import { Prose } from "../../primitives/Prose/Prose";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * A Session's conversation, drawn on Helm's own thread rows
 * (`HelmThread.css`) with the one thing Helm's thread has no room for: **a
 * message another Session wrote, named by its sender**. Three voices, and the
 * third is a Session, never folded into the agent's.
 *
 * **The first write is a row of its own**, filled in the running hue, because
 * it is the one moment a Session stops being blank: the slot leased and the
 * branch cut, in the thread where it happened.
 */
export type SessionThreadRow =
  | { id: string; at: string; kind: "message"; from: "you" | "agent"; text: string }
  | { id: string; at: string; kind: "message"; from: "session"; sender: { id: string; title: string }; text: string }
  /** A tool call, mono. */
  | { id: string; at: string; kind: "tool"; text: string }
  | { id: string; at: string; kind: "lease"; slot: number; branch: string };

export type SessionThreadProps = {
  rows: readonly SessionThreadRow[];
  /** The permission the agent is held on. */
  asked?: { command: string };
  onAnswer: () => void;
  /** A turn is running: Send is off. */
  working: boolean;
  onSend: (text: string) => void;
};

function Row({ row }: { row: SessionThreadRow }) {
  if (row.kind === "lease") {
    return (
      <li className="armada-session-lease" role="region" aria-label="Leased on first write">
        <span className="armada-session-lease__eyebrow">First write</span>
        <span className="armada-session-lease__facts">
          <span className="armada-session-lease__chip" role="img" aria-label={`Slot ${row.slot}`}>
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
  const sender = row.from === "session" ? row.sender : undefined;
  return (
    <li
      className="armada-helm-thread__row"
      data-actor={row.from === "you" ? "you" : "helm"}
      data-from={row.from}
      {...(sender === undefined ? {} : { role: "region", "aria-label": `Message from ${sender.title}` })}
    >
      <div className="armada-helm-thread__head">
        <span className="armada-helm-thread__who">
          {row.from === "you" ? "You" : sender === undefined ? "Agent" : `${sender.id} · ${sender.title}`}
        </span>
        <span className="armada-helm-thread__at">{row.at}</span>
      </div>
      <div className="armada-helm-thread__message">
        <Prose text={row.text} />
      </div>
    </li>
  );
}

export function SessionThread({ rows, asked, onAnswer, working, onSend }: SessionThreadProps) {
  const [text, setText] = useState("");
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (text.trim() === "") return;
    onSend(text.trim());
    setText("");
  };
  return (
    <div className="armada-session-thread">
      <div className="armada-session-thread__rows" role="region" aria-label="Thread">
        <ol className="armada-helm-thread__rows">
          {rows.map((row) => (
            <Row key={row.id} row={row} />
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
      <form className="armada-session-thread__composer" onSubmit={submit}>
        <Textarea aria-label="Message" rows={2} value={text} onChange={(event) => setText(event.target.value)} />
        <Button type="submit" variant="primary" size="sm" disabled={working || text.trim() === ""}>
          <Send size={12} strokeWidth={2} aria-hidden />
          Send
        </Button>
      </form>
    </div>
  );
}
