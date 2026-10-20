// What `get_settings` answers on the mock Fleet: a settings.json with a few keys changed, one of
// them waiting on a restart and one overridden by an environment variable, so every mark the page
// draws has a row to draw it on. The schema is Fleet's; these rows stand in for it.

import type { SaveSettings, Setting, SettingKind, SettingsList, SettingValue } from "@armada/protocol";

type Extra = Partial<Pick<Setting, "saved" | "applies" | "pending_restart" | "overridden_by_env" | "value" | "tip">>;

/** Each section's group, as `config::settings` names it. */
const GROUP: Record<string, string> = {
  Limits: "Fleet", Timeouts: "Fleet", Retention: "Fleet", Machine: "Fleet", Account: "Fleet", Store: "Fleet", Harness: "Agents", Model: "Agents", Effort: "Agents", Plan: "Agents", Prompts: "Prompts",
  Features: "Features", Editor: "Tools", Terminal: "Tools", Links: "Tools", Bridge: "Appearance",
};

function row(section: string, key: string, title: string, description: string, kind: SettingKind, shipped: SettingValue, extra: Extra = {}): Setting {
  const saved = extra.saved ?? null;
  return {
    key,
    group: GROUP[section] ?? section,
    section,
    title,
    description,
    tip: extra.tip ?? null,
    kind,
    default: shipped,
    value: extra.value ?? saved ?? shipped,
    saved,
    applies: extra.applies ?? "live",
    pending_restart: extra.pending_restart ?? false,
    overridden_by_env: extra.overridden_by_env ?? null,
  };
}

const count = (min: number, max: number, unit?: string): SettingKind => ({ kind: "integer", min, max, ...(unit === undefined ? {} : { unit }) });
const seconds = (min: number, max: number): SettingKind => ({ kind: "seconds", min, max });
const BOOLEAN: SettingKind = { kind: "boolean" };
const TEXT: SettingKind = { kind: "text" };
const PROMPT: SettingKind = { kind: "prompt" };
const LIST: SettingKind = { kind: "text_list" };
const EFFORT: SettingKind = { kind: "choice", options: ["harness default", "low", "medium", "high"] };

const BASELINE = [
  "You are working one step of a job in a git worktree Fleet made for it.",
  "Stay inside the worktree. Run the checks the step names before you hand in evidence.",
  "When the step is done, submit evidence that says what changed and how you know it works.",
].join("\n");

const PROPOSER = [
  "Read the request and propose a job: which workflow it takes, what to call it, and,",
  "where the work is several jobs, the order between them.",
].join("\n");

export const MOCK_SETTINGS: SettingsList = {
  path: "/Users/user/Library/Application Support/Armada/settings.json",
  refused: null,
  settings: [
    row("Limits", "limits.dronesAtOnce", "Drones at once", "How many drones Fleet runs at the same time. A job past this waits for one to finish.", count(1, 8), 2, { saved: 3 }),
    row("Limits", "limits.checksAtOnce", "Checks at once", "How many of a step's checks run at the same time. Ships at half this machine's cores.", count(1, 8), 4),
    row("Limits", "limits.memorySparePercent", "Memory to keep free", "The share of memory Fleet leaves free before it starts another drone.", count(0, 50, "%"), 15),
    row("Limits", "limits.diskFloorGib", "Disk to keep free", "The disk space Fleet leaves free on the worktree volume before it starts another drone.", count(0, 100, "GiB"), 10),
    row("Limits", "limits.portRangeBase", "First port", "Where the ports Fleet lends a job's servers start.", count(1024, 60000), 40000, { applies: "at_restart" }),
    row("Timeouts", "timeouts.checkSeconds", "Check time limit", "How long one check may run before Fleet stops it. A step's own limit wins over this.", seconds(10, 7200), 900),
    row("Timeouts", "timeouts.judgeSeconds", "Judge time limit", "How long one Judge call may take before Fleet gives up on it.", seconds(10, 1800), 120),
    row("Timeouts", "timeouts.commandSeconds", "Command time limit", "How long Fleet works on one command from Bridge before it answers that it gave up.", seconds(5, 300), 15, { applies: "at_restart", saved: 30, value: 15, pending_restart: true }),
    row("Timeouts", "timeouts.bridgeReconnectMs", "Reconnect interval", "How long Bridge waits before it looks for Fleet again after losing it.", count(250, 60000, "ms"), 2000),
    row("Retention", "retention.runLogDays", "Keep run logs", "How long a finished job's logs stay on disk.", count(1, 365, "days"), 30),
    row("Machine", "machine.name", "Machine name", "How the other machines signed in to your account name this one. Blank uses the host's name.", TEXT, ""),
    row("Account", "account.clientId", "Sign-in app", "The client id of the OAuth app Armada signs in with. Blank leaves sign-in unavailable until an app is registered.", TEXT, ""),
    row("Store", "store.url", "Store address", "The address of the central store your machines share. Blank leaves sign-in unavailable until a store is deployed.", TEXT, ""),
    row("Harness", "harness.agent", "Agent", "The program each drone runs.", { kind: "choice", options: ["agent-cli"] }, "agent-cli", { applies: "at_restart" }),
    row("Harness", "harness.binaryPath", "Agent program", "The agent's program, by name on PATH or by absolute path.", TEXT, "agent", { overridden_by_env: "ARMADA_AGENT_BINARY", value: "/opt/agent/bin/agent" }),
    row("Harness", "harness.dronePath", "Drone PATH", "The directories a drone's PATH holds, in order.", LIST, ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]),
    row("Model", "models.roster", "Models", "The models a job or a step may choose from.", LIST, ["opus", "sonnet", "haiku"]),
    row("Model", "models.default", "Default model", "The model a step runs on when nothing names one.", TEXT, "opus"),
    row("Model", "models.session", "Job model", "The model a Job starts on when nothing names one.", TEXT, "opus"),
    row("Model", "models.proposer", "Job proposer model", "The model that reads a request and proposes a job.", TEXT, "haiku", { saved: "sonnet" }),
    row("Effort", "effort.default", "Default effort", "The effort a step runs at when it names none. Harness default passes nothing.", EFFORT, "harness default"),
    row("Effort", "effort.judge", "Judge effort", "The effort each Judge call runs at.", EFFORT, "harness default"),
    row("Plan", "plan.downshiftAt", "Offer a lower default at", "How much of the weekly plan window must be used before the default model is offered a step down until the window resets.", count(1, 100, "percent"), 80),
    row("Plan", "plan.downshiftTo", "Lower default model", "The model the default steps down to when you accept the offer.", TEXT, "sonnet"),
    row("Plan", "plan.contextHandoff", "Large context", "", { kind: "choice", options: ["ask", "carry on automatically", "never"] }, "ask", { tip: "Context limit" }),
    row("Prompts", "prompts.droneBaseline", "Drone baseline", "What every drone is told before its step's own brief.", PROMPT, BASELINE),
    row("Prompts", "prompts.proposer", "Job proposer", "What the job proposer is told before the request.", PROMPT, PROPOSER),
    row("Features", "features.helmCanAct", "Helm can act", "Helm may redirect a job itself. Off, Helm reads one and suggests it.", BOOLEAN, true, { applies: "at_restart" }),
    row("Features", "features.draftPullRequests", "Draft pull requests", "A pull request is opened as a draft unless the repository, the workflow or the job says otherwise.", BOOLEAN, false),
    row("Features", "features.openGuidesFirstTime", "Open guides the first time", "A guide opens once for a piece you have not met.", BOOLEAN, true),
    row("Features", "features.uiSounds", "Interface sounds", "A quiet tick when you move through a list or press a control.", BOOLEAN, true),
    row("Features", "features.notifications", "Notifications", "A macOS notification when a job needs you and Bridge is not in front.", BOOLEAN, true),
    row("Features", "features.reachFolders", "Folders jobs reach", "Folders outside its slots a job may read and write with no card, and folders it may not.", { kind: "json" }, {}, { saved: { block: ["/Users/user/.ssh"] } }),
    row("Editor", "editor.command", "Editor command", "The editor Bridge opens a file in. {file} and {line} are filled in. Empty uses $VISUAL, then $EDITOR, then this Mac's default.", TEXT, ""),
    row("Terminal", "terminal.app", "Terminal", "The terminal Bridge opens a worktree in.", { kind: "choice", options: ["Terminal", "iTerm", "Ghostty", "Warp", "WezTerm", "Custom"] }, "Terminal"),
    row("Terminal", "terminal.command", "Custom terminal command", "Run when Terminal is Custom. {dir} is filled in with the worktree.", TEXT, ""),
    row("Links", "links.openPullRequestsIn", "Open pull requests in", "Where every pull request link in Bridge opens.", { kind: "choice", options: ["armada", "browser"] }, "armada"),
    row("Bridge", "bridge.layout", "Layout", "Settings → Layout writes this.", { kind: "json" }, {}),
  ],
};

/**
 * A save, as Fleet answers it: each change set or removed, the value in force recomputed, and a key
 * that applies at a restart marked as waiting for one. An environment override still wins.
 */
export function savedInto(list: SettingsList, changes: SaveSettings["changes"]): SettingsList {
  return {
    ...list,
    settings: list.settings.map((one) => {
      if (!(one.key in changes)) return one;
      const saved = changes[one.key] ?? null;
      const wanted = saved ?? one.default;
      if (one.applies === "at_restart") {
        return { ...one, saved, pending_restart: JSON.stringify(wanted) !== JSON.stringify(one.value) };
      }
      return { ...one, saved, value: one.overridden_by_env === null ? wanted : one.value };
    }),
  };
}
