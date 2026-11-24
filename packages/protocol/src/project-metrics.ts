// A project's live metrics: the recipe the librarian writes once, and what Fleet read with it. Mirrors `crates/ipc/src/metrics.rs`.
// A recipe is kept in the project's file as `look.metrics`, so it reaches his other Macs; a token is kept in this Mac's keychain and is never on the wire's way out.

/** A header of an HTTP call, or a variable of an MCP server's environment. */
export type MetricHeader = { name: string; value: string };

/**
 * One call, as a tracker mod's call is written. `{credential}` is filled into `env`, `arguments`, `headers` and `body`
 * only, never `argv`, `server` or `url`.
 */
export type MetricCall =
  /** A program and its arguments, run with no shell. */
  | { transport: "cli"; argv: string[] }
  | { transport: "http"; method: string; url: string; headers?: MetricHeader[]; body?: string }
  /** A server started on standard input and output for the call, and one tool called on it. `arguments` is JSON text, `{}` where absent. */
  | { transport: "mcp"; server: string[]; env?: MetricHeader[]; tool: string; arguments?: string };

/** The token a recipe signs in with. One per tool on each Mac: two recipes naming `sentry` share one. */
export type MetricCredential = {
  /** The tool, as the keychain item is named: `sentry`, `grafana`. */
  name: string;
  /** What to make, in a sentence. */
  how: string;
  /** Where it is made. */
  url?: string;
};

/** What the librarian writes once. `map` is a `jq` filter whose output reads as a {@link MetricAnswer}. */
export type MetricRecipe = {
  /** Unique in the project, lower case with dashes. The block that shows it is `metric:<id>`. */
  id: string;
  title: string;
  call: MetricCall;
  map: string;
  /** How long a reading stands before Bridge's next ask fetches again. Absent is 60. */
  every_secs?: number;
  credential?: MetricCredential;
};

/** One row of an answer. `value` is a number Graph can draw, or words. */
export type MetricRow = {
  label: string;
  value: number | string;
  unit?: string;
  /** When the tool says the value stood. Rows that carry it are a series. */
  at?: string;
  link?: string;
};

/** What `map` prints. `picture` is an image the tool renders, offered only where an answer gives one. */
export type MetricAnswer = {
  rows: MetricRow[];
  picture?: string;
};

/** One kept value of the first numeric row, from a reading Fleet made while the dashboard was open. */
export type MetricPoint = { at: string; value: number };

/** Where a recipe stands on this Mac. `why` is scrubbed of the token. */
export type MetricState =
  | { state: "read" }
  | { state: "no_token"; credential: MetricCredential }
  | { state: "broken"; why: string };

/** One recipe's reading, as Bridge draws its block. */
export type MetricReading = {
  id: string;
  title: string;
  /** The last answer that read, kept through a broken fetch. Absent where none ever read. */
  answer?: MetricAnswer;
  /** When `answer` was read. */
  read_at?: string;
  /** The values kept from this Fleet's readings, oldest first, at most 500. Absent is none. */
  kept?: MetricPoint[];
} & MetricState;

/** `GET /projects/project/:project/metrics`: a reading for each recipe the project keeps, in the file's order. */
export type MetricReadings = {
  project: string;
  readings: MetricReading[];
};

/** `POST /projects/project/:project/metrics/try` and `POST /projects/project/:project/metrics`: a recipe to run once, or to run and keep. */
export type TryMetric = { recipe: MetricRecipe };

/** `POST /projects/project/:project/metrics`: the recipe as kept and its first reading. A recipe that cannot read is refused 422 `fleet.metric_unreadable`, unless the only thing missing is this Mac's token. */
export type MetricSaved = {
  project: string;
  recipe: MetricRecipe;
  reading: MetricReading;
};

/** `POST /projects/project/:project/metrics/token`: a token for one credential name, into this Mac's keychain. */
export type SaveMetricToken = {
  credential: string;
  token: string;
};
