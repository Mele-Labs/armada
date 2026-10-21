// A Job whose agent saved its Create order request to the owner's Store API collection when he asked for the query. Sent from
// Requests with the Job open and the `test` environment he adds, his request is a row in the thread and the agent acts on a 200.
// The repository commits a Billing API his own hides, and a Health collection. Over the `sessions` scenario; the walk
// `requests-collections-and-environments` plays it. Every host is `example.test`, and every name is invented.

import type { RequestCollection, RequestSent } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { OwnSend, RequestsSeed } from "../slices/requests";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const JOB = "s61";
const JSON_HEADER = [{ name: "Content-Type", value: "application/json" }];

const CREATED: Omit<RequestSent, "url"> = {
  status: 200,
  ms: 212,
  headers: [
    { name: "content-type", value: "application/json" },
    { name: "x-request-id", value: "req_4f2a" },
  ],
  body: '{"order":{"id":"o_12","status":"created","sku":"sku_42","quantity":1}}',
  truncated: false,
};

const NOT_SIGNED_IN: Omit<RequestSent, "url"> = {
  status: 401,
  ms: 184,
  headers: [{ name: "content-type", value: "application/json" }],
  body: '{"error":"no bearer token"}',
  truncated: false,
};

const storeApi: RequestCollection = {
  id: "store-api",
  name: "Store API",
  headers: [{ name: "Authorization", value: "Bearer {{token}}" }],
  requests: [
    { id: "create-order", name: "Create order", method: "POST", url: "{{base}}/orders", headers: JSON_HEADER, body: '{"sku":"sku_42","quantity":1}' },
    { id: "get-order", name: "Get order", method: "GET", url: "{{base}}/orders/o_11", headers: [] },
  ],
};

const billingYours: RequestCollection = {
  id: "billing-api",
  name: "Billing API",
  headers: [{ name: "Authorization", value: "Bearer {{token}}" }],
  requests: [{ id: "list-invoices", name: "List invoices", method: "GET", url: "{{base}}/invoices", headers: [] }],
};

const billingRepository: RequestCollection = {
  id: "billing-api",
  name: "Billing API",
  headers: [],
  requests: [{ id: "latest-invoice", name: "Latest invoice", method: "GET", url: "{{base}}/invoices/latest", headers: [] }],
};

const health: RequestCollection = {
  id: "health",
  name: "Health",
  headers: [],
  requests: [{ id: "health", name: "Health", method: "GET", url: "{{base}}/health", headers: [] }],
};

const ordering: Session = {
  id: JOB,
  title: "Order confirmation shows no order",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "q1", at: minutesAgo(9), kind: "message", from: { kind: "you" }, text: "Give me the query you ran to create an order, so I can send it against test myself." },
    { id: "q2", at: minutesAgo(5), kind: "saved", name: "Create order", collection: "Store API", collectionId: "store-api", request: "create-order" },
    {
      id: "q3",
      at: minutesAgo(3),
      kind: "message",
      from: { kind: "agent" },
      text: "Saved Create order to Store API. It posts to `{{base}}/orders`, and the collection sends `Authorization: Bearer {{token}}`. An environment with `base` and `token` fills both.",
    },
  ],
  attachments: [{ kind: "slot", slot: 2 }, { kind: "branch", name: "fix/order-confirmation", slot: 2 }],
};

/** The window's Sessions while it is up, so a send can write the owner's row and the agent can answer it. */
let live: { told: (send: OwnSend, url: string, answer: Omit<RequestSent, "url">) => void } | undefined;

function sessionsOf(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]) {
  const store = s200Sessions.draft!.sessions!(board, [ordering]);
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
      // The row holds the request as written: `{{token}}` stays a name, so no secret reaches the thread.
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
      }));
      if (answer.status !== 200) return;
      after(700, () => store.edit(JOB, (one) => ({ ...one, turn: { state: "working" } })));
      after(2000, () => {
        add((id, at) => ({
          id,
          at,
          kind: "message",
          from: { kind: "agent" },
          text: "Create order on test answered 200 with order `o_12`. The confirmation page reads the order before the redirect carries its id. Passing `o_12` in the redirect.",
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
  yours: { collections: [storeApi, billingYours] },
  repository: { collections: [billingRepository, health], environments: [{ name: "local", variables: [{ name: "base", secret: false, value: "https://api.local.example.test" }] }] },
  servers: { [JOB]: [{ name: "web", base: "http://127.0.0.1:5173" }] },
  // Every `{{name}}` filled answers 200; a token left unfilled answers 401. A send with the Job open is its row.
  answer: (send, url, filled) => {
    const answer = filled ? CREATED : { ...NOT_SIGNED_IN, note: "{{token}} was not filled. Pick an environment that sets token." };
    if (send.session_id === JOB) live?.told(send, url, answer);
    return answer;
  },
};

export const s311SessionRequests: Scenario = {
  ...s200Sessions,
  name: "session-requests",
  says: "A Job whose agent saved Create order to Store API, and Requests with collections and environments",
  draft: { ...s200Sessions.draft, sessions: sessionsOf, requests },
};
