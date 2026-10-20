// Requests' member, answered from memory for the life of the window: the hosts holding a kept header, the saved environments and
// each Job's servers, from `draft.requests` where a scenario gives them. A send resolves `{{base}}` and is refused as Fleet
// refuses it; it answers 200 unless the scenario's own `send` says otherwise. **No kept value is kept here**, only its host and header name.

import { BASE_PLACEHOLDER } from "@armada/protocol";
import type { RequestEnvironment, RequestEnvironmentsFile, RequestHost, RequestSent, SendRequest, WireError } from "@armada/protocol";

import type { RequestsAnswer, RequestsApi } from "../../../../shared/api/requests";
import type { Slice } from "../fake-context";

/** What a scenario starts Requests with, and how a send is answered. Mock only. */
export type RequestsSeed = {
  hosts?: readonly RequestHost[];
  /** The saved file's environments, or why it did not read. */
  environments?: readonly RequestEnvironment[] | { notRead: string };
  /** The servers each Job runs, by its id, named `server:<name>`. */
  servers?: Readonly<Record<string, readonly RequestEnvironment[]>>;
  /** The answer to a send, given the address resolved and the host and header that went with it. Absent answers 200. */
  answer?: (send: SendRequest, url: string, kept: RequestHost | undefined) => Omit<RequestSent, "url" | "cookie_host" | "kept_header">;
};

const refusal = (code: string, message: string): RequestsAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const hostOf = (url: string): string | undefined => {
  try {
    return new URL(url).hostname;
  } catch {
    return undefined;
  }
};

const OK: Omit<RequestSent, "url" | "cookie_host" | "kept_header"> = { status: 200, ms: 42, headers: [{ name: "content-type", value: "application/json" }], body: '{"ok":true}', truncated: false };

export const requests: Slice<RequestsApi, Record<string, never>> = {
  name: "requests",
  state: {},
  api: (scenario) => {
    const seed = scenario.draft?.requests;
    const hosts = new Map((seed?.hosts ?? []).map((one) => [one.host, one.header] as const));
    const notRead = seed?.environments !== undefined && "notRead" in seed.environments ? seed.environments.notRead : undefined;
    let file: RequestEnvironmentsFile = { version: 1, environment: Array.isArray(seed?.environments) ? [...seed.environments] : [] };
    const listed = () => ({ hosts: [...hosts].map(([host, header]) => ({ host, header })).sort((a, b) => a.host.localeCompare(b.host)) });
    const serversOf = (sessionId: string | undefined) => (sessionId === undefined ? [] : [...(seed?.servers?.[sessionId] ?? [])]);
    return {
      sendRequest: async (send) => {
        const environments = [...file.environment, ...serversOf(send.session_id)];
        const base = send.environment === undefined ? undefined : environments.find((one) => one.name === send.environment)?.base;
        if (send.environment !== undefined && base === undefined) return refusal("fleet.request_refused", `There is no environment named ${send.environment}.`);
        if (send.url.includes(BASE_PLACEHOLDER) && base === undefined) return refusal("fleet.request_refused", "The address holds {{base}} and no environment was picked.");
        const url = base === undefined ? send.url : send.url.replaceAll(BASE_PLACEHOLDER, base);
        const host = hostOf(url);
        if (host === undefined || !/^https?:/.test(url)) return refusal("fleet.request_refused", "The address is not an http or https address.");
        // As Fleet does: the kept header goes unless the request carries one of that name.
        const header = hosts.get(host);
        const kept = header === undefined || send.headers.some((one) => one.name.trim().toLowerCase() === header.toLowerCase()) ? undefined : { host, header };
        const answer = seed?.answer?.(send, url, kept) ?? OK;
        return { ok: true, value: { ...answer, url, ...(kept === undefined ? {} : { cookie_host: kept.host, kept_header: kept.header }) } };
      },
      listRequestHosts: async () => ({ ok: true, value: listed() }),
      setRequestCookie: async (host, header) => {
        if (!/^[A-Za-z0-9.-]+$/.test(host)) return refusal("fleet.request_cookie_refused", "A host is letters, digits, dots and hyphens.");
        if (!/^[A-Za-z0-9!#$%&'*+.^_`|~-]+$/.test(header)) return refusal("fleet.request_cookie_refused", "A header name is letters, digits and hyphens.");
        hosts.set(host, header);
        return { ok: true, value: listed() };
      },
      forgetRequestCookie: async (host) => {
        hosts.delete(host);
        return { ok: true, value: listed() };
      },
      listRequestEnvironments: async (sessionId) => ({
        ok: true,
        value: { saved: notRead === undefined ? { what: "read", file } : { what: "not_read", why: notRead }, servers: serversOf(sessionId) },
      }),
      saveRequestEnvironments: async (next) => {
        const names = new Set<string>();
        for (const [at, one] of next.environment.entries()) {
          if (one.name.trim() === "") return refusal("fleet.request_environments_refused", `environment[${at}].name is blank.`);
          if (names.has(one.name)) return refusal("fleet.request_environments_refused", `environment[${at}].name is used twice.`);
          if (!/^https?:\/\/\S+[^/]$/.test(one.base)) return refusal("fleet.request_environments_refused", `environment[${at}].base is not an http or https address without a trailing slash.`);
          names.add(one.name);
        }
        file = { version: next.version, environment: next.environment.map(({ name, base }) => ({ name, base })) };
        return { ok: true, value: { what: "read", file } };
      },
    };
  },
};
