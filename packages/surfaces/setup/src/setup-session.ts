// Set up with a session: a Session started in the repository being set up, whose first message
// asks it to write armada.yml with the owner and carries the reading Setup already holds, so the
// Session starts from it rather than scanning again. No React, so the message is a unit test.

import type { ManifestProposal, Outcome, Provenance, SendSessionMessage } from "@armada/protocol";

import type { SetupOpen } from "./setup-view";
import { stateOf } from "./setup-view";

/**
 * **The rules are stated inline as well as named.** `onboard-repo` is not shipped by Armada: it is
 * whatever skill the machine's own Claude Code holds, and a Session in another repository may not
 * load it. `armada manifest config verify`, the step the skill ends on, is v1's and this binary has
 * no such verb, so the last rule names the one that runs a Check.
 */
const INSTRUCTION = [
  "Set up this repository's armada.yml with me, using the onboard-repo skill. Where that skill is not loaded, keep its rules, which are these:",
  "",
  "- Ask me one question at a time, and wait for my answer before the next.",
  "- Say where every guess came from: the file and the key it was read from.",
  "- Write nothing before I confirm it. Show me the whole file first.",
  "- Before you write anything, ask me whose this armada.yml is, in these words: \"Do your colleagues use Armada on this repository? If they do not, I can keep this armada.yml as yours: it stays in your Kit and nothing is committed to the repository, so your colleagues never see it, and it runs only for you. Or I can write it as the repository's: it goes at the workspace root and is committed with this Job, so your colleagues see a new file in the pull request, and anyone who does use Armada here runs it too. Yours, or the repository's?\"",
  "- Write it where I answered. If it is the repository's, write armada.yml here, at the repository's root, where this Job starts. A workspace that needs its own writes its armada.yml in its own directory. If it is mine, write it to ~/.armada/manifests/<owner--name>.yml, the remote's owner and name joined by two dashes, or the path where there is no remote, and put nothing in the repository. If Fleet refuses that write, say so and stop; never write it into the repository instead.",
  "- If it is the repository's, before writing it, ask me whether to add armada.yml to this repository's .gitignore, and do what I answer. Tell me with the question that an ignored armada.yml is missing from the worktrees Jobs work in, so `armada check` and `armada run` fail there, while Fleet's own Checks still run.",
  "- Write only armada.yml files, .gitignore files and .git/info/exclude, with the Write or Edit tool. Fleet refuses anything else here until the root's armada.yml is served.",
  "- End on a real run, never a claim. Run `armada check <name>` for each Check you wrote, from the directory its armada.yml is in (the repository's root, for one kept in Kit), and show me what each said. Not done until they pass.",
].join("\n");

const READING =
  "Bridge has already scanned this checkout. Start from what it read rather than scanning again. Each workspace, the files it read there, and what Setup proposed:";

function from(provenance: Provenance): string {
  const where = [provenance.file, provenance.key].filter((part) => part !== undefined).join(" ");
  return where === "" ? provenance.source : `${where}, ${provenance.source}`;
}

const lines = (rows: readonly { name: string; run: string; provenance: Provenance }[]): string =>
  rows.map((row) => `${row.name} \`${row.run}\` (${from(row.provenance)})`).join("; ");

/** One workspace, one line: its files and its state, then why it is thin, then what is proposed. */
function workspaceLine(held: SetupOpen, proposal: ManifestProposal): string {
  const scanned = held.scan.workspaces.find((one) => one.dir === proposal.dir);
  const files = scanned === undefined ? [] : [...scanned.manifests, ...scanned.lockfiles].map((one) => one.file);
  const read = files.length === 0 ? "no files read" : `read ${files.join(", ")}`;
  const parts = [`- \`${proposal.dir}\` (${read}): ${stateOf(proposal, false, held.marks[proposal.dir])}.`];
  if (scanned?.evidence === "not_followed") parts.push("Nothing here could be read.");
  else if (scanned?.evidence === "thin") parts.push("No file here names a script, so anything proposed is convention.");
  if (proposal.checks.length > 0) parts.push(`Checks: ${lines(proposal.checks)}.`);
  if (proposal.commands.length > 0) parts.push(`Commands: ${lines(proposal.commands)}.`);
  return parts.join(" ");
}

/** The Session's first message: what to do and by which rules, then the reading, a line a workspace. */
export function setupMessageOf(held: SetupOpen): string {
  return [INSTRUCTION, "", READING, "", ...held.proposals.proposals.map((one) => workspaceLine(held, one))].join("\n");
}

/** What the Session is called: the repository's folder, which is what a person knows it by. */
export function setupTitleOf(root: string): string {
  return `Set up ${root.split("/").filter((part) => part !== "").pop() ?? root}`;
}

/** The two calls a start makes on Fleet's seam. Desktop's `BridgeApi` has both. */
export type SetupSessionFleet = {
  startSession: (title?: string, root?: string) => Promise<{ ok: true; value: { id: string } } | { ok: false; outcome: Outcome }>;
  sendSessionMessage: (send: SendSessionMessage) => Promise<{ ok: true } | { ok: false; outcome: Outcome }>;
};

/** `id` on a refusal is a Session that started and did not take the message: it is still opened. */
export type SetupSessionAnswer = { ok: true; id: string } | { ok: false; saying: string; id?: string };

/** Start a Session in `root` and send it the message. A refusal at either call comes back as its words. */
export async function startSetupSession(
  held: SetupOpen,
  root: string,
  fleet: SetupSessionFleet,
  say: (outcome: Outcome) => string,
): Promise<SetupSessionAnswer> {
  const started = await fleet.startSession(setupTitleOf(root), root);
  if (!started.ok) return { ok: false, saying: `Job not started: ${say(started.outcome) || "Fleet refused it"}` };
  const { id } = started.value;
  const sent = await fleet.sendSessionMessage({ session_id: id, text: setupMessageOf(held) });
  if (!sent.ok) return { ok: false, id, saying: `Job started, but the reading was not sent: ${say(sent.outcome) || "Fleet refused it"}` };
  return { ok: true, id };
}
