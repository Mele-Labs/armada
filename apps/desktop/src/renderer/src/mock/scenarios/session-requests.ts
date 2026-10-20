// A Job whose agent sent a GraphQL POST to `{{base}}/graphql` on staging and got 401, and asked for a cookie. Sent from
// Requests with the Job open, the owner's request is a row in the thread, and the agent acts on a 200. Over the `sessions`
// scenario; the walk `requests-with-the-owners-cookie` plays it. Every host is `example.test`.

import type { RequestSent, SendRequest } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { RequestsSeed } from "../slices/requests";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const JOB = "s61";
const BASE = "https://api.example.test";
const QUERY = '{"query":"{ me { id email } }"}';
const JSON_HEADER = [{ name: "Content-Type", value: "application/json" }];

const NOT_SIGNED_IN: Omit<RequestSent, "url" | "cookie_host"> = {
  status: 401,
  ms: 184,
  headers: [{ name: "content-type", value: "application/json" }],
  body: '{"errors":[{"message":"Not signed in","extensions":{"code":"UNAUTHENTICATED"}}]}',
  truncated: false,
};

const SIGNED_IN: Omit<RequestSent, "url" | "cookie_host"> = {
  status: 200,
  ms: 212,
  headers: [
    { name: "content-type", value: "application/json" },
    { name: "x-request-id", value: "req_4f2a" },
  ],
  body: '{"data":{"me":{"id":"u_7","email":"owner@example.test"}}}',
  truncated: false,
};

const checking: Session = {
  id: JOB,
  title: "Account page shows nobody signed in",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "q1", at: minutesAgo(9), kind: "message", from: { kind: "you" }, text: "The account page on staging shows nobody signed in. Find out why." },
    {
      id: "q2",
      at: minutesAgo(5),
      kind: "requested",
      by: "agent",
      method: "POST",
      url: `${BASE}/graphql`,
      environment: "staging",
      headers: JSON_HEADER,
      body: QUERY,
      status: 401,
      ms: 184,
      response: NOT_SIGNED_IN.body,
      truncated: false,
    },
    {
      id: "q3",
      at: minutesAgo(3),
      kind: "message",
      from: { kind: "agent" },
      text: "Staging answers `me` with 401 and no session. Add a cookie for `api.example.test` in Requests and send the request again, and I will read the answer.",
    },
  ],
  attachments: [{ kind: "slot", slot: 2 }, { kind: "branch", name: "fix/account-signed-in", slot: 2 }],
};

/** The window's Sessions while it is up, so a send can write the owner's row and the agent can answer it. */
let live: { told: (send: SendRequest, url: string, answer: Omit<RequestSent, "url" | "cookie_host">) => void } | undefined;

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [checking]);
  const timers = new Set<number>();
  let rows = 0;
  const after = (ms: number, run: () => void) => {
    const handle = window.setTimeout(() => {
      timers.delete(handle);
      run();
    }, ms);
    timers.add(handle);
  };
  const add = (make: (rowId: string, at: string) => SessionRow) => store.edit(JOB, (one) => ({ ...one, rows: [...one.rows, make(`r${(rows += 1)}`, new Date().toISOString())] }));
  live = {
    told: (send, url, answer) => {
      add((id, at) => ({
        id,
        at,
        kind: "requested",
        by: "owner",
        method: send.method,
        url,
        ...(send.environment === undefined ? {} : { environment: send.environment }),
        headers: send.headers,
        ...(send.body === undefined ? {} : { body: send.body }),
        status: answer.status ?? null,
        ms: answer.ms,
        response: answer.body,
        truncated: false,
        ...(answer.status === 200 ? { cookieHost: "api.example.test" } : {}),
      }));
      if (answer.status !== 200) return;
      after(700, () => store.edit(JOB, (one) => ({ ...one, turn: { state: "working" } })));
      after(2000, () => {
        add((id, at) => ({
          id,
          at,
          kind: "message",
          from: { kind: "agent" },
          text: "With your cookie `me` is `u_7`, so the session is good. The account page asks for `me` before the cookie is set. Moving the read after sign in.",
        }));
        store.edit(JOB, (one) => ({ ...one, turn: { state: "idle" }, lastTurnAt: new Date().toISOString() }));
      });
    },
  };
  return {
    ...store,
    dispose: () => {
      live = undefined;
      timers.forEach((one) => window.clearTimeout(one));
      store.dispose();
    },
  };
}

const requests: RequestsSeed = {
  environments: [{ name: "staging", base: BASE }],
  servers: { [JOB]: [{ name: "server:web", base: "http://127.0.0.1:5173" }] },
  // Staging answers 401 to a request that carries no cookie, and 200 to one that does. A send with the Job open is its row.
  answer: (send, url, cookieHost) => {
    const answer = cookieHost === undefined ? { ...NOT_SIGNED_IN, note: "api.example.test holds no cookie. The owner can give one in Requests." } : SIGNED_IN;
    if (send.session_id === JOB) live?.told(send, url, answer);
    return answer;
  },
};

export const s311SessionRequests: Scenario = {
  ...s200Sessions,
  name: "session-requests",
  says: "A Job whose agent got 401 from staging's GraphQL, and Requests with the owner's cookie",
  draft: { ...s200Sessions.draft, sessions: sessionsOf, requests },
};
