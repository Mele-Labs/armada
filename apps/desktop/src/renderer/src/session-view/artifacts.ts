// Everything a Session made or used, grouped for the Artifacts layer. It is read from what the ledger already holds (the attachments, drawn
// as `LedgerEntry` rows that know where each press goes) and from the thread: the files its drones touched, the commits and the commands it ran.
// Nothing is fetched and nothing is invented. The ledger holds no commit of its own, so commits are the thread's `git commit` lines.

import { ExternalLink, GitCommitHorizontal, GitPullRequest, Link, MessageSquareText, PencilRuler, Presentation, Split, SquareTerminal, Ticket, type LucideIcon } from "lucide-react";
import { ARTIFACT, BACKGROUND, FILE_SHOWN, closedMark, type LedgerEntry, type LedgerPullRequest, type SessionLedgerTone, type SessionState } from "@armada/components";
import { attachmentsOf, issueLabel, type Session, type SessionRow } from "@armada/screens/src/draft/sessions";

import { openLink } from "../commands";
import { NOTE_SAID, shownOnly } from "../session-cards";
import { openInPanel } from "../shared-panel/store";
import { childrenOf, linkedPropsOf } from "./linked";

export type ArtifactRow = {
  key: string;
  Glyph: LucideIcon;
  /** What the glyph is, as its tooltip says it. */
  said: string;
  title: string;
  /** An address, a path or a branch: mono, quiet, after the title. */
  sub?: string;
  /** Quiet words after `sub`, such as the screen a note was written on. */
  words?: readonly string[];
  /** The row's state, as the ledger's marks draw it. `word` stands beside the glyph in place of the tone's own. */
  mark?: { glyph: SessionLedgerTone; said: string; word?: string };
  /** A picture's absolute path, drawn as a thumbnail and read as an artifact file is. */
  picture?: string;
  pr?: LedgerPullRequest;
  /** Set on a pull request's own row, which the layer draws as Landing does. */
  prNumber?: number;
  /** Absent for a row that has nowhere to go. */
  open?: () => void;
  /** An item he closed from Needs you: when, and Reopen where its thing still stands. */
  closed?: { at: string; onReopen?: () => void };
};

export type ArtifactGroup = { id: string; label: string; rows: ArtifactRow[] };

const ACTS = { write: "Written", edit: "Edited", remove: "Removed" } as const;

/** A sent note's state, drawn with the ledger's marks: open waits on the Job, returned went back when the Job closed unfinished. */
const NOTE_TONE = { open: "pending", done: "done", returned: "stopped" } as const satisfies Record<keyof typeof NOTE_SAID, SessionLedgerTone>;

/** The ledger row an attachment is drawn as, which knows where its press goes. */
const entryOf = (entries: readonly LedgerEntry[], kind: LedgerEntry["kind"], key: string) => entries.find((one) => one.kind === kind && one.key === key);

const commitOf = (row: SessionRow): string | undefined => {
  if (row.kind !== "tool") return undefined;
  return /^Bash\s+git commit\b.*?(?:-m|--message)\s+(?:"([^"]+)"|'([^']+)')/.exec(row.text)?.slice(1).find((one) => one !== undefined);
};

const commandOf = (row: SessionRow): ArtifactRow | undefined => {
  if (row.kind === "ran") {
    const sub = row.code === null ? "Stopped" : row.code === 0 ? undefined : `Exit ${row.code}`;
    return { key: `ran:${row.id}`, Glyph: SquareTerminal, said: "Command run", title: row.command, ...(sub === undefined ? {} : { sub }) };
  }
  if (row.kind === "tool" && row.text.startsWith("Bash ") && commitOf(row) === undefined) return { key: `bash:${row.id}`, Glyph: SquareTerminal, said: "Command run", title: row.text.slice(5) };
  return undefined;
};

/** A Session's state as the ledger's marks say it. */
const TONE: Record<SessionState, SessionLedgerTone> = { working: "running", stuck: "failed", failing: "failed", waiting: "escalated", replied: "done", done: "done", ended: "stopped", quiet: "stopped", idle: "pending", blank: "pending", limited: "pending", after: "pending", draft: "pending", held: "pending", paused: "pending", unstarted: "failed" };

/** A web address as its row titles it: the host and the path, with no scheme, query or trailing slash. Nothing is fetched. */
export const linkTitle = (url: string): string => {
  try {
    const { host, pathname } = new URL(url);
    return `${host}${pathname === "/" ? "" : pathname.replace(/\/$/, "")}`;
  } catch {
    return url;
  }
};

/** The other Sessions in the window, for the ones this Session started, and how to open one. */
export type Beside = { sessions: readonly Session[]; open: (id: string) => void };

export function artifactsOf(session: Session, entries: readonly LedgerEntry[], beside?: Beside): ArtifactGroup[] {
  const groups: ArtifactGroup[] = [];
  const add = (id: string, label: string, rows: ArtifactRow[]) => {
    if (rows.length > 0) groups.push({ id, label, rows });
  };
  const open = (entry: LedgerEntry | undefined) => (entry === undefined ? {} : { open: entry.onOpen });
  const newest = <T,>(rows: readonly T[]) => [...rows].reverse();

  // First section: the Jobs this one started, with the plan group each was handed under it where it had one.
  add(
    "session",
    "Jobs",
    beside === undefined
      ? []
      : childrenOf(beside.sessions, session).map(({ session: child, title }) => {
          const props = linkedPropsOf(child, beside.open);
          return { key: `session:${child.id}`, Glyph: Link, said: "Linked Job", title: props.title, ...(title === undefined ? {} : { sub: title }), mark: { glyph: TONE[props.state], said: props.said }, ...(props.pr === undefined ? {} : { pr: props.pr }), open: props.onOpen };
        }),
  );

  add(
    "pull_request",
    "Pull requests",
    newest(attachmentsOf(session, "pull_request")).map((one) => {
      const entry = entryOf(entries, "pull_request", String(one.number));
      return { key: `pr:${one.number}`, Glyph: GitPullRequest, said: "Pull request", title: `#${one.number} ${one.title}`, sub: one.branch, prNumber: one.number, ...(entry?.pr === undefined ? {} : { pr: entry.pr }), ...open(entry) };
    }),
  );

  // What he closed from Needs you, as the ledger holds it, newest first.
  add(
    "closed",
    "Closed",
    entries.flatMap((entry) => (entry.kind !== "closed" || entry.closed === undefined ? [] : [{ key: entry.key, ...closedMark(entry), title: entry.name, closed: entry.closed }])),
  );

  add(
    "issue",
    "Issues",
    attachmentsOf(session, "issue").map((one) => ({
      key: `issue:${one.number}`,
      Glyph: Ticket,
      said: "Issue",
      title: `${issueLabel(one.number)} ${one.title}`,
      ...(one.state === "closed" ? { mark: { glyph: "done" as const, said: "Closed" } } : {}),
      open: () => void openInPanel({ kind: "issue", number: one.number, address: one.address, title: one.title, ...(session.manifestId === undefined ? {} : { manifestId: session.manifestId }) }),
    })),
  );

  const commits = session.rows.flatMap((row) => {
    const message = commitOf(row);
    return message === undefined ? [] : [{ key: `commit:${row.id}`, Glyph: GitCommitHorizontal, said: "Commit", title: message, sub: row.at } satisfies ArtifactRow];
  });
  add("commit", "Commits", newest(commits));

  const made = attachmentsOf(session, "artifact");
  const artifactRow = (one: (typeof made)[number]): ArtifactRow => {
    const { Glyph, said } = ARTIFACT[one.form];
    return { key: `artifact:${one.id}`, Glyph, said: shownOnly(one) ? FILE_SHOWN : said, title: one.title, sub: one.id, ...open(entryOf(entries, "artifact", one.id)) };
  };
  add("page", "Pages", newest(made.filter((one) => one.form === "page" || one.form === "window")).map(artifactRow));

  const written = made.filter((one) => one.form === "file" || one.form === "image");
  const named = new Set(written.map((one) => one.id));
  const touched = (session.work?.files ?? []).flatMap((one) => (one.act === undefined || named.has(one.path) ? [] : [{ key: `touched:${one.path}`, Glyph: ARTIFACT.file.Glyph, said: "File touched", title: one.path, sub: ACTS[one.act] } satisfies ArtifactRow]));
  add("file", "Files", [...newest(written).map(artifactRow), ...touched]);
  add("doc", "Docs", newest(made.filter((one) => one.form === "doc" || one.form === "document")).map(artifactRow));

  add(
    "sketch",
    "Sketches",
    newest(attachmentsOf(session, "sketch")).map((one) => ({ key: `sketch:${one.id}`, Glyph: PencilRuler, said: "Sketch", title: one.title, sub: one.by === "agent" ? "Drawn by the agent" : "Drawn by you", ...open(entryOf(entries, "sketch", one.id)) })),
  );

  add(
    "note",
    "Notes",
    newest(attachmentsOf(session, "note")).map((one) => ({
      key: `note:${one.id}`,
      Glyph: MessageSquareText,
      said: "Note",
      title: one.text,
      ...(one.component === undefined ? {} : { sub: one.component }),
      ...(one.screen === undefined ? {} : { words: [one.screen] }),
      mark: { glyph: NOTE_TONE[one.status], said: NOTE_SAID[one.status], word: NOTE_SAID[one.status] },
      ...(one.picture === undefined ? {} : { picture: one.picture }),
    })),
  );

  add(
    "link",
    "Links",
    newest(attachmentsOf(session, "link")).map((one) => ({ key: `link:${one.url}`, Glyph: ExternalLink, said: "Link", title: linkTitle(one.url), sub: one.by === "agent" ? "Agent" : "You", open: () => void openLink(one.url) })),
  );

  add(
    "subagent",
    "Subagents",
    [...attachmentsOf(session, "subagent")]
      .sort((a, b) => Number(b.state === "running") - Number(a.state === "running"))
      .map((one) => {
        const entry = entryOf(entries, "subagent", one.id);
        return { key: `subagent:${one.id}`, Glyph: Split, said: "Subagent", title: one.task, ...(one.agentType === undefined ? {} : { sub: one.agentType }), ...(entry?.mark === undefined ? {} : { mark: entry.mark }), ...open(entry) };
      }),
  );

  const background = attachmentsOf(session, "background").map((one): ArtifactRow => {
    const entry = entryOf(entries, "background", one.id);
    return { key: `background:${one.id}`, Glyph: BACKGROUND[one.form].Glyph, said: BACKGROUND[one.form].said, title: one.task, sub: one.command, ...(entry?.mark === undefined ? {} : { mark: entry.mark }), ...open(entry) };
  });
  const ran = newest(session.rows.flatMap((row) => commandOf(row) ?? []));
  add("command", "Commands", [...background, ...ran]);

  add(
    "studio",
    "Studios",
    attachmentsOf(session, "studio").map((one) => ({ key: `studio:${one.id}`, Glyph: Presentation, said: "Studio", title: one.title, ...open(entryOf(entries, "studio", one.id)) })),
  );
  return groups;
}

/** The groups cut to the rows the words match, in a row's title, its address or path, or its group's name. A group with no row left is gone. */
export function matching(groups: readonly ArtifactGroup[], words: string): ArtifactGroup[] {
  const query = words.trim().toLowerCase();
  if (query === "") return [...groups];
  return groups.flatMap((group) => {
    const rows = group.label.toLowerCase().includes(query) ? group.rows : group.rows.filter((row) => `${row.title} ${row.sub ?? ""} ${row.words?.join(" ") ?? ""} ${row.said}`.toLowerCase().includes(query));
    return rows.length === 0 ? [] : [{ ...group, rows }];
  });
}
