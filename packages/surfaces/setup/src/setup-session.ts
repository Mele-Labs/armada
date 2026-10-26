// Set up with a session: a Session started in the repository being set up, whose first message
// asks it to write armada.yml with the owner and carries the reading Setup already holds, so the
// Session starts from it rather than scanning again. No React, so the message is a unit test.

import { placePhrase, type RunsPlace } from "@armada/components";
import type { ManifestProposal, Outcome, Provenance, SendSessionMessage } from "@armada/protocol";

import type { SetupOpen } from "./setup-view";
import { stateOf } from "./setup-view";

/**
 * **The rules are stated inline as well as named.** `onboard-repo` is not shipped by Armada: it is
 * whatever skill the machine's own Claude Code holds, and a Session in another repository may not
 * load it. `armada manifest config verify`, the step the skill ends on, is v1's and this binary has
 * no such verb, so the last rule names the one that runs a Check.
 *
 * **The place is named, never asked.** The Session cannot read or set it (`get_run_memory_place` is
 * closed to agents), so Bridge reads it and the message says where the file is kept.
 */
function instructionOf(place: RunsPlace | undefined): string {
  const inRepository = place?.id === "repository";
  const kept = place === undefined ? "in your Kit" : placePhrase(place);
  const where = inRepository
    ? [
        `- This repository's armada.yml is kept ${kept}. Do not ask me where to keep it.`,
        "- Write armada.yml at the repository's root, on this Job's branch, where this Job starts. It lands with this Job's pull request. A workspace that needs its own writes its armada.yml in its own directory.",
        "- Before writing it, ask me whether to add armada.yml to this repository's .gitignore, and do what I answer. Tell me with the question that an ignored armada.yml is missing from the worktrees Jobs work in, so `armada check` and `armada run` fail there, while Fleet's own Checks still run.",
        "- Write only armada.yml files, .gitignore files and .git/info/exclude, with the Write or Edit tool. Fleet refuses anything else here until the root's armada.yml is served.",
      ]
    : [
        `- This repository's armada.yml is kept ${kept}. Do not ask me where to keep it.`,
        `- Save it by calling the save_kit_manifest tool with this repository's root and the whole file, which keeps it ${kept} and puts nothing in the repository. If that tool refuses, tell me what it said and stop; never write the file into the repository instead.`,
        "- Write no file in the repository.",
      ];
  return [
    "Set up this repository's armada.yml with me, using the onboard-repo skill. Where that skill is not loaded, keep its rules, which are these:",
    "",
    "- Ask me one question at a time, and wait for my answer before the next.",
    "- Say where every guess came from: the file and the key it was read from.",
    "- Write nothing before I confirm it. Show me the whole file first.",
    ...where,
    `- End on a real run, never a claim. Run \`armada check <name>\` for each Check you wrote, from ${inRepository ? "the directory its armada.yml is in" : "the repository's root"}, and show me what each said. Not done until they pass.`,
  ].join("\n");
}

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

/** The Session's first message: what to do and by which rules, then the reading, a line a workspace. `place` is where the repository's configuration is kept, `undefined` where Fleet could not say. */
export function setupMessageOf(held: SetupOpen, place: RunsPlace | undefined): string {
  return [instructionOf(place), "", READING, "", ...held.proposals.proposals.map((one) => workspaceLine(held, one))].join("\n");
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

/** Start a Session in `root` and send it the message, naming `place`. A refusal at either call comes back as its words. */
export async function startSetupSession(
  held: SetupOpen,
  root: string,
  fleet: SetupSessionFleet,
  say: (outcome: Outcome) => string,
  place: RunsPlace | undefined,
): Promise<SetupSessionAnswer> {
  const started = await fleet.startSession(setupTitleOf(root), root);
  if (!started.ok) return { ok: false, saying: `Job not started: ${say(started.outcome) || "Fleet refused it"}` };
  const { id } = started.value;
  const sent = await fleet.sendSessionMessage({ session_id: id, text: setupMessageOf(held, place) });
  if (!sent.ok) return { ok: false, id, saying: `Job started, but the reading was not sent: ${say(sent.outcome) || "Fleet refused it"}` };
  return { ok: true, id };
}
