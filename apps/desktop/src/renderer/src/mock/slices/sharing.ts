// A piece committed to a Job's branch and a package made, read and added, answered from memory for the life of the window. A commit
// with no Job is refused as Fleet refuses it. A package this window made reads back as the pieces it was made from; any other text is
// refused as malformed, so a scenario that pastes one supplies `readPackage` and `addPackage` through `Scenario.behaves`.

import type { PackagePiece, PieceToShare, UnboundPiece, WireError } from "@armada/protocol";

import type { SharingAnswer, SharingApi } from "../../../../shared/api/sharing";
import type { Slice } from "../fake-context";

const refusal = (code: string, message: string): SharingAnswer<never> => {
  const error: WireError = { code, message, run_id: "run-mock", fields: {}, chain: [] };
  return { ok: false, outcome: { ok: false, why: "refused", error } };
};

const idOf = ({ piece }: PieceToShare): string => (piece.kind === "workflow" ? piece.workflow_id : piece.kind === "review_tool" ? piece.id : [...(piece.job ?? []), ...(piece.deck ?? [])].join("-"));

const kindOf = ({ piece }: PieceToShare): PackagePiece["kind"] => (piece.kind === "flight_controls" ? "tile" : piece.kind);

/** The text a package of these pieces reads as here: one table each, naming what it holds. */
function textOf(pieces: readonly PieceToShare[]): string {
  return ["version = 1", ...pieces.flatMap((one) => ["", "[[piece]]", `kind = "${kindOf(one)}"`, `id = "${idOf(one)}"`])].join("\n") + "\n";
}

export const sharing: Slice<SharingApi, Record<string, never>> = {
  name: "sharing",
  state: {},
  api: () => {
    const made = new Map<string, PackagePiece[]>();
    const unbound: UnboundPiece[] = [];
    return {
      commitToRepository: async (body) => {
        if (body.session_id === undefined) return refusal("fleet.commit_no_job", "Open a Job in this repository to commit to its branch");
        const id = body.piece.kind === "workflow" ? body.piece.workflow_id : body.piece.kind === "review_tool" ? body.piece.id : "flight-controls";
        const path = body.piece.kind === "workflow" ? `.armada/workflows/${id}.toml` : body.piece.kind === "review_tool" ? `.armada/review-tools/${id}.md` : ".armada/flight-controls.toml";
        return { ok: true, value: { repository: body.repository, branch: `mock/${body.session_id}`, paths: [path], commit: "4f2c9a1" } };
      },
      sharePackage: async (body) => {
        const text = textOf(body.pieces);
        made.set(text, body.pieces.map((one) => ({ kind: kindOf(one), id: idOf(one), name: idOf(one), commands: [], repository: one.repository, exists: true })));
        return { ok: true, value: { file_name: `${idOf(body.pieces[0]!)}.armada.toml`, text } };
      },
      readPackage: async (text) => {
        const pieces = made.get(text);
        return pieces === undefined ? refusal("fleet.share_malformed", "This is not an Armada package") : { ok: true, value: { pieces } };
      },
      addPackage: async (body) => {
        const pieces = made.get(body.text);
        if (pieces === undefined) return refusal("fleet.share_malformed", "This is not an Armada package");
        const held = pieces.find((one) => one.exists);
        if (held !== undefined && body.replace !== true) return refusal("fleet.share_exists", `You already have ${held.name}`);
        return { ok: true, value: { pieces: pieces.map(({ kind, id, name, repository, exists }) => ({ kind, id, name, ...(repository === undefined ? {} : { repository }), landed: exists ? ("replaced" as const) : ("added" as const) })) } };
      },
      listUnboundPieces: async () => ({ ok: true, value: { pieces: [...unbound] } }),
      bindUnboundPiece: async (body) => {
        const at = unbound.findIndex((one) => one.unbound_id === body.unbound_id);
        if (at < 0) return refusal("fleet.share_no_unbound_piece", "That piece is no longer waiting");
        const [one] = unbound.splice(at, 1);
        return { ok: true, value: { kind: one!.kind, id: one!.id, name: one!.name, repository: body.repository, landed: "added" } };
      },
      savePackageFile: async (name) => ({ saved: true, name }),
    };
  },
};
