// Commit to repository, Share and Copy on one piece of the person's own: a tile kept for a repository, a workflow or a review tool
// in Kit. Commit copies it onto the open Job's branch; Share saves the package Fleet makes, and Copy puts the same text on the
// clipboard. `docs/design/who-sees-each-piece-of-configuration.md`.
//
// **A refusal is said in Fleet's words**, beside the buttons, and an id the branch already holds offers Replace. A failure that is
// not a refusal, such as a route this Fleet does not serve yet, is the window's toast, which names the pull request that builds it.

import { useState } from "react";
import { Button } from "@armada/components";
import type { Outcome, PieceToCommit } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { raiseOutcome } from "../tell";
import "./sharing.css";

/** What the line beside the buttons says, and whether Replace is offered. */
export type ActSaid = { said: string; replace?: true } | undefined;

/** What a failed answer leaves beside the buttons: Fleet's words for a refusal, and nothing for a failure the toast says. */
export function failedSaid(outcome: Outcome): ActSaid {
  if (outcome.ok) return undefined;
  if (outcome.why !== "refused") {
    raiseOutcome(outcome);
    return undefined;
  }
  return { said: refusalWords(outcome), ...(outcome.error.code === "fleet.commit_exists" ? { replace: true as const } : {}) };
}

/** Commit, with Replace where the branch already holds the id. */
export async function commitPiece(repository: string, sessionId: string | undefined, piece: PieceToCommit, overwrite = false): Promise<ActSaid> {
  const answer = await window.armada.commitToRepository({ repository, piece, ...(sessionId === undefined ? {} : { session_id: sessionId }), ...(overwrite ? { overwrite } : {}) });
  if (!answer.ok) return failedSaid(answer.outcome);
  return { said: answer.value.commit === undefined ? `Already on ${answer.value.branch}` : `Committed to ${answer.value.branch}` };
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

/** The three acts on one piece. `sessionId` is the Job open now, whose branch a commit lands on. */
export function PieceActs({ repository, sessionId, piece }: { repository: string; sessionId: string | undefined; piece: PieceToCommit }) {
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
  return (
    <div className="piece-acts">
      <div className="piece-acts__row">
        <Button variant="secondary" size="sm" disabled={busy} onClick={act(() => commitPiece(repository, sessionId, piece))}>
          Commit to repository
        </Button>
        <Button variant="secondary" size="sm" disabled={busy} onClick={act(() => sharePiece(repository, piece, "share"))}>
          Share
        </Button>
        <Button variant="secondary" size="sm" disabled={busy} onClick={act(() => sharePiece(repository, piece, "copy"))}>
          Copy
        </Button>
      </div>
      {said === undefined ? null : (
        <p className="piece-acts__said" role="status">
          {said.said}
          {said.replace === true ? (
            <Button variant="ghost" size="sm" disabled={busy} onClick={act(() => commitPiece(repository, sessionId, piece, true))}>
              Replace
            </Button>
          ) : null}
        </p>
      )}
    </div>
  );
}
