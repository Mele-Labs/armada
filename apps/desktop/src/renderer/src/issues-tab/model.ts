// What the Issues tab draws. **Bridge never knows which tracker sent an issue**: a tracker's own facts (GitHub's labels,
// milestone and projects; Jira's status, priority, sprint and the rest) arrive as named fields, and a query is text in the
// tracker's own language that Bridge shows and never reads. Fleet serves these over the tracker routes (`source.ts`); the
// mock's `issues-tab` scenario stands in for it in a walk (`mock/slices/issues.ts`).

/** One field the tracker returned for an issue, by the tracker's own name for it. */
export type TicketField = { name: string; values: string[] };

/** An issue as a list holds it. `reference` is as the tracker writes it: `#2810`, `ABC-41`. */
export type Ticket = {
  reference: string;
  title: string;
  url: string;
  state: "open" | "closed";
  /** The reporter. */
  author?: string;
  assignees: string[];
  updated_at: string;
  excerpt: string;
  fields: TicketField[];
  linked_pull_request: boolean;
  /** The Jobs working it, by id, as Fleet's ledger holds them. */
  holders: string[];
};

/** A pull request that names an issue. */
/** A pull request naming the issue, and its Checks where the forge reports them. */
export type LinkedPull = { number: number; title: string; address: string; state: "open" | "merged" | "closed"; checks?: { state: "passed" | "running" | "failed"; said: string } };

/** One issue read whole, for the reader: the row, and what only reading it answers. */
export type TicketRead = Ticket & { body: string; linked_pull_requests: LinkedPull[] };

/** A query he saved for a repository: his words, the query the model wrote for them (or he edited), and the fields its rows show. */
export type SavedQuery = { id: string; name: string; words: string; query: string; row_fields: string[] };

/** What the model wrote for his words, in the tracker's own language. */
export type WrittenQuery = {
  query: string;
  row_fields: string[];
  /** What the query matched, where the call already ran it. Then the list is not asked again. */
  tickets?: Ticket[];
};

/** A query to keep. With an `id` it edits that saved query; without, Fleet gives the new one its own. */
export type QueryToSave = Omit<SavedQuery, "id"> & { id?: string };

/**
 * A repository's tracker, as Fleet reads it. **Read** is any tracker a build or a mod reads; `mod` names the tracker mod
 * where it is one. **Refused** is a tracker the Manifest names and nothing reads, with Fleet's words for why.
 */
export type Tracker =
  | {
      manifestId: string;
      state: "read";
      /** What the tracker is called, for the link out. */
      name: string;
      mod?: string;
      /** The built-in queries, in the tracker's own language. */
      open: string;
      assigned: string;
      /** The built-in query for closed issues. A tracker without one is left out of the Closed choice. */
      closed?: string;
      saved: SavedQuery[];
    }
  /** `named` is the provider the Manifest names, where Fleet says it. */
  | { manifestId: string; state: "refused"; named?: string; said: string };

export type ReadTracker = Extract<Tracker, { state: "read" }>;

/** An answer, or Fleet's words for why there is none. */
export type Read<T> = { ok: true; value: T } | { ok: false; said: string; /** The program the tracker could not run, as Fleet named it. */ missing?: string; /** The program the tracker ran and found no project for, as Fleet named it. */ needsSetup?: string };

/**
 * A bug an agent in a Job reported in Armada itself, as Fleet's `BugReport` with `kept` read as whether the owner kept it.
 * `job_title` is the title of the Job it came from, `at` an RFC 3339 instant, `errors` the exact lines and `logs` the paths
 * the agent named.
 */
export type ReportedBug = {
  id: string;
  session_id: string;
  job_title: string;
  title: string;
  what: string;
  steps: string[];
  errors: string[];
  logs: string[];
  at: string;
  kept: boolean;
  /** The pull request of the Job started on it merged: the report is fixed. Absent while it stands open. */
  fixed?: { at: string; pull_request: number; title: string; url: string };
};

/** What the tab reads and writes. Fleet answers it (`source.ts`), and the mock does in a walk. */
export type IssuesSource = {
  /** Reported bugs, the two acts on one, and Fleet telling the whole list again. Absent where no Fleet serves them. */
  reports?: {
    list: () => Promise<Read<ReportedBug[]>>;
    keep: (id: string) => Promise<Read<ReportedBug>>;
    drop: (id: string) => Promise<Read<void>>;
    /** Hears `bug_reports.changed` until the returned function is called. */
    follow: (onChanged: (reports: ReportedBug[]) => void) => () => void;
  };
  /** The tracker of each of these repositories, by Manifest id. One Fleet does not serve is left out. */
  trackers: (manifestIds: readonly string[]) => Promise<Read<Tracker[]>>;
  /** The issues a query matches. Runs the query as written: no model is called. */
  run: (manifestId: string, query: string) => Promise<Read<Ticket[]>>;
  read: (manifestId: string, reference: string) => Promise<Read<TicketRead>>;
  /** Installs the program a refused read named as missing. Only a press calls it. `said` is why it did not install. */
  install: (manifestId: string, tool: string) => Promise<Read<void>>;
  /** A model writes a query for his words, in this repository's tracker's language. This is the call that spends. */
  write: (manifestId: string, words: string) => Promise<Read<WrittenQuery>>;
  /** Keeps a query, new or edited, and answers the repository's saved queries. */
  save: (manifestId: string, query: QueryToSave) => Promise<Read<SavedQuery[]>>;
  remove: (manifestId: string, id: string) => Promise<Read<SavedQuery[]>>;
};

/** The built-ins every repository has, and the box a query is described in. */
export const OPEN = "open";
export const ASSIGNED = "assigned";
export const CLOSED = "closed";
export const DESCRIBE = "describe";
/** A saved query is chosen by its name, so on All one name reaches each repository that keeps it. */
export const savedChoice = (name: string): string => `saved:${name}`;
export const savedName = (choice: string): string | undefined => (choice.startsWith("saved:") ? choice.slice("saved:".length) : undefined);

/** What a built-in query's rows show. */
export const BUILT_IN_FIELDS = ["Labels"];
