// The page an API walk plays over, in Armada's walk window: the requests of the walk, each with a button that sends it, and
// under it the response. **The walk's steps press those buttons**, so an API walks the way a web page does and the window
// has one way to walk anything. Mock only: the responses are the ones the walk was written with, and nothing is sent.

import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Check, Play, Send, X } from "lucide-react";
import { Badge, Button, PrMark, Tooltip } from "@armada/components";

import { sendName } from "./walk-model";
import type { ApiStep } from "./walk-model";
import "./walk.css";

type Sent = "sending" | "sent";

export function WalkRunner({ base, requests }: { base: string; requests: readonly ApiStep[] }) {
  const [sent, setSent] = useState<Readonly<Record<string, Sent>>>({});
  const send = (id: string) => {
    if (sent[id] !== undefined) return;
    setSent((was) => ({ ...was, [id]: "sending" }));
    window.setTimeout(() => setSent((was) => ({ ...was, [id]: "sent" })), 300);
  };
  return (
    <main className="pr-walk pr-runner" aria-label="Requests">
      <p className="pr-walk__against">
        <PrMark Glyph={Send} said="API base address" />
        <code className="pr-walk__code">{base}</code>
      </p>
      <ol className="pr-walk__requests">
        {requests.map((step, at) => {
          const ok = step.response.status < 400;
          return (
            <li key={step.id}>
              <section className="pr-change pr-walk__request" aria-label={`Request ${at + 1}`}>
                <header className="pr-change__head">
                  <span className="pr-steps__n mono">{at + 1}</span>
                  <code className="pr-walk__verb">{step.method}</code>
                  <code className="pr-walk__path">{step.path}</code>
                  <span className="pr-change__marks">
                    <Button variant="ghost" size="sm" aria-label={sendName(step, at)} pending={sent[step.id] === "sending"} onClick={() => send(step.id)}>
                      <Play size={16} strokeWidth={2} aria-hidden />
                      Send
                    </Button>
                  </span>
                </header>
                {step.body === undefined ? null : <pre className="pr-walk__json" aria-label="Request body">{step.body}</pre>}
                {sent[step.id] !== "sent" ? null : (
                  <div className="pr-walk__response" role="group" aria-label={`Response ${at + 1}`}>
                    <p className="pr-walk__status">
                      <Badge status={ok ? "completed-success" : "completed-failed"} icon={ok ? Check : X}>
                        {String(step.response.status)}
                      </Badge>
                      <Tooltip label="Time the response took">
                        <span className="pr-walk__ms mono">{`${step.response.ms} ms`}</span>
                      </Tooltip>
                    </p>
                    <pre className="pr-walk__json">{step.response.body}</pre>
                  </div>
                )}
              </section>
            </li>
          );
        })}
      </ol>
    </main>
  );
}

/** The runner on the page, in place of the app. */
export function mountRunner(host: HTMLElement, base: string, requests: readonly ApiStep[]): void {
  createRoot(host).render(<WalkRunner base={base} requests={requests} />);
}
