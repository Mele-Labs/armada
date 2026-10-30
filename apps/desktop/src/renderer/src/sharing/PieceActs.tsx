// Commit to repository, Share and Copy on one piece of the person's own: a tile kept for a repository, a workflow or a review tool
// in Kit. Commit starts a Job of its own that commits the piece alone and opens a pull request for it; Share saves the package Fleet
// makes, and Copy puts the same text on the clipboard. `docs/design/who-sees-each-piece-of-configuration.md`,
// `.claude/decisions/2026-10-21-commit-starts-a-job-of-its-own.md`.
//
// **A refusal is said in Fleet's words**, beside the buttons, and an id main already holds offers Replace. A failure that is
// not a refusal, such as a route this Fleet does not serve yet, is the window's toast, which names the pull request that builds it.

import { useState } from "react";
import { Copy, GitCommitHorizontal, Share } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { CommittedToRepository, Outcome, PieceToCommit } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { askToOpenSessionInCockpit } from "../open-session";
import { raiseOutcome } from "../tell";
import "./sharing.css";

/** What the line beside the buttons says: words, with Replace where main holds the id, or the Job a commit started. */
export type ActSaid = { said: string; replace?: true; job?: CommittedToRepository } | undefined;

/** What a failed answer leaves beside the buttons: Fleet's words for a refusal, and nothing for a failure the toast says. */
export function failedSaid(outcome: Outcome): ActSaid {
  if (outcome.ok) return undefined;
  if (outcome.why !== "refused") {
    raiseOutcome(outcome);
    return undefined;
  }
  return { said: refusalWords(outcome), ...(outcome.error.code === "fleet.commit_exists" ? { replace: true as const } : {}) };
}

/** Commit, with Replace where main already holds the id. No Job need be open: Fleet starts one for the commit. */
export async function commitPiece(repository: string, piece: PieceToCommit, overwrite = false): Promise<ActSaid> {
  const answer = await window.armada.commitToRepository({ repository, piece, ...(overwrite ? { overwrite } : {}) });
  if (!answer.ok) return failedSaid(answer.outcome);
  const job = answer.value;
  return { said: "Job started", job };
}

/** Share writes the package where the person picks; Copy puts its text on the clipboard. */
export async function sharePiece(repository: string, piece: PieceToCommit, how: "share" | "copy"): Promise<ActSaid> {
  const answer = await window.armada.sharePackage({ pieces: [{ repository, piece }] });
  if (!answer.ok) return failedSaid(answer.outcome);
  if (how === "copy") {
    await navigator.clipboard.writeText(answer.value.text);
    return { said: "Copied" };
  }
  const saved = await window.armada.savePackageFile(answer.value.file_name, answer.value.text);
  return saved.saved ? { said: `Saved ${saved.name}` } : saved.why === "cancelled" ? undefined : { said: "Not saved" };
}

/**
 * The three acts on one piece. **With `icons` each is a glyph with its tooltip**, in one group a toolbar draws beside its others; the sentence they leave drops to a line of its own under the toolbar.
 */
export function PieceActs({ repository, piece, icons = false }: { repository: string; piece: PieceToCommit; icons?: boolean }) {
  const [said, setSaid] = useState<ActSaid>();
  const [busy, setBusy] = useState(false);
  const act = (run: () => Promise<ActSaid>) => async () => {
    if (busy) return;
    setBusy(true);
    setSaid(undefined);
    try {
      setSaid(await run());
    } finally {
      setBusy(false);
    }
  };
  const job = said?.job;
  const acts = [
    { label: "Commit to repository", Glyph: GitCommitHorizontal, run: () => commitPiece(repository, piece) },
    { label: "Share", Glyph: Share, run: () => sharePiece(repository, piece, "share") },
    { label: "Copy", Glyph: Copy, run: () => sharePiece(repository, piece, "copy") },
  ];
  return (
    <div className="piece-acts" data-icons={icons || undefined}>
      <div className="piece-acts__row" {...(icons ? { role: "group", "aria-label": "Where it is kept" } : {})}>
        {acts.map(({ label, Glyph, run }) =>
          icons ? (
            <Tooltip key={label} label={label}>
              <Button variant="secondary" size="sm" iconOnly aria-label={label} disabled={busy} onClick={act(run)}>
                <Glyph size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          ) : (
            <Button key={label} variant="secondary" size="sm" disabled={busy} onClick={act(run)}>
              {label}
            </Button>
          ),
        )}
      </div>
      {said === undefined ? null : (
        <p className="piece-acts__said" role="status">
          {said.said}
          {said.replace === true ? (
            <Button variant="ghost" size="sm" disabled={busy} onClick={act(() => commitPiece(repository, piece, true))}>
              Replace
            </Button>
          ) : null}
          {job === undefined ? null : (
            <Button variant="ghost" size="sm" onClick={() => askToOpenSessionInCockpit(job.session_id)}>
              Open the Job
            </Button>
          )}
        </p>
      )}
    </div>
  );
}
