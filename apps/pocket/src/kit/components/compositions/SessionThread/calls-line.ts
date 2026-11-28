// The one line a group of tool calls reads as: what is happening, or what happened. Derived from the calls alone, a
// row's tool name and its input as Fleet wrote them ("Edit crates/store/src/audit.rs"), never from what the agent said.
// Several files are "multiple files", with no count; checks are the one place a count stays, since it says how many ran.

type Family = "edit" | "read" | "search" | "run" | "agent" | "web" | "watch" | "ask";

/** [verb running, verb finished] for each family. */
const VERB: Record<
  Family,
  readonly [
    string,
    string,
  ]
> = {
  edit: [
    "Editing",
    "Edited",
  ],
  read: [
    "Reading",
    "Read",
  ],
  search: [
    "Searching",
    "Searched",
  ],
  run: [
    "Running",
    "Ran",
  ],
  agent: [
    "Briefing",
    "Briefed",
  ],
  web: [
    "Reading",
    "Read",
  ],
  watch: [
    "Watching",
    "Watched",
  ],
  ask: [
    "Waiting for you to answer",
    "You answered",
  ],
};

const FAMILY: Record<string, Family> = {
  Edit: "edit",
  Write: "edit",
  MultiEdit: "edit",
  NotebookEdit: "edit",
  Read: "read",
  Grep: "search",
  Glob: "search",
  Bash: "run",
  Agent: "agent",
  Task: "agent",
  WebFetch: "web",
  WebSearch: "web",
  AskUserQuestion: "ask",
};

/** What a shell line runs counts as a check when it is one of these, as `Running 3 checks` says. */
const CHECK =
  /^(armada check|cargo (check|test|clippy)|pnpm (test|exec (vitest|tsc))|npm test|vitest|tsc|pytest)\b/;
/** Commands whose second word names what they do: `cargo check`, `git diff`. */
const WITH_SUBCOMMAND = new Set([
  "git",
  "cargo",
  "pnpm",
  "npm",
  "yarn",
  "npx",
  "bun",
  "armada",
  "gh",
  "make",
  "go",
  "uv",
  "brew",
]);
/** Lines that only set the scene for the one after them. */
const SETTING_THE_SCENE = new Set([
  "cd",
  "pushd",
  "popd",
  "export",
  "set",
  "unset",
  "source",
]);
const AT_MOST = 40;

const toolOf = (
  text: string,
): {
  tool: string;
  detail: string;
} => {
  const at = text.search(/\s/);
  const raw = at === -1 ? text : text.slice(0, at);
  const detail = at === -1 ? "" : text.slice(at).trim();
  // An MCP tool is `mcp__<server>__<tool>`; the tool is what names it.
  return {
    tool: raw.includes("__") ? (raw.split("__").pop() ?? raw) : raw,
    detail,
  };
};

const familyOf = (tool: string): Family | undefined =>
  tool === "start_watch" ? "watch" : FAMILY[tool];

const baseName = (detail: string): string =>
  (detail.split(/\s/, 1)[0] ?? "")
    .split("/")
    .filter((part) => part !== "")
    .pop() ?? "";

/** The segment of a shell line that does the work, after any `cd x &&` or environment assignment. */
const bodyOf = (line: string): string[] => {
  for (const segment of line.split(/&&|\|\||;|\n/)) {
    const words = segment
      .trim()
      .split(/\s+/)
      .filter((word) => word !== "" && !/^\w+=/.test(word));
    if (words[0] !== undefined && !SETTING_THE_SCENE.has(words[0])) return words;
  }
  return [];
};

/** The first one or two words of what a shell line runs. */
export function commandOf(line: string): string {
  const words = bodyOf(line);
  const first = words[0];
  if (first === undefined) return "";
  const name = first.split("/").pop() ?? first;
  const second = words[1];
  return WITH_SUBCOMMAND.has(name) && second !== undefined && !second.startsWith("-")
    ? `${name} ${second}`
    : name;
}

/**
 * What a shell line does by the command it runs, after any `cd x &&`: `grep`, `rg` and `find` search, `cat`, `sed -n`,
 * `head` and `tail` read. `about` is cut from the line: the pattern and places searched, or the last word of a read.
 */
function shellDoing(line: string):
  | {
      family: Family;
      about: string;
    }
  | undefined {
  const all = bodyOf(line);
  const pipe = all.indexOf("|");
  const words = pipe === -1 ? all : all.slice(0, pipe);
  // `cat > file <<EOF` writes: a line that sends its output to a file is no read.
  if (words.some((word) => word.includes(">") && !word.startsWith("2>"))) return undefined;
  const name = words[0]?.split("/").pop() ?? "";
  const family: Family | undefined =
    name === "grep" || name === "rg" || name === "find"
      ? "search"
      : name === "cat" || name === "head" || name === "tail" || (name === "sed" && words[1] === "-n")
        ? "read"
        : undefined;
  if (family === undefined) return undefined;
  if (family === "search") {
    const first = words.slice(1).findIndex((word) => !word.startsWith("-"));
    if (first === -1) return undefined;
    return {
      family,
      about: words.slice(first + 1).join(" "),
    };
  }
  const last = words.length > 1 ? words[words.length - 1] : undefined;
  return last === undefined
    ? undefined
    : {
        family,
        about: last,
      };
}

/** A call's family, and the call with what it is about where a shell line reads or searches. */
function seen(call: { tool: string; detail: string }): {
  family: Family | undefined;
  call: {
    tool: string;
    detail: string;
  };
} {
  if (call.tool === "Bash") {
    const doing = shellDoing(call.detail);
    if (doing !== undefined)
      return {
        family: doing.family,
        call: {
          tool: call.tool,
          detail: doing.about,
        },
      };
  }
  return {
    family: familyOf(call.tool),
    call,
  };
}

const isCheck = (line: string): boolean => CHECK.test(bodyOf(line).join(" "));

const clip = (said: string): string =>
  said.length > AT_MOST ? `${said.slice(0, AT_MOST - 1)}…` : said;

/** The target of one family's calls: a file, several files, a pattern, a command. */
function aboutOf(
  family: Family,
  calls: readonly {
    tool: string;
    detail: string;
  }[],
): string {
  const details = [
    ...new Set(calls.map((one) => one.detail)),
  ];
  const last = calls[calls.length - 1]!;
  switch (family) {
    case "edit":
    case "read": {
      const files = new Set(calls.map((one) => baseName(one.detail)).filter((name) => name !== ""));
      return files.size > 1 ? "multiple files" : (files.values().next().value ?? "a file");
    }
    case "search":
      return clip(last.detail);
    case "run": {
      if (details.length > 1)
        return details.every(isCheck) ? `${details.length} checks` : "multiple commands";
      return clip(commandOf(details[0] ?? ""));
    }
    case "agent":
      return "an agent";
    case "web":
      return "the web";
    case "watch":
      return clip(last.detail);
    case "ask":
      return "";
  }
}

/**
 * The line for a group of calls. The last call that Bridge knows by name says what the group is doing, and the calls of its
 * kind in the group say how many places it touched; a group of nothing known reads as its last tool's name.
 */
export function callsLine(
  rows: readonly {
    text: string;
  }[],
  working: boolean,
): string {
  const calls = rows.map((row) => seen(toolOf(row.text)));
  const last = calls
    .map((one) => one.family)
    .filter((family): family is Family => family !== undefined)
    .pop();
  if (last === undefined) return calls[calls.length - 1]?.call.tool ?? "";
  const of = calls.filter((one) => one.family === last).map((one) => one.call);
  // A briefed agent is named by its description, which is already a phrase.
  const described = last === "agent" ? of[of.length - 1]!.detail : "";
  if (described !== "") return clip(described);
  const verb = VERB[last][working ? 0 : 1];
  const about = aboutOf(last, of);
  return about === "" ? verb : `${verb} ${about}`;
}

/** One call on a line of its own: a tool with no verb says its whole call, since its name alone says nothing of what ran. */
export function callLine(
  row: {
    text: string;
  },
  working: boolean,
): string {
  return seen(toolOf(row.text)).family === undefined
    ? clip(row.text)
    : callsLine(
        [
          row,
        ],
        working,
      );
}

/** What a call does, by the tile it takes in a run's card: its glyph and hue. */
export type CallKind = "read" | "run" | "edit" | "search" | "agent" | "web" | "message" | "other";

/** The kind a call's tile draws. A watch or an ask is drawn plain. */
export function kindOf(text: string): CallKind {
  const family = seen(toolOf(text)).family;
  return family === undefined || family === "watch" || family === "ask" ? "other" : family;
}

/** What a running call works on, for the card's running box: a command's first line after `$ `, else the path or pattern. */
export function workingOn(text: string): string | undefined {
  const call = toolOf(text);
  const detail = call.detail.trim();
  if (detail === "") return undefined;
  const first = detail.split("\n", 1)[0] ?? "";
  const family = familyOf(call.tool);
  if (family === "run") return `$ ${first}`;
  return family === "read" || family === "edit" || family === "search" ? first : undefined;
}
