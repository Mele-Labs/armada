// The page an API walk plays over in the mock's walk window: the requests of the walk, each with a button that sends it, and under it the
// response. **The walk's steps press those buttons**, so an API walks the way a web page does. Electron's main serves the real one
// (`main/capture/runner.ts`); here the responses are the ones the walk was seeded with and nothing is sent.

import { useState } from "react";
import { createRoot } from "react-dom/client";
import { Check, Play, Send, X } from "lucide-react";
import { Badge, Button, PrMark, Tooltip } from "@armada/components";
import type { ReviewRequest } from "@armada/protocol";

import { sendName } from "../../../shared/review-walk";
import "../pr-review/walk.css";
import "./review-walk-runner.css";

export type MockResponse = { status: number; ms: number; body: string };

/** What the page is handed in its address: the base address, and each request with the response it gets back. */
export type RunnerPage = { base: string; requests: readonly { request: ReviewRequest; response: MockResponse }[] };

type Sent = "sending" | "sent";

export function WalkRunner({ base, requests }: RunnerPage) {
  const [sent, setSent] = useState<Readonly<Record<number, Sent>>>({});
  const send = (at: number) => {
    if (sent[at] !== undefined) return;
    setSent((was) => ({ ...was, [at]: "sending" }));
    window.setTimeout(() => setSent((was) => ({ ...was, [at]: "sent" })), 300);
  };
  return (
    <main className="pr-walk pr-runner" aria-label="Requests">
      <p className="pr-walk__against">
        <PrMark Glyph={Send} said="API base address" />
        <code className="pr-walk__code">{base}</code>
      </p>
      <ol className="pr-walk__requests">
        {requests.map(({ request, response }, at) => {
          const ok = response.status < 400;
          return (
            <li key={at}>
              <section className="pr-change pr-walk__request" aria-label={`Request ${at + 1}`}>
                <header className="pr-change__head">
                  <span className="pr-steps__n mono">{at + 1}</span>
                  <code className="pr-walk__verb">{request.method}</code>
                  <code className="pr-walk__path">{request.path}</code>
                  <span className="pr-change__marks">
                    <Button variant="ghost" size="sm" aria-label={sendName(request, at)} pending={sent[at] === "sending"} onClick={() => send(at)}>
                      <Play size={16} strokeWidth={2} aria-hidden />
                      Send
                    </Button>
                  </span>
                </header>
                {request.body === undefined ? null : (
                  <pre className="pr-walk__json" aria-label="Request body">
                    {request.body}
                  </pre>
                )}
                {sent[at] !== "sent" ? null : (
                  <div className="pr-walk__response" role="group" aria-label={`Response ${at + 1}`}>
                    <p className="pr-walk__status">
                      <Badge status={ok ? "completed-success" : "completed-failed"} icon={ok ? Check : X}>
                        {String(response.status)}
                      </Badge>
                      <Tooltip label="Time the response took">
                        <span className="pr-walk__ms mono">{`${response.ms} ms`}</span>
                      </Tooltip>
                    </p>
                    <pre className="pr-walk__json">{response.body}</pre>
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
export function mountRunner(host: HTMLElement, page: RunnerPage): void {
  createRoot(host).render(<WalkRunner {...page} />);
}
