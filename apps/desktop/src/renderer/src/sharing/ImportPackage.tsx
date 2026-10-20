// A package someone sent, dropped on Bridge as its file or pasted as its text, opens a sheet listing every piece and every shell
// command it would add, read by Fleet without writing anything (`read_package`). **Add is the one ask** (`add_package`); where Kit
// already holds an id, the one ask is Replace. A piece whose remote names no repository this Fleet serves waits until one is chosen
// for it (`list_unbound_pieces`, `bind_unbound_piece`). `docs/design/who-sees-each-piece-of-configuration.md`, *Share is a package*.

import { useEffect, useState } from "react";
import { Button, Select, Sheet } from "@armada/components";
import type { PackagePiece, PieceAdded, PieceKind, RepositorySummary, UnboundPiece } from "@armada/protocol";

import { failedSaid, type ActSaid } from "./PieceActs";
import "./sharing.css";

/** What a package's file is called, and what its text opens with. */
const FILE = /\.armada\.toml$/i;
const TEXT = /^\s*version\s*=\s*\d+\s*$/m;

/** Whether pasted text is a package: its version line and at least one piece. */
export const isPackageText = (text: string): boolean => TEXT.test(text) && text.includes("[[piece]]");

const KIND: Record<PieceKind, string> = { tile: "Tile", workflow: "Workflow", review_tool: "Review tool" };
const LANDED: Record<PieceAdded["landed"], string> = { added: "Added", replaced: "Replaced", unbound: "Waiting for a repository" };

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** Hears a package dropped or pasted anywhere in the window but a field being typed in, and hands its text on. */
export function usePackageArrivals(open: (text: string) => void): void {
  useEffect(() => {
    const fileOf = (event: DragEvent): File | undefined => [...(event.dataTransfer?.files ?? [])].find((one) => FILE.test(one.name));
    const named = (event: DragEvent): boolean => [...(event.dataTransfer?.items ?? [])].some((one) => one.kind === "file") || fileOf(event) !== undefined;
    const over = (event: DragEvent) => {
      if (!event.defaultPrevented && named(event)) event.preventDefault();
    };
    const drop = (event: DragEvent) => {
      if (event.defaultPrevented || !named(event)) return;
      // Any file let fall on the window is kept from replacing it; only a package is read.
      event.preventDefault();
      void fileOf(event)?.text().then(open);
    };
    const paste = (event: ClipboardEvent) => {
      const text = event.clipboardData?.getData("text/plain") ?? "";
      if (event.defaultPrevented || typing(event.target) || !isPackageText(text)) return;
      event.preventDefault();
      open(text);
    };
    window.addEventListener("dragover", over);
    window.addEventListener("drop", drop);
    window.addEventListener("paste", paste);
    return () => {
      window.removeEventListener("dragover", over);
      window.removeEventListener("drop", drop);
      window.removeEventListener("paste", paste);
    };
  }, [open]);
}

const nameOf = (repositories: readonly RepositorySummary[], root: string | undefined): string | undefined =>
  root === undefined ? undefined : (repositories.find((one) => one.root === root)?.manifest?.repository ?? root);

/** One piece as the list draws it: its kind and name, where it lands, and each command it runs. */
function PieceRow({ piece, repositories, landed }: { piece: Pick<PackagePiece, "kind" | "id" | "name" | "remote" | "repository"> & { commands?: readonly string[]; exists?: boolean }; repositories: readonly RepositorySummary[]; landed?: string }) {
  const where = nameOf(repositories, piece.repository) ?? piece.remote ?? "No repository";
  return (
    <li className="import-piece" aria-label={`${KIND[piece.kind]} ${piece.name}`}>
      <span className="import-piece__head">
        <span className="import-piece__kind">{KIND[piece.kind]}</span>
        <span className="import-piece__name">{piece.name}</span>
        <span className="import-piece__where">{where}</span>
        {landed !== undefined ? <span className="import-piece__fact">{landed}</span> : piece.exists === true ? <span className="import-piece__fact">Replaces yours</span> : null}
      </span>
      {(piece.commands ?? []).map((command) => (
        <code key={command} className="import-piece__command" aria-label="Command">
          {command}
        </code>
      ))}
    </li>
  );
}

/** A piece waiting for a repository, and where to choose one. */
function Unbound({ piece, repositories, onBound }: { piece: UnboundPiece; repositories: readonly RepositorySummary[]; onBound: (added: PieceAdded) => void }) {
  const [root, setRoot] = useState(repositories[0]?.root ?? "");
  const [said, setSaid] = useState<ActSaid>();
  const bind = async () => {
    const answer = await window.armada.bindUnboundPiece({ unbound_id: piece.unbound_id, repository: root });
    if (!answer.ok) return setSaid(failedSaid(answer.outcome));
    onBound(answer.value);
  };
  return (
    <li className="import-piece" aria-label={`${KIND[piece.kind]} ${piece.name}, waiting`}>
      <span className="import-piece__head">
        <span className="import-piece__kind">{KIND[piece.kind]}</span>
        <span className="import-piece__name">{piece.name}</span>
        <span className="import-piece__where">{piece.remote ?? "No repository"}</span>
      </span>
      <span className="import-piece__bind">
        <Select label="Repository" value={root} onChange={(event) => setRoot(event.target.value)}>
          {repositories.map((one) => (
            <option key={one.root} value={one.root}>
              {one.manifest?.repository ?? one.root}
            </option>
          ))}
        </Select>
        <Button variant="secondary" size="sm" disabled={root === ""} onClick={() => void bind()}>
          Choose
        </Button>
      </span>
      {said === undefined ? null : (
        <p className="piece-acts__said" role="alert">
          {said.said}
        </p>
      )}
    </li>
  );
}

type Shown = { what: "reading" } | { what: "read"; pieces: readonly PackagePiece[] } | { what: "added"; pieces: readonly PieceAdded[]; unbound: readonly UnboundPiece[] } | { what: "refused"; said: string };

/** The sheet over a package's text: what it would add, then Add. */
export function ImportSheet({ text, repositories, onClose }: { text: string; repositories: readonly RepositorySummary[]; onClose: () => void }) {
  const [shown, setShown] = useState<Shown>({ what: "reading" });
  const [busy, setBusy] = useState(false);
  const refusedOr = (outcome: Parameters<typeof failedSaid>[0]): Shown | undefined => {
    const said = failedSaid(outcome);
    return said === undefined ? undefined : { what: "refused", said: said.said };
  };
  useEffect(() => {
    let current = true;
    void window.armada.readPackage(text).then((answer) => {
      if (!current) return;
      if (answer.ok) setShown({ what: "read", pieces: answer.value.pieces });
      else {
        const next = refusedOr(answer.outcome);
        if (next === undefined) onClose();
        else setShown(next);
      }
    });
    return () => {
      current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [text]);
  const unboundNow = async (): Promise<readonly UnboundPiece[]> => {
    const answer = await window.armada.listUnboundPieces();
    return answer.ok ? answer.value.pieces : [];
  };
  const replacing = shown.what === "read" && shown.pieces.some((one) => one.exists);
  const add = async () => {
    if (busy) return;
    setBusy(true);
    try {
      const answer = await window.armada.addPackage({ text, ...(replacing ? { replace: true } : {}) });
      if (!answer.ok) {
        const next = refusedOr(answer.outcome);
        if (next !== undefined) setShown(next);
        return;
      }
      const pieces = answer.value.pieces;
      setShown({ what: "added", pieces, unbound: pieces.some((one) => one.landed === "unbound") ? await unboundNow() : [] });
    } finally {
      setBusy(false);
    }
  };
  const rebound = async (unboundId: string, added: PieceAdded) => {
    if (shown.what !== "added") return;
    const pieces = shown.pieces.map((one) => (one.unbound_id === unboundId ? added : one));
    setShown({ what: "added", pieces, unbound: await unboundNow() });
  };
  const footer =
    shown.what === "read" ? (
      <Button onClick={() => void add()} disabled={busy || shown.pieces.length === 0}>
        {replacing ? "Replace" : "Add"}
      </Button>
    ) : shown.what === "reading" ? null : (
      <Button variant="secondary" onClick={onClose}>
        Close
      </Button>
    );
  return (
    <Sheet open kind="import-package" title="Import" onClose={onClose} footer={footer}>
      {shown.what === "refused" ? (
        <p className="piece-acts__said" role="alert">
          {shown.said}
        </p>
      ) : shown.what === "read" ? (
        <ul className="import-pieces" aria-label="Pieces">
          {shown.pieces.map((one) => (
            <PieceRow key={`${one.kind}:${one.id}:${one.repository ?? one.remote ?? ""}`} piece={one} repositories={repositories} />
          ))}
        </ul>
      ) : shown.what === "added" ? (
        <ul className="import-pieces" aria-label="Pieces">
          {shown.pieces
            .filter((one) => one.landed !== "unbound" || !shown.unbound.some((it) => it.unbound_id === one.unbound_id))
            .map((one) => (
              <PieceRow key={`${one.kind}:${one.id}:${one.repository ?? one.remote ?? ""}`} piece={one} repositories={repositories} landed={LANDED[one.landed]} />
            ))}
          {shown.unbound.map((one) => (
            <Unbound key={one.unbound_id} piece={one} repositories={repositories} onBound={(added) => void rebound(one.unbound_id, added)} />
          ))}
        </ul>
      ) : null}
    </Sheet>
  );
}

/** The window's import: heard at the window, drawn as one sheet at a time. */
export function ImportPackage({ repositories }: { repositories: readonly RepositorySummary[] }) {
  const [text, setText] = useState<string | undefined>();
  usePackageArrivals(setText);
  return text === undefined ? null : <ImportSheet key={text} text={text} repositories={repositories} onClose={() => setText(undefined)} />;
}
