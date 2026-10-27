// The one line a group of tool calls reads as: what is happening, or what happened. Derived from the calls alone, a
// row's tool name and its input as Fleet wrote them ("Edit crates/store/src/audit.rs"), never from what the agent said.
// Several files are "multiple files", with no count; checks are the one place a count stays, since it says how many ran.

type Family = "edit" | "read" | "search" | "run" | "agent" | "web" | "watch" | "ask";

/** [verb running, verb finished] for each family. */
const VERB: Record<Family, readonly [string, string]> = {
  edit: ["Editing", "Edited"],
  read: ["Reading", "Read"],
  search: ["Searching", "Searched"],
  run: ["Running", "Ran"],
  agent: ["Briefing", "Briefed"],
  web: ["Reading", "Read"],
  watch: ["Watching", "Watched"],
  ask: ["Waiting for you to answer", "You answered"],
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
const CHECK = /^(armada check|cargo (check|test|clippy)|pnpm (test|exec (vitest|tsc))|npm test|vitest|tsc|pytest)\b/;
/** Commands whose second word names what they do: `cargo check`, `git diff`. */
const WITH_SUBCOMMAND = new Set(["git", "cargo", "pnpm", "npm", "yarn", "npx", "bun", "armada", "gh", "make", "go", "uv", "brew"]);
/** Lines that only set the scene for the one after them. */
const SETTING_THE_SCENE = new Set(["cd", "pushd", "popd", "export", "set", "unset", "source"]);
const AT_MOST = 40;

const toolOf = (text: string): { tool: string; detail: string } => {
  const at = text.search(/\s/);
  const raw = at === -1 ? text : text.slice(0, at);
  const detail = at === -1 ? "" : text.slice(at).trim();
  // An MCP tool is `mcp__<server>__<tool>`; the tool is what names it.
  return { tool: raw.includes("__") ? (raw.split("__").pop() ?? raw) : raw, detail };
};

const familyOf = (tool: string): Family | undefined => (tool === "start_watch" ? "watch" : FAMILY[tool]);

const baseName = (detail: string): string => (detail.split(/\s/, 1)[0] ?? "").split("/").filter((part) => part !== "").pop() ?? "";

/** The segment of a shell line that does the work, after any `cd x &&` or environment assignment. */
const bodyOf = (line: string): string[] => {
  for (const segment of line.split(/&&|\|\||;|\n/)) {
    const words = segment.trim().split(/\s+/).filter((word) => word !== "" && !/^\w+=/.test(word));
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
  return WITH_SUBCOMMAND.has(name) && second !== undefined && !second.startsWith("-") ? `${name} ${second}` : name;
}

const isCheck = (line: string): boolean => CHECK.test(bodyOf(line).join(" "));

const clip = (said: string): string => (said.length > AT_MOST ? `${said.slice(0, AT_MOST - 1)}…` : said);

/** The target of one family's calls: a file, several files, a pattern, a command. */
function aboutOf(family: Family, calls: readonly { tool: string; detail: string }[]): string {
  const details = [...new Set(calls.map((one) => one.detail))];
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
      if (details.length > 1) return details.every(isCheck) ? `${details.length} checks` : "multiple commands";
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
export function callsLine(rows: readonly { text: string }[], working: boolean): string {
  const calls = rows.map((row) => toolOf(row.text));
  const last = calls.map((one) => familyOf(one.tool)).filter((family): family is Family => family !== undefined).pop();
  if (last === undefined) return calls[calls.length - 1]?.tool ?? "";
  const of = calls.filter((one) => familyOf(one.tool) === last);
  // A briefed agent is named by its description, which is already a phrase.
  const described = last === "agent" ? of[of.length - 1]!.detail : "";
  if (described !== "") return clip(described);
  const verb = VERB[last][working ? 0 : 1];
  const about = aboutOf(last, of);
  return about === "" ? verb : `${verb} ${about}`;
}
