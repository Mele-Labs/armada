// Requests' member, answered from memory for the life of the window. **The proposed members are served here and nowhere
// else** (`shared-panel/requests-model.ts`): the person's own and the repository's collections and environments, the person's
// winning by name, and each Job's servers as `server:<name>`. A send fills `{{name}}` and answers 200 unless the scenario's
// `answer` says otherwise. **No secret is kept here**, only that it is set. Today's members answer from an empty memory.

import type { RequestEnvironmentsFile, WireError } from "@armada/protocol";

import type { RequestsAnswer, RequestsApi } from "../../../../shared/api/requests";
import type {
  Placed,
  ProposedRequestsApi,
  ProposedSend,
  ProposedSent,
  RequestCollection,
  RequestEnvironmentRead,
  RequestsRead,
  VariableRead,
} from "../../shared-panel/requests-model";
import { headersWith } from "../../shared-panel/requests-model";
import type { Slice } from "../fake-context";

/** What a scenario starts Requests with, and how a send is answered. Mock only. */
export type RequestsSeed = {
  yours?: { collections?: readonly RequestCollection[]; environments?: readonly RequestEnvironmentRead[] };
  repository?: { collections?: readonly RequestCollection[]; environments?: readonly RequestEnvironmentRead[] };
  /** The servers each Job runs, by its id, by name without `server:`. */
  servers?: Readonly<Record<string, readonly { name: string; base: string }[]>>;
  /** The answer to a send, given the address filled in and whether every `{{name}}` was. Absent answers 200. */
  answer?: (send: ProposedSend, url: string, filled: boolean) => Omit<ProposedSent, "url">;
};

const refusal = (code: string, message: string): RequestsAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const OK: Omit<ProposedSent, "url"> = { status: 200, ms: 42, headers: [{ name: "content-type", value: "application/json" }], body: '{"ok":true}', truncated: false };

const NAME = /\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g;

/** The person's own over the repository's: a name in both is the person's, marked as hiding the other. */
function placed<T extends { name: string }>(yours: readonly T[], repository: readonly T[]): Placed<T>[] {
  const mine = new Set(yours.map((one) => one.name));
  const theirs = new Set(repository.map((one) => one.name));
  return [
    ...yours.map((one) => ({ ...one, place: "yours" as const, hides: theirs.has(one.name) })),
    ...repository.filter((one) => !mine.has(one.name)).map((one) => ({ ...one, place: "repository" as const, hides: false })),
  ];
}

export const requests: Slice<RequestsApi & ProposedRequestsApi, Record<string, never>> = {
  name: "requests",
  state: {},
  api: (scenario) => {
    const seed = scenario.draft?.requests;
    let collections: RequestCollection[] = [...(seed?.yours?.collections ?? [])];
    let environments: RequestEnvironmentRead[] = [...(seed?.yours?.environments ?? [])];
    const serversOf = (sessionId: string | undefined): Placed<RequestEnvironmentRead>[] =>
      (sessionId === undefined ? [] : (seed?.servers?.[sessionId] ?? [])).map((one) => ({
        name: `server:${one.name}`,
        variables: [{ name: "base", secret: false, value: one.base }],
        place: "server",
        hides: false,
      }));
    const read = (sessionId?: string): RequestsAnswer<RequestsRead> => ({
      ok: true,
      value: {
        collections: placed(collections, seed?.repository?.collections ?? []),
        environments: [...placed(environments, seed?.repository?.environments ?? []), ...serversOf(sessionId)],
      },
    });
    let file: RequestEnvironmentsFile = { version: 1, environment: [] };
    return {
      readRequests: async (sessionId) => read(sessionId),
      saveRequestCollection: async (collection, sessionId) => {
        if (collection.name.trim() === "") return refusal("fleet.request_collection_refused", "A collection needs a name.");
        collections = collections.some((one) => one.id === collection.id) ? collections.map((one) => (one.id === collection.id ? collection : one)) : [...collections, collection];
        return read(sessionId);
      },
      saveRequestEnvironment: async (environment, sessionId) => {
        if (environment.name.trim() === "") return refusal("fleet.request_environment_refused", "An environment needs a name.");
        if (environment.name.startsWith("server:")) return refusal("fleet.request_environment_refused", "server: names a Job's server.");
        const before = environments.find((one) => one.name === environment.name);
        // A secret is set where a value came with it, or where one was set before; the value itself goes nowhere.
        const variables: VariableRead[] = environment.variables.map((one) =>
          one.secret ? { name: one.name, secret: true, set: one.value !== undefined || before?.variables.some((was) => was.name === one.name && was.secret && was.set) === true } : one,
        );
        const next = { name: environment.name, variables };
        environments = before === undefined ? [...environments, next] : environments.map((one) => (one === before ? next : one));
        return read(sessionId);
      },
      sendSavedRequest: async (send) => {
        const all = read(send.session_id);
        if (!all.ok) return all;
        const environment = send.environment === undefined ? undefined : all.value.environments.find((one) => one.name === send.environment);
        if (send.environment !== undefined && environment === undefined) return refusal("fleet.request_refused", `There is no environment named ${send.environment}.`);
        let filled = true;
        const fill = (text: string) =>
          text.replace(NAME, (whole, name: string) => {
            const one = environment?.variables.find((variable) => variable.name === name);
            if (one === undefined || (one.secret && !one.set)) {
              filled = false;
              return whole;
            }
            // A secret is filled in Fleet; here it only counts as filled.
            return one.secret ? whole : one.value;
          });
        const url = fill(send.url);
        const collection = send.collection === undefined ? undefined : all.value.collections.find((one) => one.id === send.collection);
        headersWith(collection?.headers ?? [], send.headers).forEach((one) => fill(one.value));
        if (send.body !== undefined) fill(send.body);
        if (!/^https?:\/\/\S+/.test(url)) return refusal("fleet.request_refused", `${url} is not an http or https address.`);
        const answer = seed?.answer?.(send, url, filled) ?? OK;
        return { ok: true, value: { ...answer, url } };
      },
      // Today's members, which the panel no longer calls.
      sendRequest: async (send) => ({ ok: true, value: { ...OK, url: send.url } }),
      listRequestHosts: async () => ({ ok: true, value: { hosts: [] } }),
      setRequestCookie: async () => ({ ok: true, value: { hosts: [] } }),
      forgetRequestCookie: async () => ({ ok: true, value: { hosts: [] } }),
      listRequestEnvironments: async () => ({ ok: true, value: { saved: { what: "read", file }, servers: [] } }),
      saveRequestEnvironments: async (next) => {
        file = next;
        return { ok: true, value: { what: "read", file } };
      },
    };
  },
};
