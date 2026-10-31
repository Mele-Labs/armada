// A judge on demand. One Job, Switch sign-in to session tokens, holding a plan waiting on him (v2), a pull request with three files, two commits, a
// document, a walk page, a sketch, a command and a subagent: every row of its Ledger tab has a Judge, and so has the Thread's head and the plan card. Pressing one
// asks what to verify and reads for two seconds. The pull request, the plan and the document come back with notes, a commit with none, and the walk page, the
// sketch and the command with one note on the whole of them. Mock only: the answers are `../judge-fleet.ts`. The walk `judgeOnDemand` plays it.

import type { PullRequestPage } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow, WorkGroup } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";

import { jobOpens } from "../../job-panes/store";
import type { PanelReads, PullRequestRead } from "../../shared-panel/reads";
import { mockAreas } from "../cockpit-areas";
import { holding } from "../holding";
import { mockJudge, MIDDLEWARE, TOKENS } from "../judge-fleet";
import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { arrow, box, scene } from "../scenes";
import { sessionsStore } from "../sessions/script";
import { sharedPanelReads } from "../shared-panel-reads";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const JOB = "judge-1";
const PULL = 2418;
const PULL_TITLE = "Move sign-in to session tokens";
const BRANCH = "armada/session-tokens";
const ADDRESS = `https://git.example/armada/pull/${PULL}`;
const DOC_PATH = "docs/design/session-tokens.html";
const DOC_TITLE = "Session tokens";
const WALK_URL = "https://example.com/walk/sign-in-after-the-switch";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const task = (id: string, text: string) => ({ id, text, state: "done" as const, seen: true });

const GROUPS: WorkGroup[] = [
  { id: "g1", agent: "rust-engineer", title: "Move sign-in to session tokens", paths: ["crates/api/src/auth"], tasks: [task("T1", "Issue a session token at sign-in"), task("T2", "Check the token on every request")] },
  { id: "g2", agent: "bridge-engineer", title: "Show the signed-in state", paths: ["apps/desktop/src/main"], tasks: [task("T3", "Read the token from the sign-in reply")] },
];

const SKETCH = scene([box("client", 0, 0, "Client", "Signs in", { icon: "globe", step: 1 }), box("api", 360, 0, "API", "Issues a session token", { icon: "server", step: 2 })], [arrow("client", "api", "code", undefined, 1)]);

const you = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "you" }, text });
const agent = (id: string, at: string, text: string): SessionRow => ({ id, at, kind: "message", from: { kind: "agent" }, text });

const ATTACHMENTS: SessionAttachment[] = [
  { kind: "branch", name: BRANCH, slot: 3 },
  { kind: "pull_request", number: PULL, title: PULL_TITLE, branch: BRANCH, address: ADDRESS, checks: { state: "pending" }, state: "open", auto: false },
  { kind: "issue", number: "2367", title: "Sign-in should not depend on a cookie", address: "https://forge.invalid/issues/2367", state: "open" },
  { kind: "artifact", form: "document", id: DOC_PATH, title: DOC_TITLE, revisions: [DOC_PATH] },
  { kind: "artifact", form: "window", id: WALK_URL, title: "Sign-in after the switch" },
  { kind: "sketch", id: "sketch:Flow", title: "Sign-in flow", by: "agent", drawing: SKETCH },
  { kind: "subagent", id: "sub-1", task: "Read where the old cookie is set", state: "done", agentType: "explorer" },
];

const job: Session = {
  id: JOB,
  title: "Switch sign-in to session tokens",
  manifestId: OWNER,
  cwd: ROOT,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(1),
  standing: "idle",
  rows: [
    you("j-1", "14:02:00", "Move sign-in from the cookie to session tokens."),
    { id: "j-2", at: "14:20:10", kind: "tool", text: 'Bash git commit -m "Issue a session token at sign-in"' },
    { id: "j-3", at: "14:31:45", kind: "tool", text: 'Bash git commit -m "Check the token on every request"' },
    { id: "j-4", at: "14:33:02", kind: "ran", command: "cargo test -p api auth", code: 0, tail: "test result: ok. 14 passed" },
    agent("j-5", "14:36:00", `The pull request is open. Plan v2 covers what the review asked for: approve it, or tell me what to change.`),
  ],
  attachments: ATTACHMENTS,
  waitingFor: [planCall(2, ago(1))],
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 0 },
    plan: {
      version: 2,
      scope: ["crates/api/src/auth", "apps/desktop/src/main"],
      approved: false,
      after: "j-5",
      summary: "Sign-in issues a session token instead of setting a cookie, the API checks it on every request, and the desktop app reads it from the sign-in reply.",
      groups: GROUPS.map((group, at) => ({ ...group, gives: ["Sign-in issues a session token", "The desktop app reads the token from the reply"][at]! })),
    },
  },
};

const DOC = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${DOC_TITLE}</title>
<link rel="stylesheet" href="armada-kit.css">
</head>
<body>
<header>
  <h1>${DOC_TITLE}</h1>
  <p>Sign-in moves from a cookie to a session token the API issues.</p>
</header>
<section>
  <h2>How a token lives</h2>
  <p>A session token lasts a day, then the client signs in again.</p>
  <p>A token issued before the switch is read once and exchanged for a session token.</p>
</section>
</body>
</html>
`;

const patch = (...lines: string[]) => lines.join("\n");
const PATCHES: Record<string, string> = {
  [MIDDLEWARE]: patch(
    "@@ -80,7 +80,12 @@ pub async fn require_session(",
    "     let header = request.headers().get(AUTHORIZATION);",
    "     let Some(header) = header else {",
    "         return Err(AuthError::Missing);",
    "     };",
    "-    let token = Token::parse_legacy(header)?;",
    "+    let token = SessionToken::parse(header)?;",
    "+    if token.expired(now()) {",
    "+        return Err(AuthError::Expired);",
    "+    }",
    "+    let session = sessions.lookup(&token).await?;",
    "+    request.extensions_mut().insert(session);",
    "     next.run(request).await",
    " }",
  ),
  [TOKENS]: patch(
    "@@ -38,6 +38,7 @@ impl SessionToken {",
    "     pub fn issue(config: &Config) -> Self {",
    "         Self {",
    "             id: Uuid::new_v4(),",
    "-            issued: Utc::now(),",
    "+            ttl: Duration::from_secs(config.ttl),",
    "+            issued: Instant::now(),",
    "         }",
    "     }",
  ),
  "apps/desktop/src/main/sign-in.ts": patch(
    "@@ -12,3 +12,4 @@ export async function signIn(code: string) {",
    '   const reply = await fetch(`${base}/sign-in`, { method: "POST", body: code });',
    "-  return reply.json();",
    "+  const { token } = await reply.json();",
    "+  return token;",
    " }",
  ),
};

/** The pull request as the forge reads it, for the panel's Files tab. */
function pulled(): PullRequestRead {
  const files = Object.keys(PATCHES);
  const page: PullRequestPage = {
    manifest_id: OWNER,
    number: PULL,
    base: "main",
    author: "claude",
    created_at: ago(30),
    additions: 12,
    deletions: 3,
    mergeable: "mergeable",
    review: "none",
    approvals: 0,
    body: "Sign-in issues a session token and the API checks it on every request.",
    conversation: [],
    checks: [{ name: "build", state: "running", address: `${ADDRESS}/checks?check_run_id=1` }],
    files: files.map((path) => ({ path, status: "modified" as const, additions: path === MIDDLEWARE ? 6 : 2, deletions: 1 })),
    commits: [
      { sha: "a41c9e2", headline: "Issue a session token at sign-in", author: "claude", at: ago(25) },
      { sha: "b7d02f1", headline: "Check the token on every request", author: "claude", at: ago(17) },
    ],
  };
  return { repository: "mele-labs/armada", root: ROOT, facts: { kind: "pull_request", number: PULL, title: PULL_TITLE, branch: BRANCH, address: ADDRESS, checks: { state: "pending" }, state: "open", auto: false, body: page.body ?? "" }, page };
}

function reads(): PanelReads {
  const base = sharedPanelReads({ lines: [] });
  return {
    ...base,
    pullRequest: async (item) => (item.number === PULL ? pulled() : base.pullRequest(item)),
    pullRequestPatches: async (item) =>
      item.number === PULL
        ? { ok: true, files: Object.entries(PATCHES).map(([path, text]) => ({ manifest_id: OWNER, number: PULL, path, patch: text })) }
        : ((await base.pullRequestPatches?.(item)) ?? { ok: false, said: "The forge could not find this pull request." }),
  };
}

function sessions() {
  const store = sessionsStore([job], [], new Map(), []);
  const judge = mockJudge(store);
  // The Job opens with a Ledger tab beside its thread, which lists what it made with a Judge on each row.
  jobOpens(JOB, "ledger");
  return {
    ...store,
    readArtifact: async (id: string, path: string) =>
      path === DOC_PATH ? { ok: true as const, bytes: new TextEncoder().encode(DOC), type: "text/html" } : (store.readArtifact?.(id, path) ?? { ok: false as const, why: "unreadable" as const }),
    dispose: () => {
      judge.dispose();
      store.dispose();
    },
  };
}

const base = holding("judge-on-demand", "A Job with a plan, a pull request, commits, a document, a walk page, a sketch and a command, each with a Judge that reads on a press and leaves notes inside it");

export const judgeOnDemandScenario: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions, areas: () => mockAreas("/armada"), sharedPanel: reads },
};
