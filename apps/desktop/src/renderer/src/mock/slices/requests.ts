// Requests' members, answered from memory for the life of the window, in the shapes Fleet answers (`get_requests`,
// `save_request_collection`, `save_request_environment`, `send_request`). Each repository keeps the person's own and the
// repository's collections and environments, the person's winning by collection id and environment name, and each Job's
// servers as `server:<name>`. A file seeded as not reading lists nothing, and a save into the person's own is refused 409 while
// it will not. A send fills `{{name}}` and answers 200 unless the scenario's `answer` says otherwise. **No secret is kept
// here**, only that it is set.

import type {
  Placed,
  RequestCollection,
  RequestEnvironmentRead,
  Requests,
  RequestSent,
  RequestsNotRead,
  SendRequest,
  VariableRead,
  WireError,
} from "@armada/protocol";

import type { RequestsAnswer, RequestsApi } from "../../../../shared/api/requests";
import { headersWith } from "../../shared-panel/requests-model";
import type { Slice } from "../fake-context";

type Kept = { collections?: readonly RequestCollection[]; environments?: readonly RequestEnvironmentRead[] };

/** One repository's two files. A place in `notRead` lists nothing from that file. Mock only. */
export type RequestsRepositorySeed = { yours?: Kept; repository?: Kept; notRead?: readonly RequestsNotRead[] };

/** What a scenario starts Requests with, and how a send is answered. Mock only. */
export type RequestsSeed = RequestsRepositorySeed & {
  /** Other repositories by root. A read naming none of these, or naming none at all, reads the seed itself. */
  repositories?: Readonly<Record<string, RequestsRepositorySeed>>;
  /** The servers each Job runs, by its id, by name without `server:`. */
  servers?: Readonly<Record<string, readonly { name: string; base: string }[]>>;
  /** The answer to a send, given the address filled in and whether every `{{name}}` was. Absent answers 200. */
  answer?: (send: OwnSend, url: string, filled: boolean) => Omit<RequestSent, "url">;
};

/** A send as its own method and address: what a scenario's `answer` and a thread row are given. */
export type OwnSend = Extract<SendRequest, { method: string }>;

/** A saved request's send as the request it names, its headers under the send's and the send's body over its own. */
function ownOf(send: SendRequest, collection: RequestCollection | undefined): OwnSend | undefined {
  if (send.saved === undefined) return send;
  const { saved, ...rest } = send;
  const kept = collection?.requests.find((one) => one.id === saved.request || one.name === saved.request);
  if (kept === undefined) return undefined;
  const body = send.body ?? kept.body;
  return { ...rest, method: kept.method, url: kept.url, headers: headersWith(kept.headers, send.headers), ...(body === undefined ? {} : { body }) };
}

const refusal = (code: string, message: string): RequestsAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const OK: Omit<RequestSent, "url"> = { status: 200, ms: 42, headers: [{ name: "content-type", value: "application/json" }], body: '{"ok":true}', truncated: false };

const NAME = /\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}/g;

/** The person's own over the repository's: a key in both is the person's, marked as hiding the other. */
function placed<T>(yours: readonly T[], repository: readonly T[], key: (one: T) => string): Placed<T>[] {
  const mine = new Set(yours.map(key));
  const theirs = new Set(repository.map(key));
  return [
    ...yours.map((one) => ({ ...one, place: "yours" as const, hides: theirs.has(key(one)) })),
    ...repository.filter((one) => !mine.has(key(one))).map((one) => ({ ...one, place: "repository" as const, hides: false })),
  ];
}

/** One repository as the window holds it: the person's own as saved, the repository's as seeded. */
type Held = { collections: RequestCollection[]; environments: RequestEnvironmentRead[]; seed: RequestsRepositorySeed };

export const requests: Slice<RequestsApi, Record<string, never>> = {
  name: "requests",
  state: {},
  api: (scenario) => {
    const seed = scenario.draft?.requests;
    const first = scenario.state?.holds.repositories?.[0]?.root ?? "/Users/user/armada";
    const held = new Map<string, Held>();
    const heldOf = (root: string): Held => {
      const known = held.get(root);
      if (known !== undefined) return known;
      const own = seed?.repositories?.[root] ?? seed ?? {};
      const made = { collections: [...(own.yours?.collections ?? [])], environments: [...(own.yours?.environments ?? [])], seed: own };
      held.set(root, made);
      return made;
    };
    const unread = (one: Held, place: RequestsNotRead["place"]) => one.seed.notRead?.some((not) => not.place === place) === true;
    const serversOf = (sessionId: string | undefined): Placed<RequestEnvironmentRead>[] =>
      (sessionId === undefined ? [] : (seed?.servers?.[sessionId] ?? [])).map((one) => ({
        name: `server:${one.name}`,
        variables: [{ name: "base", secret: false, value: one.base }],
        place: "server",
        hides: false,
      }));
    const read = (sessionId?: string, repository?: string): Requests => {
      const root = repository ?? first;
      const one = heldOf(root);
      const yours = unread(one, "yours") ? { collections: [], environments: [] } : one;
      const theirs = unread(one, "repository") ? {} : (one.seed.repository ?? {});
      return {
        repository: root,
        place: "this_mac",
        collections: placed(yours.collections, theirs.collections ?? [], (it) => it.id),
        environments: [...placed(yours.environments, theirs.environments ?? [], (it) => it.name), ...serversOf(sessionId)],
        not_read: [...(one.seed.notRead ?? [])],
      };
    };
    const refusedUnread = (repository: string | undefined): RequestsAnswer<never> | undefined => {
      const one = heldOf(repository ?? first);
      const why = one.seed.notRead?.find((not) => not.place === "yours")?.why;
      return why === undefined ? undefined : refusal("fleet.requests_unread", `Your requests file would not read: ${why}`);
    };
    return {
      readRequests: async (sessionId, repository) => ({ ok: true, value: read(sessionId, repository) }),
      saveRequestCollection: async (collection, sessionId, repository) => {
        const unreadAnswer = refusedUnread(repository);
        if (unreadAnswer !== undefined) return unreadAnswer;
        if (collection.name.trim() === "") return refusal("fleet.request_collection_refused", "collection.name is blank.");
        const one = heldOf(repository ?? first);
        one.collections = one.collections.some((it) => it.id === collection.id) ? one.collections.map((it) => (it.id === collection.id ? collection : it)) : [...one.collections, collection];
        return { ok: true, value: read(sessionId, repository) };
      },
      saveRequestEnvironment: async (environment, sessionId, repository) => {
        const unreadAnswer = refusedUnread(repository);
        if (unreadAnswer !== undefined) return unreadAnswer;
        if (environment.name.trim() === "") return refusal("fleet.request_environment_refused", "environment.name is blank.");
        if (environment.name.startsWith("server:")) return refusal("fleet.request_environment_refused", "environment.name starts server:, which names a Job's server.");
        const one = heldOf(repository ?? first);
        const before = one.environments.find((it) => it.name === environment.name);
        // A secret is set where a value came with it, or where one was set before; the value itself goes nowhere.
        const variables: VariableRead[] = environment.variables.map((it) =>
          it.secret ? { name: it.name, secret: true, set: it.value !== undefined || before?.variables.some((was) => was.name === it.name && was.secret && was.set) === true } : it,
        );
        const next = { name: environment.name, variables };
        one.environments = before === undefined ? [...one.environments, next] : one.environments.map((it) => (it === before ? next : it));
        return { ok: true, value: read(sessionId, repository) };
      },
      sendRequest: async (send) => {
        const all = read(send.session_id, send.repository);
        const environment = send.environment === undefined ? undefined : all.environments.find((one) => one.name === send.environment);
        if (send.environment !== undefined && environment === undefined) return refusal("fleet.request_refused", `There is no environment named ${send.environment}.`);
        const named = send.saved?.collection ?? send.collection;
        const collection = named === undefined ? undefined : all.collections.find((one) => one.id === named || one.name === named);
        const own = ownOf(send, collection);
        if (own === undefined) return refusal("fleet.request_refused", `There is no saved request ${send.saved?.request ?? ""} in ${named ?? ""}.`);
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
        const url = fill(own.url);
        headersWith(collection?.headers ?? [], own.headers).forEach((one) => fill(one.value));
        if (own.body !== undefined) fill(own.body);
        if (!/^https?:\/\/\S+/.test(url)) return refusal("fleet.request_refused", `${url} is not an http or https address.`);
        const answer = seed?.answer?.(own, url, filled) ?? OK;
        return { ok: true, value: { ...answer, url } };
      },
    };
  },
};
