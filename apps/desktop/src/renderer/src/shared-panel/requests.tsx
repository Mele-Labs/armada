// Requests in the side panel: an HTTP request sent through Fleet, which attaches the owner's cookie for the host, and its
// answer; the Job's own requests to open again; the named base addresses; and the hosts holding a cookie. **A cookie's
// value is typed here and sent once**: Fleet never answers one, so nothing here can draw it.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Alert, Button, Input, RequestMark, Select, Stamp, Textarea, requestTimeOf } from "@armada/components";
import { BASE_PLACEHOLDER, SERVER_ENVIRONMENT_PREFIX } from "@armada/protocol";
import type { RequestEnvironment, RequestEnvironments, RequestSent } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { RequestsAnswer, RequestsApi } from "../../../shared/api/requests";
import { useSessions } from "../sessions-draft";
import { HolderLink, useBack } from "./panels";
import { useReading, usePanelReads } from "./reads";
import { SidePanel } from "./side";
import { closePanel } from "./store";
import type { PanelItem, RequestStart } from "./store";
import { METHODS, bodyShown, environmentsOf, headersOf, headersText, historyOf, resolvedOf, startOf } from "./requests-model";
import type { RequestedRow } from "./requests-model";

type Item = Extract<PanelItem, { kind: "requests" }>;

type Draft = { method: string; url: string; environment: string; headers: string; body: string };

const draftOf = (start: RequestStart | undefined): Draft => ({
  method: start?.method.toUpperCase() ?? "GET",
  url: start?.url ?? "",
  environment: start?.environment ?? "",
  headers: headersText(start?.headers ?? []),
  body: start?.body ?? "",
});

/** What a send came to: Fleet's answer, or its refusal in words. */
type Sent = { answer: RequestSent } | { said: string };

const said = <T,>(answer: RequestsAnswer<T>): string | undefined => (answer.ok ? undefined : refusalWords(answer.outcome));

export function RequestsPanel({ item }: { item: Item }) {
  const api = usePanelReads()?.requests;
  const back = useBack();
  const session = useSessions().find((one) => one.id === item.sessionId);
  const [tab, setTab] = useState("request");
  const [readAgain, setReadAgain] = useState(0);
  const read = useReading(async () => api?.listRequestEnvironments(item.sessionId), `requests:${item.sessionId ?? ""}`, readAgain);
  const environments = read.out || read.value?.ok !== true ? undefined : read.value.value;
  const offered = environmentsOf(environments);
  const [draft, setDraft] = useState<Draft>(() => draftOf(item.start));
  // The row's address is resolved; once the environments are read it is put back as `{{base}}`, where nobody has typed in it yet.
  const seeded = useRef(false);
  useEffect(() => {
    if (seeded.current || item.start === undefined || environments === undefined) return;
    seeded.current = true;
    const again = startOf(item.start, offered).url;
    setDraft((now) => (now.url === item.start!.url ? { ...now, url: again } : now));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [environments]);
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState<Sent | undefined>(undefined);
  const [badHeader, setBadHeader] = useState<string | undefined>(undefined);
  const history = historyOf(session);
  const scope = `requests:${item.sessionId ?? "none"}:${item.from ?? "new"}`;

  const send = async () => {
    if (api === undefined) return;
    const parsed = headersOf(draft.headers);
    if ("bad" in parsed) {
      setBadHeader(parsed.bad);
      return;
    }
    setBadHeader(undefined);
    setSending(true);
    const answer = await api
      .sendRequest({
        ...(item.sessionId === undefined ? {} : { session_id: item.sessionId }),
        method: draft.method,
        url: draft.url.trim(),
        ...(draft.environment === "" ? {} : { environment: draft.environment }),
        headers: parsed.headers,
        ...(draft.body === "" ? {} : { body: draft.body }),
      })
      .catch(() => undefined);
    setSending(false);
    setSent(answer === undefined ? { said: "Bridge could not reach Fleet" } : answer.ok ? { answer: answer.value } : { said: refusalWords(answer.outcome) });
  };

  const openFromHistory = (row: RequestedRow) => {
    setDraft(draftOf(startOf(row, offered)));
    setSent(undefined);
    setTab("request");
  };

  const set = (patch: Partial<Draft>) => setDraft((now) => ({ ...now, ...patch }));
  const resolved = resolvedOf(draft.url, draft.environment, offered);
  const tabs = [
    { id: "request", label: "Request" },
    ...(item.sessionId === undefined ? [] : [{ id: "history", label: "History" }]),
    { id: "environments", label: "Environments" },
    { id: "cookies", label: "Cookies" },
  ];
  return (
    <SidePanel
      title="Requests"
      heading={draft.url.trim() === "" ? "New request" : `${draft.method} ${draft.url.trim()}`}
      {...(session === undefined ? {} : { meta: <HolderLink id={session.id} title={session.title ?? session.id} /> })}
      tabs={tabs}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {api === undefined ? (
        <Alert tone="caution">Fleet does not serve Requests here.</Alert>
      ) : tab === "request" ? (
        <div className="side-request">
          <form
            className="side-request__form"
            aria-label="Request"
            onSubmit={(event) => {
              event.preventDefault();
              void send();
            }}
          >
            <div className="side-request__line">
              <Select label="Method" value={draft.method} onChange={(event) => set({ method: event.currentTarget.value })}>
                {METHODS.map((one) => (
                  <option key={one} value={one}>
                    {one}
                  </option>
                ))}
              </Select>
              <Select label="Environment" value={draft.environment} onChange={(event) => set({ environment: event.currentTarget.value })}>
                <option value="">None</option>
                {offered.map((one) => (
                  <option key={one.name} value={one.name}>
                    {one.name}
                  </option>
                ))}
              </Select>
            </div>
            <Input label="Address" mono value={draft.url} placeholder={`${BASE_PLACEHOLDER}/path`} spellCheck={false} onChange={(event) => set({ url: event.currentTarget.value })} />
            {draft.url.includes(BASE_PLACEHOLDER) ? (
              <p className="side-request__resolved mono" aria-label="Resolved address">
                {resolved ?? `${BASE_PLACEHOLDER} needs an environment`}
              </p>
            ) : null}
            <Textarea
              label="Headers"
              keep={`${scope}:headers`}
              className="mono"
              rows={3}
              spellCheck={false}
              placeholder="Content-Type: application/json"
              value={draft.headers}
              invalid={badHeader !== undefined}
              {...(badHeader === undefined ? {} : { message: `Not Name: value: ${badHeader}` })}
              onChange={(event) => set({ headers: event.currentTarget.value })}
            />
            <Textarea label="Body" keep={`${scope}:body`} className="mono" rows={5} spellCheck={false} value={draft.body} onChange={(event) => set({ body: event.currentTarget.value })} />
            <div className="side-request__acts">
              <Button type="submit" variant="primary" disabled={sending || draft.url.trim() === ""}>
                {sending ? "Sending" : "Send"}
              </Button>
            </div>
          </form>
          {sent === undefined ? null : "said" in sent ? <Alert tone="escalated">{sent.said}</Alert> : <Response sent={sent.answer} />}
        </div>
      ) : tab === "history" ? (
        <History rows={history} onOpen={openFromHistory} />
      ) : tab === "environments" ? (
        <Environments key={readAgain} read={read.out ? undefined : read.value} onSaved={() => setReadAgain((one) => one + 1)} save={api.saveRequestEnvironments} />
      ) : (
        <Cookies api={api} />
      )}
    </SidePanel>
  );
}

function Facts({ label, rows }: { label: string; rows: readonly (readonly [string, ReactNode, boolean?])[] }) {
  return (
    <dl className="side-facts" aria-label={label}>
      {rows.map(([term, value, mono]) => (
        <div key={term} className="side-facts__row">
          <dt>{term}</dt>
          <dd data-mono={mono || undefined}>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function Response({ sent }: { sent: RequestSent }) {
  const shown = bodyShown(sent.body);
  const status: ReactNode = (
    <span className="side-request__status">
      <RequestMark status={sent.status ?? null} error={sent.error} />
      {sent.status === undefined ? "No answer" : sent.status}
    </span>
  );
  const facts: (readonly [string, ReactNode, boolean?])[] = [
    ["Status", status],
    ["Time", requestTimeOf(sent.ms), true],
    ["Address", sent.url, true],
    ...(sent.cookie_host === undefined ? [] : [["Cookie", sent.cookie_host, true] as const]),
    ...(sent.error === undefined ? [] : [["Error", sent.error] as const]),
    ...(sent.note === undefined ? [] : [["Note", sent.note] as const]),
    ...(sent.truncated ? [["Body", "Cut at 256 KiB"] as const] : []),
  ];
  return (
    <section className="side-request__response" aria-label="Response">
      <Facts label="Answer" rows={facts} />
      {sent.headers.length === 0 ? null : (
        <details className="side-request__headers">
          <summary>Headers</summary>
          <dl className="side-facts" aria-label="Response headers">
            {sent.headers.map((one, at) => (
              <div key={`${at}:${one.name}`} className="side-facts__row">
                <dt>{one.name}</dt>
                <dd data-mono>{one.value}</dd>
              </div>
            ))}
          </dl>
        </details>
      )}
      {sent.body === "" ? null : (
        <pre className="side-log__lines mono" role="log" aria-label={shown.json ? "Body, JSON" : "Body"}>
          {shown.text}
        </pre>
      )}
    </section>
  );
}

function History({ rows, onOpen }: { rows: readonly RequestedRow[]; onOpen: (row: RequestedRow) => void }) {
  if (rows.length === 0) return <p className="side-request__none">No requests in this Job yet.</p>;
  return (
    <ul className="armada-pr-list armada-pr-rows" aria-label="History">
      {rows.map((row) => (
        <li key={row.id} className="armada-pr-row" data-press>
          <button type="button" className="armada-pr-row__button side-row" data-columns="request" aria-label={`${row.by === "owner" ? "You" : "Agent"}: ${row.status ?? "No answer"} ${row.method} ${row.url}`} onClick={() => onOpen(row)}>
            <RequestMark status={row.status} error={row.error} />
            <span className="armada-pr-row__name mono">{`${row.method} ${row.url}`}</span>
            <span className="armada-pr-row__figure">{row.by === "owner" ? "You" : "Agent"}</span>
            <Stamp still at={row.at} className="armada-pr-row__figure" />
          </button>
        </li>
      ))}
    </ul>
  );
}

function Environments({
  read,
  save,
  onSaved,
}: {
  read: RequestsAnswer<RequestEnvironments> | undefined;
  save: RequestsApi["saveRequestEnvironments"];
  onSaved: () => void;
}) {
  const [rows, setRows] = useState<RequestEnvironment[] | undefined>(undefined);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  if (read === undefined) return null;
  if (!read.ok) return <Alert tone="escalated">{said(read)}</Alert>;
  const { saved, servers } = read.value;
  const own = rows ?? (saved.what === "read" ? saved.file.environment : []);
  const edit = (at: number, patch: Partial<RequestEnvironment>) => setRows(own.map((one, i) => (i === at ? { ...one, ...patch } : one)));
  const store = async () => {
    setSaving(true);
    const answer = await save({ version: saved.what === "read" ? saved.file.version : 1, environment: own }).catch(() => undefined);
    setSaving(false);
    if (answer === undefined) setRefused("Bridge could not reach Fleet");
    else if (!answer.ok) setRefused(said(answer));
    else if (answer.value.what === "not_read") setRefused(answer.value.why);
    else onSaved();
  };
  return (
    <div className="side-request">
      {saved.what === "not_read" ? (
        <Alert tone="escalated" title="The environments file did not read">
          {saved.why}
        </Alert>
      ) : (
        <form
          className="side-request__form"
          aria-label="Saved environments"
          onSubmit={(event) => {
            event.preventDefault();
            void store();
          }}
        >
          {own.map((one, at) => (
            <div key={at} className="side-request__environment" role="group" aria-label={one.name === "" ? "New environment" : one.name}>
              <Input aria-label="Name" value={one.name} spellCheck={false} onChange={(event) => edit(at, { name: event.currentTarget.value })} />
              <Input aria-label="Base" mono value={one.base} placeholder="https://api.example.test" spellCheck={false} onChange={(event) => edit(at, { base: event.currentTarget.value })} />
              <Button variant="ghost" size="sm" onClick={() => setRows(own.filter((_, i) => i !== at))}>
                Remove
              </Button>
            </div>
          ))}
          <div className="side-request__acts">
            <Button variant="ghost" onClick={() => setRows([...own, { name: "", base: "" }])}>
              Add environment
            </Button>
            <Button type="submit" variant="primary" disabled={saving || rows === undefined}>
              Save
            </Button>
          </div>
          {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
        </form>
      )}
      {servers.length === 0 ? null : <Facts label="Servers of this Job" rows={servers.map((one) => [one.name.slice(SERVER_ENVIRONMENT_PREFIX.length), one.base, true] as const)} />}
    </div>
  );
}

function Cookies({ api }: { api: RequestsApi }) {
  const [again, setAgain] = useState(0);
  const read = useReading(async () => api.listRequestHosts(), "requests:hosts", again);
  const [hosts, setHosts] = useState<readonly string[] | undefined>(undefined);
  const [host, setHost] = useState("");
  const [cookie, setCookie] = useState("");
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const shown = hosts ?? (read.out || read.value?.ok !== true ? undefined : read.value.value.hosts);
  const took = (answer: RequestsAnswer<{ hosts: string[] }> | undefined): boolean => {
    if (answer === undefined) setRefused("Bridge could not reach Fleet");
    else if (!answer.ok) setRefused(said(answer));
    else {
      setRefused(undefined);
      setHosts(answer.value.hosts);
      return true;
    }
    return false;
  };
  const keep = async () => {
    const answer = await api.setRequestCookie(host.trim(), cookie).catch(() => undefined);
    // The value goes whether or not Fleet kept it: the box never holds one longer than a press.
    setCookie("");
    if (took(answer)) setHost("");
  };
  return (
    <div className="side-request">
      {!read.out && read.value !== undefined && !read.value.ok ? <Alert tone="escalated">{said(read.value)}</Alert> : null}
      {shown === undefined ? null : shown.length === 0 ? (
        <p className="side-request__none">No host holds a cookie.</p>
      ) : (
        <ul className="armada-pr-list armada-pr-rows" aria-label="Hosts holding a cookie">
          {shown.map((one) => (
            <li key={one} className="armada-pr-row" data-columns="link">
              <span className="armada-pr-row__name mono">{one}</span>
              <span />
              <Button variant="ghost" size="sm" aria-label={`Forget the cookie for ${one}`} onClick={() => void api.forgetRequestCookie(one).catch(() => undefined).then(took).then(() => setAgain((n) => n + 1))}>
                Forget
              </Button>
            </li>
          ))}
        </ul>
      )}
      <form
        className="side-request__form"
        aria-label="Add a cookie"
        onSubmit={(event) => {
          event.preventDefault();
          void keep();
        }}
      >
        <Input label="Host" mono value={host} placeholder="api.example.test" spellCheck={false} onChange={(event) => setHost(event.currentTarget.value)} />
        <Input label="Cookie" type="password" autoComplete="off" spellCheck={false} value={cookie} onChange={(event) => setCookie(event.currentTarget.value)} />
        <p className="side-request__none">Kept in the Keychain. Bridge never reads it back.</p>
        <div className="side-request__acts">
          <Button type="submit" variant="primary" disabled={host.trim() === "" || cookie === ""}>
            Keep cookie
          </Button>
        </div>
        {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
      </form>
    </div>
  );
}
