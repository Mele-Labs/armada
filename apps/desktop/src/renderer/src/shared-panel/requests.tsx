// Requests in the side panel: collections of saved requests and their shared headers, environments of variables that fill
// `{{name}}`, the editor that sends one through Fleet with its answer, and the Job's own requests to open again. **A secret is
// typed here and sent once**: Fleet answers only that it is set, so nothing here can draw it. A file that would not read is
// named with Fleet's why, and nothing is saved over the person's own while it will not. The environment picked and each open
// request's edits and answer are the panel's, so a tab switch keeps them and Save can sit on the heading's line.

import { useContext, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { ChevronDown, ChevronRight, FolderGit2, SquareAsterisk, Trash2, TriangleAlert } from "lucide-react";
import {
  Alert,
  Button,
  Checkbox,
  Input,
  RequestMark,
  Select,
  Separator,
  Stamp,
  TabPanel,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
  Tabs,
  Textarea,
  Tooltip,
  requestTimeOf,
} from "@armada/components";
import type { Placed, RequestCollection, RequestEnvironmentRead, RequestHeader, Requests, RequestSent, RequestsNotRead, SavedRequest, VariableWrite } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { repositoryLabel, useAtFloor, useRecentRepositories } from "@armada/shell";

import type { RequestsAnswer, RequestsApi } from "../../../shared/api/requests";
import { useSessions } from "../sessions-draft";
import { HolderLink, useBack } from "./panels";
import { useReading, usePanelReads } from "./reads";
import { InlinePanel, SidePanel } from "./side";
import { closePanel } from "./store";
import type { PanelItem, RequestStart } from "./store";
import { METHODS, bodyShown, filledOf, historyOf, repositoryToRead, startOf, unreadFileOf, yoursUnread } from "./requests-model";
import type { RequestedRow } from "./requests-model";

type Item = Extract<PanelItem, { kind: "requests" }>;

/** What the Request tab has open: a collection's headers, a saved request, or a request in no collection yet. */
type Open = { kind: "collection"; id: string } | { kind: "request"; collection: string; request: string } | { kind: "draft"; start: RequestStart | undefined; at: number };

const openOf = (item: Item): Open | undefined =>
  item.saved !== undefined ? { kind: "request", ...item.saved } : item.start !== undefined ? { kind: "draft", start: item.start, at: 0 } : undefined;

const keyOf = (open: Open): string =>
  open.kind === "collection" ? `collection:${open.id}` : open.kind === "request" ? `request:${open.collection}/${open.request}` : `draft:${open.at}`;

const said = <T,>(answer: RequestsAnswer<T> | undefined): string | undefined =>
  answer === undefined ? "Bridge could not reach Fleet" : answer.ok ? undefined : refusalWords(answer.outcome);

/** A header as the table holds it. `key` is the row's own, so removing one keeps each field under its row. */
type HeaderRow = { key: string; name: string; value: string };

let added = 0;
const blankRow = (): HeaderRow => ({ key: `added:${(added += 1)}`, name: "", value: "" });

/** The rows a list of headers starts as, one empty row where it has none. Keyed by place, so a render draws the same rows. */
const rowsOf = (headers: readonly RequestHeader[]): HeaderRow[] =>
  headers.length === 0 ? [{ key: "at:0", name: "", value: "" }] : headers.map((one, at) => ({ key: `at:${at}`, name: one.name, value: one.value }));

/** The rows as headers. A row with no name is not sent. */
const headersFrom = (rows: readonly HeaderRow[]): RequestHeader[] =>
  rows.filter((one) => one.name.trim() !== "").map((one) => ({ name: one.name.trim(), value: one.value.trim() }));

/** The first row whose name holds a space, which no header name can. */
const badRow = (rows: readonly HeaderRow[]): number => rows.findIndex((one) => /\s/.test(one.name.trim()));

type Draft = { name: string; collection: string; method: string; url: string; headers: HeaderRow[]; body: string };

const draftOf = (saved: SavedRequest | undefined, start: RequestStart | undefined, environments: readonly RequestEnvironmentRead[]): Draft => {
  // A thread row's address is filled; it reads `{{base}}` again where its environment's base starts it.
  const from = saved ?? (start === undefined ? undefined : startOf({ ...start, headers: [...start.headers] }, environments));
  return { name: saved?.name ?? "", collection: "", method: from?.method.toUpperCase() ?? "GET", url: from?.url ?? "", headers: rowsOf(from?.headers ?? []), body: from?.body ?? "" };
};

/** A plain address slug for a request saved under a new name. */
const idOf = (name: string): string => name.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "request";

/** What a send came to: Fleet's answer and whose shared headers went with it, or its refusal in words. */
type Sent = { answer: RequestSent; from: string | undefined } | { said: string };

const without = <T,>(record: Readonly<Record<string, T>>, key: string): Record<string, T> => {
  const { [key]: _gone, ...rest } = record;
  return rest;
};

export function RequestsPanel({ item }: { item: Item }) {
  const reads = usePanelReads();
  const api = reads?.requests;
  const back = useBack();
  const session = useSessions().find((one) => one.id === item.sessionId);
  const inline = useContext(InlinePanel);
  const atFloor = useAtFloor();
  const floor = atFloor || inline;
  const [tab, setTab] = useState("request");
  const [recent, remember] = useRecentRepositories();
  // With no Job open, the repository is picked here, starting where the Cockpit is narrowed to, as it stood at the open.
  const [places] = useState(() => (item.sessionId === undefined ? reads?.repositories?.() : undefined));
  const listed = places?.listed ?? [];
  const [repository, setRepository] = useState(() => (places === undefined ? undefined : repositoryToRead(listed.map((one) => one.root), places.filter, recent)));
  const read = useReading(async () => api?.readRequests(item.sessionId, repository), `requests:${item.sessionId ?? ""}:${repository ?? ""}`);
  const [latest, setLatest] = useState<Requests | undefined>(undefined);
  const now = latest ?? (read.out || read.value?.ok !== true ? undefined : read.value.value);
  const [open, setOpen] = useState<Open | undefined>(() => openOf(item));
  const [environment, setEnvironment] = useState(item.start?.environment ?? "");
  const [drafts, setDrafts] = useState<Readonly<Record<string, Draft>>>({});
  const [shared, setShared] = useState<Readonly<Record<string, HeaderRow[]>>>({});
  const [sends, setSends] = useState<Readonly<Record<string, Sent>>>({});
  const [refused, setRefused] = useState<{ key: string; said: string } | undefined>(undefined);
  const history = historyOf(session);
  const switchTo = (root: string) => {
    remember(root);
    setRepository(root);
    // Another repository's collections and environments: nothing open here belongs to them.
    setLatest(undefined);
    setOpen(undefined);
    setEnvironment("");
    setDrafts({});
    setShared({});
    setSends({});
    setRefused(undefined);
  };
  const tabs = [{ id: "request", label: "Request" }, { id: "environments", label: "Environments" }, ...(item.sessionId === undefined ? [] : [{ id: "history", label: "History" }])];
  const openFromHistory = (row: RequestedRow) => {
    setOpen({ kind: "draft", start: startOf(row, now?.environments), at: Date.now() });
    if (row.environment !== undefined) setEnvironment(row.environment);
    setTab("request");
  };
  const openFromTree = (next: Open) => {
    setOpen(next);
    setTab("request");
  };

  const key = open === undefined ? undefined : keyOf(open);
  const collection = open?.kind === "collection" ? now?.collections.find((one) => one.id === open.id) : undefined;
  const inCollection = open?.kind === "request" ? now?.collections.find((one) => one.id === open.collection) : undefined;
  const saved = open?.kind === "request" ? inCollection?.requests.find((one) => one.id === open.request) : undefined;
  const draft =
    now === undefined || key === undefined || (saved === undefined && open?.kind !== "draft")
      ? undefined
      : (drafts[key] ?? draftOf(saved, open?.kind === "draft" ? open.start : undefined, now.environments));
  const rows = collection === undefined ? undefined : (shared[collection.id] ?? rowsOf(collection.headers));
  const into = saved !== undefined ? inCollection : draft === undefined ? undefined : now?.collections.find((one) => one.id === draft.collection);
  const onDraft = (patch: Partial<Draft>) => {
    if (key === undefined || draft === undefined) return;
    setDrafts((was) => ({ ...was, [key]: { ...(was[key] ?? draft), ...patch } }));
  };

  const keep = async (next: RequestCollection): Promise<Requests | undefined> => {
    if (api === undefined || key === undefined) return undefined;
    const answer = await api.saveRequestCollection(next, item.sessionId, repository).catch(() => undefined);
    const words = said(answer);
    setRefused(words === undefined ? undefined : { key, said: words });
    if (answer?.ok !== true) return undefined;
    setLatest(answer.value);
    return answer.value;
  };
  const save = async () => {
    if (collection !== undefined && rows !== undefined) {
      const { place: _place, hides: _hides, ...own } = collection;
      if ((await keep({ ...own, headers: headersFrom(rows) })) !== undefined) setShared((was) => without(was, collection.id));
      return;
    }
    if (draft === undefined || into === undefined || key === undefined) return;
    const id = saved?.id ?? idOf(draft.name);
    const request: SavedRequest = { id, name: saved?.name ?? draft.name.trim(), method: draft.method, url: draft.url.trim(), headers: headersFrom(draft.headers), ...(draft.body === "" ? {} : { body: draft.body }) };
    const { place: _place, hides: _hides, ...kept } = into;
    const requests = kept.requests.some((one) => one.id === id) ? kept.requests.map((one) => (one.id === id ? request : one)) : [...kept.requests, request];
    if ((await keep({ ...kept, requests })) === undefined) return;
    const opened: Open = { kind: "request", collection: into.id, request: id };
    // What was saved is now the saved request, so its edits go; its answer moves with it.
    setDrafts((was) => without(was, key));
    setSends((was) => (was[key] === undefined ? was : { ...without(was, key), [keyOf(opened)]: was[key] }));
    setOpen(opened);
  };
  const savable =
    collection !== undefined && rows !== undefined
      ? badRow(rows) < 0
      : draft !== undefined && badRow(draft.headers) < 0 && (saved !== undefined || (draft.name.trim() !== "" && into !== undefined));

  // A save over the person's own file while it will not read is refused, so none is offered.
  const locked = yoursUnread(now) !== undefined;
  const heading = collection?.name ?? saved?.name ?? (open?.kind === "draft" ? "New request" : "No request open");
  const crumb = collection !== undefined ? undefined : into?.name;
  const editing = tab === "request" && (rows !== undefined || draft !== undefined);
  return (
    <SidePanel
      title="Requests"
      heading={heading}
      crumb={crumb}
      {...(!editing ? {} : {
        beside: (
          <Button variant="secondary" disabled={!savable || locked} onClick={() => void save()}>
            Save
          </Button>
        ),
      })}
      {...(session === undefined ? {} : { meta: <HolderLink id={session.id} title={session.title ?? session.id} /> })}
      {...(now === undefined && listed.length === 0 ? {} : {
        bar: (
          <>
            {listed.length === 0 ? null : (
              <Select label="Repository" value={repository ?? ""} onChange={(event) => switchTo(event.currentTarget.value)}>
                {listed.map((one) => (
                  <option key={one.root} value={one.root}>
                    {repositoryLabel(one, listed)}
                  </option>
                ))}
              </Select>
            )}
            {now === undefined ? null : (
              <Select label="Environment" value={environment} onChange={(event) => setEnvironment(event.currentTarget.value)}>
                <option value="">None</option>
                {now.environments.map((one) => (
                  <option key={one.name} value={one.name}>
                    {one.name}
                  </option>
                ))}
              </Select>
            )}
          </>
        ),
      })}
      tabs={tabs}
      tab={tab}
      onTab={setTab}
      back={back}
      onClose={closePanel}
      {...(now === undefined ? {} : { left: { title: "Collections", body: <CollectionTree read={now} open={open} onOpen={openFromTree} /> } })}
    >
      {api === undefined ? (
        <Alert tone="caution">Fleet does not serve Requests here.</Alert>
      ) : !read.out && read.value !== undefined && !read.value.ok ? (
        <Alert tone="escalated">{said(read.value)}</Alert>
      ) : now === undefined ? null : (
        <>
          {now.not_read.map((one) => (
            <Unread key={one.place} unread={one} />
          ))}
          {tab === "request" ? (
            <RequestTab tree={floor ? <CollectionTree read={now} open={open} onOpen={openFromTree} /> : undefined}>
              {collection !== undefined && rows !== undefined ? (
                <CollectionView collection={collection} rows={rows} onRows={(next) => setShared((was) => ({ ...was, [collection.id]: next }))} refused={refused?.key === key ? refused?.said : undefined} />
              ) : draft !== undefined && key !== undefined ? (
                <Editor
                  api={api}
                  read={now}
                  sessionId={item.sessionId}
                  repository={repository}
                  environment={environment}
                  draft={draft}
                  onDraft={onDraft}
                  collection={inCollection}
                  fresh={saved === undefined}
                  scope={`requests:${item.sessionId ?? repository ?? "none"}:${key}`}
                  sent={sends[key]}
                  onSent={(sent) => setSends((was) => ({ ...was, [key]: sent }))}
                  onEditCollection={(id) => setOpen({ kind: "collection", id })}
                  refused={refused?.key === key ? refused?.said : undefined}
                />
              ) : null}
            </RequestTab>
          ) : tab === "environments" ? (
            <Environments api={api} read={now} sessionId={item.sessionId} repository={repository} locked={locked} onRead={setLatest} />
          ) : (
            <History rows={history} onOpen={openFromHistory} />
          )}
        </>
      )}
    </SidePanel>
  );
}

/** A file that would not read, and Fleet's why. None of its items are listed, so the panel never draws it as empty. */
function Unread({ unread }: { unread: RequestsNotRead }) {
  const why = /[.!?]$/.test(unread.why.trim()) ? unread.why.trim() : `${unread.why.trim()}.`;
  return (
    <Alert tone="caution" title={`${unreadFileOf(unread)} would not read`}>
      {unread.place === "yours" ? `${why} Save is off until it reads.` : why}
    </Alert>
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

/** A label leading its value on one line: `Sends to`, `Content type`, `Kept`. */
function Lead({ lead, children }: { lead: string; children: ReactNode }) {
  return (
    <p className="side-request__lead-line">
      <span className="side-request__lead">{lead}</span>
      {children}
    </p>
  );
}

/** A table of names and values nobody edits here: a collection's headers under a request, a server's variables, an answer's headers. */
function ReadTable({ label, rows, dimmed = false }: { label: string; rows: readonly { name: string; value: ReactNode }[]; dimmed?: boolean }) {
  return (
    <Table className="side-table" aria-label={label}>
      <colgroup>
        <col className="side-table__name" />
        <col />
      </colgroup>
      <TableHead>
        <TableRow>
          <TableHeaderCell>Name</TableHeaderCell>
          <TableHeaderCell>Value</TableHeaderCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map((one, at) => (
          <TableRow key={`${at}:${one.name}`} dimmed={dimmed}>
            <TableCell variant="mono">
              <span className="side-table__value">{one.name}</span>
            </TableCell>
            <TableCell variant="mono">
              <span className="side-table__value">{one.value}</span>
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/** Headers as rows of Name and Value, each a field, with Add header under them. */
function HeaderTable({ label, rows, onRows }: { label: string; rows: readonly HeaderRow[]; onRows: (rows: HeaderRow[]) => void }) {
  const bad = badRow(rows);
  const edit = (at: number, patch: Partial<HeaderRow>) => onRows(rows.map((one, i) => (i === at ? { ...one, ...patch } : one)));
  return (
    <div className="side-requests__rows">
      <Table className="side-table" aria-label={label}>
        <colgroup>
          <col className="side-table__name" />
          <col />
          <col className="side-table__icon" />
        </colgroup>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Name</TableHeaderCell>
            <TableHeaderCell>Value</TableHeaderCell>
            <TableHeaderCell />
          </TableRow>
        </TableHead>
        <TableBody>
          {rows.map((one, at) => (
            <TableRow key={one.key}>
              <TableCell>
                <Input
                  aria-label={`Header ${at + 1} name`}
                  mono
                  value={one.name}
                  placeholder="Content-Type"
                  spellCheck={false}
                  invalid={at === bad}
                  {...(at === bad ? { message: "A header name has no spaces" } : {})}
                  onChange={(event) => edit(at, { name: event.currentTarget.value })}
                />
              </TableCell>
              <TableCell>
                <Input aria-label={`Header ${at + 1} value`} mono value={one.value} spellCheck={false} onChange={(event) => edit(at, { value: event.currentTarget.value })} />
              </TableCell>
              <TableCell>
                <Tooltip label="Remove" asChild>
                  <Button variant="ghost" size="sm" iconOnly aria-label={`Remove header ${at + 1}`} onClick={() => onRows(rows.filter((_, i) => i !== at))}>
                    <Trash2 size={16} aria-hidden="true" />
                  </Button>
                </Tooltip>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <div>
        <Button variant="ghost" size="sm" onClick={() => onRows([...rows, blankRow()])}>
          Add header
        </Button>
      </div>
    </div>
  );
}

/** The Request tab: what is open, under the collections where the window is at its floor and they cannot float beside the panel. */
function RequestTab({ tree, children }: { tree: ReactNode; children: ReactNode }) {
  return (
    <div className="side-request">
      {tree}
      {tree === undefined || children === null ? null : <Separator />}
      {children}
    </div>
  );
}

function CollectionTree({ read, open, onOpen }: { read: Requests; open: Open | undefined; onOpen: (open: Open) => void }) {
  const [folded, setFolded] = useState<ReadonlySet<string>>(new Set());
  const step = (depth: number) => ({ "--depth": depth }) as CSSProperties;
  return (
    <div className="armada-pr-tree side-requests__tree">
      <div className="armada-pr-tree__head">
        <span />
        <Button variant="ghost" size="sm" onClick={() => onOpen({ kind: "draft", start: undefined, at: Date.now() })}>
          New request
        </Button>
      </div>
      {read.collections.length === 0 ? (
        read.not_read.length > 0 ? null : <p className="side-request__none">No collections in this repository.</p>
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
  );
}

function CollectionView({ collection, rows, onRows, refused }: { collection: Placed<RequestCollection>; rows: readonly HeaderRow[]; onRows: (rows: HeaderRow[]) => void; refused: string | undefined }) {
  return (
    <section className="side-requests__section" aria-label={`Collection ${collection.name}`}>
      <Lead lead="Kept">{collection.place === "repository" ? "In the repository" : collection.hides ? "Yours. Hides the repository's" : "Yours"}</Lead>
      <div className="side-requests__group">
        <h4 className="side-requests__title">Shared headers</h4>
        <p className="side-request__none">{`Every request in ${collection.name} sends these.`}</p>
        <HeaderTable label={`Headers of ${collection.name}`} rows={rows} onRows={onRows} />
      </div>
      {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
    </section>
  );
}

/** The address as it will go: filled from the environment picked, or what it lacks, in the warning hue. */
function SendsTo({ url, picked }: { url: string; picked: RequestEnvironmentRead | undefined }) {
  if (url.trim() === "") return null;
  const filled = filledOf(url, picked);
  if (filled.missing.length === 0)
    return (
      <Lead lead="Sends to">
        <span className="mono">{filled.text}</span>
      </Lead>
    );
  const names = filled.missing.map((one) => `{{${one}}}`).join(", ");
  const has = filled.missing.length === 1 ? "has" : "have";
  return (
    <p className="side-request__lead-line" data-tone="caution">
      <Tooltip label="No value">
        <span className="side-request__warn" role="img" aria-label="No value">
          <TriangleAlert size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span>
        <code className="mono">{names}</code>
        {picked === undefined ? ` ${has} no value with no environment picked` : ` ${has} no value in this environment`}
      </span>
    </p>
  );
}

type EditorProps = {
  api: RequestsApi;
  read: Requests;
  sessionId: string | undefined;
  /** The repository picked with no Job open. */
  repository: string | undefined;
  environment: string;
  draft: Draft;
  onDraft: (patch: Partial<Draft>) => void;
  /** The collection a saved request is in, whose headers go with it. */
  collection: Placed<RequestCollection> | undefined;
  /** In no collection yet, so it takes a name and a collection to save. */
  fresh: boolean;
  /** What the body's text is kept under. */
  scope: string;
  sent: Sent | undefined;
  onSent: (sent: Sent) => void;
  onEditCollection: (id: string) => void;
  refused: string | undefined;
};

function Editor({ api, read, sessionId, repository, environment, draft, onDraft, collection, fresh, scope, sent, onSent, onEditCollection, refused }: EditorProps) {
  const [part, setPart] = useState("headers");
  const [sending, setSending] = useState(false);
  const picked = read.environments.find((one) => one.name === environment);
  const bad = badRow(draft.headers);
  const inherited = collection === undefined || collection.headers.length === 0 ? undefined : collection;
  const own = headersFrom(draft.headers);
  const type = [...own, ...(collection?.headers ?? [])].find((one) => one.name.toLowerCase() === "content-type")?.value;

  const send = async () => {
    if (bad >= 0) return;
    setSending(true);
    const answer = await api
      .sendRequest({
        ...(sessionId === undefined ? {} : { session_id: sessionId }),
        ...(repository === undefined ? {} : { repository }),
        method: draft.method,
        url: draft.url.trim(),
        ...(environment === "" ? {} : { environment }),
        ...(collection === undefined ? {} : { collection: collection.id }),
        headers: own,
        ...(draft.body === "" ? {} : { body: draft.body }),
      })
      .catch(() => undefined);
    setSending(false);
    onSent(answer?.ok === true ? { answer: answer.value, from: inherited?.name } : { said: said(answer) ?? "" });
  };

  return (
    <>
      <form
        className="side-request__form"
        aria-label="Request"
        onSubmit={(event) => {
          event.preventDefault();
          void send();
        }}
      >
        {!fresh ? null : (
          <div className="side-request__line" data-even>
            <Input label="Name" value={draft.name} spellCheck={false} onChange={(event) => onDraft({ name: event.currentTarget.value })} />
            <Select label="Collection" value={draft.collection} onChange={(event) => onDraft({ collection: event.currentTarget.value })}>
              <option value="">None</option>
              {read.collections.map((one) => (
                <option key={one.id} value={one.id}>
                  {one.name}
                </option>
              ))}
            </Select>
          </div>
        )}
        <div className="side-request__send-line">
          <Select label="Method" value={draft.method} onChange={(event) => onDraft({ method: event.currentTarget.value })}>
            {METHODS.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
          <Input label="Address" mono value={draft.url} placeholder="{{base}}/path" spellCheck={false} onChange={(event) => onDraft({ url: event.currentTarget.value })} />
          <Button type="submit" variant="primary" disabled={sending || bad >= 0 || draft.url.trim() === ""}>
            {sending ? "Sending" : "Send"}
          </Button>
        </div>
        <SendsTo url={draft.url} picked={picked} />
        {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
        <div className="side-request__parts">
          <Tabs
            items={[
              { id: "headers", label: "Headers" },
              { id: "body", label: "Body" },
            ]}
            value={part}
            onChange={setPart}
          />
          <TabPanel tab={part} role="tabpanel" aria-label={part === "headers" ? "Headers" : "Body"} className="side-requests__section">
            {part === "headers" ? (
              <>
                {inherited === undefined ? null : (
                  <section className="side-requests__group" role="group" aria-label={`From ${inherited.name}`}>
                    <div className="side-requests__group-head">
                      <h5 className="side-requests__subtitle">{`From ${inherited.name}`}</h5>
                      <span className="side-requests__push">
                        <Button variant="ghost" size="sm" aria-label={`Edit ${inherited.name}`} onClick={() => onEditCollection(inherited.id)}>
                          Edit
                        </Button>
                      </span>
                    </div>
                    <ReadTable label={`Headers from ${inherited.name}`} rows={inherited.headers} dimmed />
                  </section>
                )}
                <section className="side-requests__group" role="group" aria-label="This request">
                  {inherited === undefined ? null : <h5 className="side-requests__subtitle">This request</h5>}
                  <HeaderTable label="Headers" rows={draft.headers} onRows={(headers) => onDraft({ headers })} />
                </section>
              </>
            ) : (
              <>
                <Lead lead="Content type">{type === undefined ? "Not stated" : <span className="mono">{type}</span>}</Lead>
                <Textarea aria-label="Body" keep={`${scope}:body`} className="mono" rows={6} spellCheck={false} value={draft.body} onChange={(event) => onDraft({ body: event.currentTarget.value })} />
              </>
            )}
          </TabPanel>
        </div>
      </form>
      {sent === undefined ? null : (
        <>
          <Separator />
          {"said" in sent ? <Alert tone="escalated">{sent.said}</Alert> : <Response sent={sent.answer} from={sent.from} />}
        </>
      )}
    </>
  );
}

function Response({ sent, from }: { sent: RequestSent; from: string | undefined }) {
  const [part, setPart] = useState("body");
  const shown = bodyShown(sent.body);
  return (
    <section className="side-requests__section" aria-label="Response">
      <h4 className="side-requests__title">Response</h4>
      <p className="side-request__summary">
        <span className="side-request__status">
          <RequestMark status={sent.status ?? null} error={sent.error} />
          <span className="side-request__code">{sent.status === undefined ? "No answer" : sent.status}</span>
        </span>
        <span className="mono">{requestTimeOf(sent.ms)}</span>
        {from === undefined ? null : <span>{`With ${from} headers`}</span>}
        {sent.truncated ? <span>Body cut at 256 KiB</span> : null}
      </p>
      {sent.error === undefined ? null : <p className="side-request__said">{sent.error}</p>}
      {sent.note === undefined ? null : <p className="side-request__said">{sent.note}</p>}
      <div className="side-request__parts">
        <Tabs
          items={[
            { id: "body", label: "Body" },
            { id: "headers", label: "Headers" },
          ]}
          value={part}
          onChange={setPart}
        />
        <TabPanel tab={part} role="tabpanel" aria-label={part === "body" ? "Response body" : "Response headers"} className="side-requests__section">
          {part === "body" ? (
            sent.body === "" ? (
              <p className="side-request__none">No body.</p>
            ) : (
              <pre className="side-log__lines mono" role="log" aria-label={shown.json ? "Body, JSON" : "Body"}>
                {shown.text}
              </pre>
            )
          ) : sent.headers.length === 0 ? (
            <p className="side-request__none">No headers.</p>
          ) : (
            <ReadTable label="Response headers" rows={sent.headers} />
          )}
        </TabPanel>
      </div>
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

type EnvironmentsProps = { api: RequestsApi; read: Requests; sessionId: string | undefined; repository: string | undefined; locked: boolean; onRead: (read: Requests) => void };

export function Environments({ api, read, sessionId, repository, locked, onRead }: EnvironmentsProps) {
  const [adding, setAdding] = useState(false);
  const editable = read.environments.filter((one) => one.place !== "server");
  const servers = read.environments.filter((one) => one.place === "server");
  const saved = (next: Requests) => {
    setAdding(false);
    onRead(next);
  };
  return (
    <div className="side-requests__groups">
      {editable.length === 0 && !adding && read.not_read.length === 0 ? <p className="side-request__none">No environments in this repository.</p> : null}
      {editable.map((one) => (
        <EnvironmentForm key={`${one.name}:${JSON.stringify(one.variables)}`} api={api} sessionId={sessionId} repository={repository} environment={one} locked={locked} onSaved={saved} />
      ))}
      {adding ? <EnvironmentForm api={api} sessionId={sessionId} repository={repository} environment={undefined} locked={locked} onSaved={saved} /> : null}
      {adding || locked ? null : (
        <div>
          <Button variant="ghost" onClick={() => setAdding(true)}>
            Add environment
          </Button>
        </div>
      )}
      {servers.map((one) => (
        <section key={one.name} className="side-requests__group" role="group" aria-label={one.name}>
          <div className="side-requests__group-head">
            <h4 className="side-requests__title mono">{one.name}</h4>
            <span className="side-requests__hides">This Job's server. Read only</span>
          </div>
          <ReadTable label={`Variables of ${one.name}`} rows={one.variables.map((variable) => ({ name: variable.name, value: variable.secret ? "Set" : variable.value }))} />
        </section>
      ))}
    </div>
  );
}

/** A variable as the form holds it. A secret already set is drawn as set until Replace opens its field. */
type Line = { name: string; secret: boolean; value: string; set: boolean; replacing: boolean };

const blankLine = (): Line => ({ name: "", secret: false, value: "", set: false, replacing: false });
const empty = (one: Line): boolean => one.name === "" && one.value === "" && !one.set;
/** The table always ends with one empty row to type into. */
const endingEmpty = (lines: Line[]): Line[] => {
  let end = lines.length;
  while (end > 0 && empty(lines[end - 1]!)) end -= 1;
  return [...lines.slice(0, end), ...(end < lines.length ? [lines[end]!] : [blankLine()])];
};

type EnvironmentFormProps = { api: RequestsApi; sessionId: string | undefined; repository: string | undefined; environment: Placed<RequestEnvironmentRead> | undefined; locked: boolean; onSaved: (read: Requests) => void };

function EnvironmentForm({ api, sessionId, repository, environment, locked, onSaved }: EnvironmentFormProps) {
  const [name, setName] = useState(environment?.name ?? "");
  const [lines, setLines] = useState<Line[]>(() =>
    endingEmpty(
      (environment?.variables ?? []).map((one) => (one.secret ? { name: one.name, secret: true, value: "", set: one.set, replacing: false } : { name: one.name, secret: false, value: one.value, set: false, replacing: false })),
    ),
  );
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const [deleting, setDeleting] = useState(false);
  const edit = (at: number, patch: Partial<Line>) => setLines((now) => endingEmpty(now.map((one, i) => (i === at ? { ...one, ...patch } : one))));
  const save = async () => {
    const variables: VariableWrite[] = lines
      .filter((one) => one.name.trim() !== "")
      .map((one) => (one.secret ? { name: one.name.trim(), secret: true, ...(one.value === "" ? {} : { value: one.value }) } : { name: one.name.trim(), secret: false, value: one.value }));
    const answer = await api.saveRequestEnvironment({ name: name.trim(), variables }, sessionId, repository).catch(() => undefined);
    // A secret's value goes whether or not Fleet kept it: the field never holds one longer than a press.
    setLines((now) => now.map((one) => (one.secret ? { ...one, value: "" } : one)));
    setRefused(said(answer));
    if (answer?.ok === true) onSaved(answer.value);
  };
  const remove = async () => {
    if (environment === undefined) return;
    const answer = await api.deleteRequestEnvironment(environment.name, sessionId, repository).catch(() => undefined);
    setDeleting(false);
    setRefused(said(answer));
    if (answer?.ok === true) onSaved(answer.value);
  };
  return (
    <form
      className="side-requests__group"
      role="group"
      aria-label={environment?.name ?? "New environment"}
      onSubmit={(event) => {
        event.preventDefault();
        void save();
      }}
    >
      <div className="side-requests__group-head">
        {environment === undefined ? (
          <h4 className="side-requests__title">New environment</h4>
        ) : (
          <>
            <h4 className="side-requests__title mono">{environment.name}</h4>
            <PlaceMark placed={environment} />
          </>
        )}
      </div>
      {environment === undefined ? <Input label="Environment name" value={name} placeholder="test" spellCheck={false} onChange={(event) => setName(event.currentTarget.value)} /> : null}
      <Table className="side-table" aria-label={`Variables of ${environment?.name ?? "the new environment"}`}>
        <colgroup>
          <col className="side-table__name" />
          <col />
          <col className="side-table__check" />
          <col className="side-table__icon" />
        </colgroup>
        <TableHead>
          <TableRow>
            <TableHeaderCell>Name</TableHeaderCell>
            <TableHeaderCell>Value</TableHeaderCell>
            <TableHeaderCell>Secret</TableHeaderCell>
            <TableHeaderCell />
          </TableRow>
        </TableHead>
        <TableBody>
          {lines.map((one, at) => (
            <TableRow key={at}>
              <TableCell>
                <Input aria-label={`Variable ${at + 1}`} mono value={one.name} placeholder="base" spellCheck={false} onChange={(event) => edit(at, { name: event.currentTarget.value })} />
              </TableCell>
              <TableCell>
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
              </TableCell>
              <TableCell>
                <Checkbox aria-label={`Secret ${at + 1}`} checked={one.secret} disabled={one.set} onChange={(event) => edit(at, { secret: event.currentTarget.checked })}>
                  {null}
                </Checkbox>
              </TableCell>
              <TableCell>
                {at === lines.length - 1 && empty(one) ? null : (
                  <Tooltip label="Remove" asChild>
                    <Button variant="ghost" size="sm" iconOnly aria-label={`Remove variable ${at + 1}`} disabled={locked} onClick={() => setLines((now) => now.filter((_, i) => i !== at))}>
                      <Trash2 size={16} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                )}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <div className="side-request__acts">
        <Button type="submit" variant="secondary" disabled={name.trim() === "" || locked}>
          Save
        </Button>
        {environment?.place === "yours" ? (
          <Button variant="ghost" size="sm" disabled={locked} onClick={() => (deleting ? void remove() : setDeleting(true))}>
            {deleting ? "Press again to delete" : "Delete"}
          </Button>
        ) : null}
      </div>
      {refused === undefined ? null : <Alert tone="escalated">{refused}</Alert>}
    </form>
  );
}
