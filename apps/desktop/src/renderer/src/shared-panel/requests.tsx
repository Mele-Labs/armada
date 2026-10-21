// Requests in the side panel: collections of saved requests and their shared headers, environments of variables that fill
// `{{name}}`, the editor that sends one through Fleet with its answer, and the Job's own requests to open again. **A secret is
// typed here and sent once**: Fleet answers only that it is set, so nothing here can draw it. The members it calls are proposed
// and only the mock serves them, `requests-model.ts`.

import { useEffect, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { ChevronDown, ChevronRight, FolderGit2, SquareAsterisk } from "lucide-react";
import { Alert, Button, Checkbox, Input, RequestMark, Select, Stamp, Textarea, Tooltip, requestTimeOf } from "@armada/components";
import type { RequestHeader } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { RequestsAnswer } from "../../../shared/api/requests";
import { useSessions } from "../sessions-draft";
import { HolderLink, useBack } from "./panels";
import { useReading, usePanelReads } from "./reads";
import { SidePanel } from "./side";
import { closePanel } from "./store";
import type { PanelItem, RequestStart } from "./store";
import { METHODS, bodyShown, filledOf, headersOf, headersText, historyOf, namesIn, startOf } from "./requests-model";
import type {
  Placed,
  ProposedRequestsApi,
  ProposedSent,
  RequestCollection,
  RequestEnvironmentRead,
  RequestsRead,
  RequestedRow,
  SavedRequest,
  VariableWrite,
} from "./requests-model";

type Item = Extract<PanelItem, { kind: "requests" }>;

/** What the Collections tab has open: a collection's headers, a saved request, or a request in no collection yet. */
type Open = { kind: "collection"; id: string } | { kind: "request"; collection: string; request: string } | { kind: "draft"; start: RequestStart | undefined; at: number };

const openOf = (item: Item): Open | undefined =>
  item.saved !== undefined ? { kind: "request", ...item.saved } : item.start !== undefined ? { kind: "draft", start: item.start, at: 0 } : undefined;

const said = <T,>(answer: RequestsAnswer<T> | undefined): string | undefined =>
  answer === undefined ? "Bridge could not reach Fleet" : answer.ok ? undefined : refusalWords(answer.outcome);

export function RequestsPanel({ item }: { item: Item }) {
  const api = usePanelReads()?.proposedRequests;
  const back = useBack();
  const session = useSessions().find((one) => one.id === item.sessionId);
  const [tab, setTab] = useState("collections");
  const read = useReading(async () => api?.readRequests(item.sessionId), `requests:${item.sessionId ?? ""}`);
  const [latest, setLatest] = useState<RequestsRead | undefined>(undefined);
  const now = latest ?? (read.out || read.value?.ok !== true ? undefined : read.value.value);
  const [open, setOpen] = useState<Open | undefined>(() => openOf(item));
  const [environment, setEnvironment] = useState(item.start?.environment ?? "");
  const history = historyOf(session);
  const tabs = [{ id: "collections", label: "Collections" }, { id: "environments", label: "Environments" }, ...(item.sessionId === undefined ? [] : [{ id: "history", label: "History" }])];
  const openFromHistory = (row: RequestedRow) => {
    setOpen({ kind: "draft", start: startOf(row, now?.environments), at: Date.now() });
    if (row.environment !== undefined) setEnvironment(row.environment);
    setTab("collections");
  };
  const heading = open?.kind === "request" ? now?.collections.find((one) => one.id === open.collection)?.requests.find((one) => one.id === open.request)?.name : undefined;
  return (
    <SidePanel
      title="Requests"
      heading={heading ?? (open?.kind === "draft" ? "New request" : "Collections")}
      {...(session === undefined ? {} : { meta: <HolderLink id={session.id} title={session.title ?? session.id} /> })}
      tabs={tabs}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {api === undefined ? (
        <Alert tone="caution">Fleet does not serve Requests here.</Alert>
      ) : !read.out && read.value !== undefined && !read.value.ok ? (
        <Alert tone="escalated">{said(read.value)}</Alert>
      ) : now === undefined ? null : tab === "collections" ? (
        <Collections api={api} read={now} sessionId={item.sessionId} open={open} onOpen={setOpen} onRead={setLatest} environment={environment} onEnvironment={setEnvironment} />
      ) : tab === "environments" ? (
        <Environments api={api} read={now} sessionId={item.sessionId} onRead={setLatest} />
      ) : (
        <History rows={history} onOpen={openFromHistory} />
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

/** Where a collection or environment is kept, beside its name: the repository's mark, or the words for the person's own hiding it. */
function PlaceMark({ placed }: { placed: { place: string; hides: boolean } }) {
  if (placed.hides) return <span className="side-requests__hides">Hides the repository's</span>;
  if (placed.place !== "repository") return <span />;
  return (
    <Tooltip label="The repository's">
      <span className="side-requests__mark" role="img" aria-label="The repository's">
        <FolderGit2 size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

type CollectionsProps = {
  api: ProposedRequestsApi;
  read: RequestsRead;
  sessionId: string | undefined;
  open: Open | undefined;
  onOpen: (open: Open) => void;
  onRead: (read: RequestsRead) => void;
  environment: string;
  onEnvironment: (name: string) => void;
};

function Collections(props: CollectionsProps) {
  const { read, open, onOpen } = props;
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const step = (depth: number) => ({ "--depth": depth }) as CSSProperties;
  const collection = open?.kind === "collection" ? read.collections.find((one) => one.id === open.id) : undefined;
  const inCollection = open?.kind === "request" ? read.collections.find((one) => one.id === open.collection) : undefined;
  const saved = open?.kind === "request" ? inCollection?.requests.find((one) => one.id === open.request) : undefined;
  return (
    <div className="side-request">
      <div className="armada-pr-tree side-requests__tree">
        <div className="armada-pr-tree__head">
          <span />
          <Button variant="ghost" size="sm" onClick={() => onOpen({ kind: "draft", start: undefined, at: Date.now() })}>
            New request
          </Button>
        </div>
        {read.collections.length === 0 ? (
          <p className="side-request__none">No collections in this repository.</p>
        ) : (
          <ul className="armada-pr-list armada-pr-tree__list" aria-label="Collections">
            {read.collections.flatMap((one) => {
              const shut = folded.has(one.id);
              const Chevron = shut ? ChevronRight : ChevronDown;
              return [
                <li key={one.id} className="armada-pr-tree__row" style={step(0)}>
                  <button
                    type="button"
                    className="armada-pr-tree__item"
                    data-kind="dir"
                    data-on={open?.kind === "collection" && open.id === one.id ? "" : undefined}
                    aria-expanded={!shut}
                    aria-label={one.name}
                    onClick={() => {
                      const selected = open?.kind === "collection" && open.id === one.id;
                      setFolded((was) => {
                        const next = new Set(was);
                        if (selected && !shut) next.add(one.id);
                        else next.delete(one.id);
                        return next;
                      });
                      onOpen({ kind: "collection", id: one.id });
                    }}
                  >
                    <Chevron className="armada-pr-tree__lead" size={16} strokeWidth={2} aria-hidden />
                    <span className="armada-pr-tree__name">{one.name}</span>
                    <PlaceMark placed={one} />
                  </button>
                </li>,
                ...(shut
                  ? []
                  : one.requests.map((request) => (
                      <li key={`${one.id}/${request.id}`} className="armada-pr-tree__row" style={step(1)}>
                        <button
                          type="button"
                          className="armada-pr-tree__item"
                          data-kind="file"
                          data-nested
                          data-on={open?.kind === "request" && open.collection === one.id && open.request === request.id ? "" : undefined}
                          aria-label={`${request.method} ${request.name}`}
                          onClick={() => onOpen({ kind: "request", collection: one.id, request: request.id })}
                        >
                          <code className="armada-requested__method">{request.method}</code>
                          <span className="armada-pr-tree__name">{request.name}</span>
                          <span />
                        </button>
                      </li>
                    ))),
              ];
            })}
          </ul>
        )}
      </div>
      {collection !== undefined ? (
        <CollectionHeaders key={collection.id} {...props} collection={collection} />
      ) : open?.kind === "request" && saved !== undefined && inCollection !== undefined ? (
        <Editor key={`${inCollection.id}/${saved.id}`} {...props} collection={inCollection} saved={saved} start={undefined} />
      ) : open?.kind === "draft" ? (
        <Editor key={`draft:${open.at}`} {...props} collection={undefined} saved={undefined} start={open.start} />
      ) : null}
    </div>
  );
}

function CollectionHeaders({ api, sessionId, onRead, collection }: CollectionsProps & { collection: Placed<RequestCollection> }) {
  const [text, setText] = useState(headersText(collection.headers));
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const save = async () => {
    const parsed = headersOf(text);
    if ("bad" in parsed) return setRefused(`${parsed.bad} is not Name: value`);
    const { place: _place, hides: _hides, ...own } = collection;
    const answer = await api.saveRequestCollection({ ...own, headers: parsed.headers }, sessionId).catch(() => undefined);
    setRefused(said(answer));
    if (answer?.ok === true) onRead(answer.value);
  };
  return (
    <form
      className="side-request__form"
      aria-label={`Collection ${collection.name}`}
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <Facts label="Collection" rows={[["Collection", collection.name], ["Kept", collection.place === "repository" ? "In the repository" : collection.hides ? "Yours. Hides the repository's" : "Yours"]]} />
      <Textarea
        label="Collection headers"
        keep={`requests:collection:${collection.id}:headers`}
        className="mono"
        rows={3}
        spellCheck={false}
        placeholder="Authorization: Bearer {{token}}"
        value={text}
        onChange={(event) => setText(event.currentTarget.value)}
      />
      <p className="side-request__none">{`Every request in ${collection.name} sends these.`}</p>
      <div className="side-request__acts">
        <Button type="submit" variant="primary">
          Save
        </Button>
      </div>
      {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
    </form>
  );
}

type Draft = { name: string; collection: string; method: string; url: string; headers: string; body: string };

const draftOf = (saved: SavedRequest | undefined, start: RequestStart | undefined): Draft => {
  const from = saved ?? start;
  return { name: saved?.name ?? "", collection: "", method: from?.method.toUpperCase() ?? "GET", url: from?.url ?? "", headers: headersText(from?.headers ?? []), body: from?.body ?? "" };
};

/** A plain address slug for a request saved under a new name. */
const idOf = (name: string): string => name.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "request";

/** What a send came to: Fleet's answer, or its refusal in words. */
type Sent = { answer: ProposedSent } | { said: string };

function Editor({ api, read, sessionId, onOpen, onRead, environment, onEnvironment, collection, saved, start }: CollectionsProps & { collection: Placed<RequestCollection> | undefined; saved: SavedRequest | undefined; start: RequestStart | undefined }) {
  const [draft, setDraft] = useState<Draft>(() => draftOf(saved, start));
  const [sending, setSending] = useState(false);
  const [sent, setSent] = useState<Sent | undefined>(undefined);
  const [bad, setBad] = useState<string | undefined>(undefined);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  // A thread row's address is filled; once the environments are read it reads `{{base}}` again, where nobody has typed in it yet.
  const seeded = useRef(false);
  useEffect(() => {
    if (seeded.current || start === undefined) return;
    seeded.current = true;
    const again = startOf({ ...start, headers: [...start.headers] }, read.environments).url;
    setDraft((now) => (now.url === start.url ? { ...now, url: again } : now));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const set = (patch: Partial<Draft>) => setDraft((now) => ({ ...now, ...patch }));
  const picked = read.environments.find((one) => one.name === environment);
  const scope = `requests:${sessionId ?? "none"}:${collection?.id ?? "draft"}:${saved?.id ?? "new"}`;
  const filled = filledOf(draft.url, picked);
  const holds = filled.text !== draft.url || filled.missing.length > 0;

  const parsed = (): RequestHeader[] | undefined => {
    const out = headersOf(draft.headers);
    if ("bad" in out) {
      setBad(out.bad);
      return undefined;
    }
    setBad(undefined);
    return out.headers;
  };

  const send = async () => {
    const headers = parsed();
    if (headers === undefined) return;
    setSending(true);
    const answer = await api
      .sendSavedRequest({
        ...(sessionId === undefined ? {} : { session_id: sessionId }),
        method: draft.method,
        url: draft.url.trim(),
        ...(environment === "" ? {} : { environment }),
        ...(collection === undefined ? {} : { collection: collection.id }),
        headers,
        ...(draft.body === "" ? {} : { body: draft.body }),
      })
      .catch(() => undefined);
    setSending(false);
    setSent(answer?.ok === true ? { answer: answer.value } : { said: said(answer) ?? "" });
  };

  const save = async () => {
    const headers = parsed();
    if (headers === undefined) return;
    const into = collection ?? read.collections.find((one) => one.id === draft.collection);
    if (into === undefined) return;
    const id = saved?.id ?? idOf(draft.name);
    const request: SavedRequest = { id, name: saved?.name ?? draft.name.trim(), method: draft.method, url: draft.url.trim(), headers, ...(draft.body === "" ? {} : { body: draft.body }) };
    const { place: _place, hides: _hides, ...kept } = into;
    const requests = kept.requests.some((one) => one.id === id) ? kept.requests.map((one) => (one.id === id ? request : one)) : [...kept.requests, request];
    const answer = await api.saveRequestCollection({ ...kept, requests }, sessionId).catch(() => undefined);
    setRefused(said(answer));
    if (answer?.ok !== true) return;
    onRead(answer.value);
    onOpen({ kind: "request", collection: into.id, request: id });
  };

  return (
    <div className="side-request">
      <form
        className="side-request__form"
        aria-label="Request"
        onSubmit={(event) => {
          event.preventDefault();
          void send();
        }}
      >
        {saved !== undefined ? null : (
          <div className="side-request__line" data-even>
            <Input label="Name" value={draft.name} spellCheck={false} onChange={(event) => set({ name: event.currentTarget.value })} />
            <Select label="Collection" value={draft.collection} onChange={(event) => set({ collection: event.currentTarget.value })}>
              <option value="">None</option>
              {read.collections.map((one) => (
                <option key={one.id} value={one.id}>
                  {one.name}
                </option>
              ))}
            </Select>
          </div>
        )}
        <div className="side-request__line">
          <Select label="Method" value={draft.method} onChange={(event) => set({ method: event.currentTarget.value })}>
            {METHODS.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
          <Input label="Address" mono value={draft.url} placeholder="{{base}}/path" spellCheck={false} onChange={(event) => set({ url: event.currentTarget.value })} />
        </div>
        {!holds ? null : (
          <p className="side-request__resolved mono" aria-label="Resolved address">
            {picked === undefined ? `${namesIn(draft.url).map((one) => `{{${one}}}`).join(", ")} needs an environment` : filled.missing.length > 0 ? `${filled.missing.map((one) => `{{${one}}}`).join(", ")} is not in ${picked.name}` : filled.text}
          </p>
        )}
        {collection === undefined || collection.headers.length === 0 ? null : <Facts label={`Headers from ${collection.name}`} rows={collection.headers.map((one) => [one.name, one.value, true] as const)} />}
        <Textarea
          label="Headers"
          keep={`${scope}:headers`}
          className="mono"
          rows={3}
          spellCheck={false}
          placeholder="Content-Type: application/json"
          value={draft.headers}
          invalid={bad !== undefined}
          {...(bad === undefined ? {} : { message: `${bad} is not Name: value` })}
          onChange={(event) => set({ headers: event.currentTarget.value })}
        />
        <Textarea label="Body" keep={`${scope}:body`} className="mono" rows={5} spellCheck={false} value={draft.body} onChange={(event) => set({ body: event.currentTarget.value })} />
        <div className="side-request__line side-request__send">
          <Select label="Environment" value={environment} onChange={(event) => onEnvironment(event.currentTarget.value)}>
            <option value="">None</option>
            {read.environments.map((one) => (
              <option key={one.name} value={one.name}>
                {one.name}
              </option>
            ))}
          </Select>
          <div className="side-request__acts">
            <Button variant="ghost" disabled={saved === undefined && (draft.name.trim() === "" || draft.collection === "")} onClick={() => void save()}>
              Save
            </Button>
            <Button type="submit" variant="primary" disabled={sending || draft.url.trim() === ""}>
              {sending ? "Sending" : "Send"}
            </Button>
          </div>
        </div>
        {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
      </form>
      {sent === undefined ? null : "said" in sent ? <Alert tone="escalated">{sent.said}</Alert> : <Response sent={sent.answer} />}
    </div>
  );
}

function Response({ sent }: { sent: ProposedSent }) {
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
  if (rows.length === 0) return <p className="side-request__none">No requests in this Job.</p>;
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

function Environments({ api, read, sessionId, onRead }: { api: ProposedRequestsApi; read: RequestsRead; sessionId: string | undefined; onRead: (read: RequestsRead) => void }) {
  const [adding, setAdding] = useState(false);
  const editable = read.environments.filter((one) => one.place !== "server");
  const servers = read.environments.filter((one) => one.place === "server");
  const saved = (next: RequestsRead) => {
    setAdding(false);
    onRead(next);
  };
  return (
    <div className="side-request">
      {editable.length === 0 && !adding ? <p className="side-request__none">No environments in this repository.</p> : null}
      {editable.map((one) => (
        <EnvironmentForm key={`${one.name}:${JSON.stringify(one.variables)}`} api={api} sessionId={sessionId} environment={one} onSaved={saved} />
      ))}
      {adding ? <EnvironmentForm api={api} sessionId={sessionId} environment={undefined} onSaved={saved} /> : null}
      {adding ? null : (
        <div className="side-request__acts">
          <Button variant="ghost" onClick={() => setAdding(true)}>
            Add environment
          </Button>
        </div>
      )}
      {servers.map((one) => (
        <Facts
          key={one.name}
          label={`Server ${one.name}`}
          rows={[["Environment", one.name, true] as const, ...one.variables.map((variable) => [variable.name, variable.secret ? "Set" : variable.value, true] as const)]}
        />
      ))}
    </div>
  );
}

/** A variable as the form holds it. A secret already set is drawn as set until Replace opens its field. */
type Line = { name: string; secret: boolean; value: string; set: boolean; replacing: boolean };

function EnvironmentForm({ api, sessionId, environment, onSaved }: { api: ProposedRequestsApi; sessionId: string | undefined; environment: Placed<RequestEnvironmentRead> | undefined; onSaved: (read: RequestsRead) => void }) {
  const [name, setName] = useState(environment?.name ?? "");
  const [lines, setLines] = useState<Line[]>(() =>
    environment === undefined
      ? [{ name: "", secret: false, value: "", set: false, replacing: false }]
      : environment.variables.map((one) => (one.secret ? { name: one.name, secret: true, value: "", set: one.set, replacing: false } : { name: one.name, secret: false, value: one.value, set: false, replacing: false })),
  );
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const edit = (at: number, patch: Partial<Line>) => setLines((now) => now.map((one, i) => (i === at ? { ...one, ...patch } : one)));
  const save = async () => {
    const variables: VariableWrite[] = lines
      .filter((one) => one.name.trim() !== "")
      .map((one) => (one.secret ? { name: one.name.trim(), secret: true, ...(one.value === "" ? {} : { value: one.value }) } : { name: one.name.trim(), secret: false, value: one.value }));
    const answer = await api.saveRequestEnvironment({ name: name.trim(), variables }, sessionId).catch(() => undefined);
    // A secret's value goes whether or not Fleet kept it: the field never holds one longer than a press.
    setLines((now) => now.map((one) => (one.secret ? { ...one, value: "" } : one)));
    setRefused(said(answer));
    if (answer?.ok === true) onSaved(answer.value);
  };
  return (
    <form
      className="side-request__form side-requests__environment"
      role="group"
      aria-label={environment?.name ?? "New environment"}
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      {environment === undefined ? (
        <Input label="Environment name" value={name} placeholder="test" spellCheck={false} onChange={(event) => setName(event.currentTarget.value)} />
      ) : (
        <div className="side-requests__head">
          <span className="side-requests__name mono">{environment.name}</span>
          <PlaceMark placed={environment} />
        </div>
      )}
      {lines.map((one, at) => (
        <div key={at} className="side-requests__variable">
          <Input aria-label={`Variable ${at + 1}`} mono value={one.name} placeholder="base" spellCheck={false} onChange={(event) => edit(at, { name: event.currentTarget.value })} />
          {one.secret && one.set && !one.replacing ? (
            <span className="side-requests__set">
              <Tooltip label="Set, in the Keychain">
                <span className="side-requests__mark" role="img" aria-label="Set, in the Keychain">
                  <SquareAsterisk size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              <span>Set</span>
              <Button variant="ghost" size="sm" aria-label={`Replace ${one.name}`} onClick={() => edit(at, { replacing: true })}>
                Replace
              </Button>
            </span>
          ) : (
            <Input aria-label={`Value ${at + 1}`} mono={!one.secret} type={one.secret ? "password" : "text"} autoComplete="off" spellCheck={false} value={one.value} onChange={(event) => edit(at, { value: event.currentTarget.value })} />
          )}
          <Checkbox checked={one.secret} disabled={one.set} onChange={(event) => edit(at, { secret: event.currentTarget.checked, value: "" })}>
            Secret
          </Checkbox>
        </div>
      ))}
      <div className="side-request__acts">
        <Button variant="ghost" onClick={() => setLines((now) => [...now, { name: "", secret: false, value: "", set: false, replacing: false }])}>
          Add variable
        </Button>
        <Button type="submit" variant="primary" disabled={name.trim() === ""}>
          Save
        </Button>
      </div>
      {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
    </form>
  );
}
