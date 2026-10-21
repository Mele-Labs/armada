// Kit's MCP servers, read for the Manifest surface and changed there. #1275.
//
// Beside `manifest-allows.ts`, whose shape this is: the wire half and the hook
// that draws from it, so main can import the read type without a component in
// reach. One type for every act — each answers with the whole list, so a caller
// folding one answer has already folded the other.

import { useEffect, useState } from "react";
import type {
  AddKitServer,
  KitAllowedCommands,
  KitInventory,
  KitPieceInstalled,
  KitPieceRemoved,
  KitSourceFound,
  KitServers,
  ManifestReach,
  Outcome,
  ReachesDrones,
  RemoveKitPiece,
  RestoreKitPiece,
  ServerAddress,
} from "@armada/protocol";

/** `GET /kit/servers` and the four acts under it, read into the app. */
export type KitServersRead = { ok: true; kit: KitServers } | { ok: false; outcome: Outcome };

/** `GET /kit/inventory`, read into the app. #1491. */
export type KitInventoryRead = { ok: true; setup: KitInventory } | { ok: false; outcome: Outcome };

/** `remove_kit_allowed_command`: what the allowlist holds after one is taken out. Since 23.35. */
export type KitAllowedCommandsRead = { ok: true; commands: KitAllowedCommands } | { ok: false; outcome: Outcome };

/** `look_at_kit_source`, read into the app. */
export type KitLookRead = { ok: true; found: KitSourceFound } | { ok: false; outcome: Outcome };

/** `install_kit_piece`, read into the app. */
export type KitInstallRead = { ok: true; installed: KitPieceInstalled } | { ok: false; outcome: Outcome };

/** `remove_kit_piece`, read into the app. */
export type KitRemoveRead = { ok: true; removed: KitPieceRemoved } | { ok: false; outcome: Outcome };

/** `restore_kit_piece`: the inventory with the skill back. */
export type KitRestoreRead = { ok: true; setup: KitInventory } | { ok: false; outcome: Outcome };

/** What the Manifest surface asks of the host for Kit. */
export type KitSlice = {
  /** The setup a person already has, read to be shown. Machine-wide. #1491. */
  onReadKitInventory: () => Promise<KitInventoryRead>;
  /** Take a command out of the allowlist, by the line as the inventory spelled it. Since 23.35. */
  onRemoveKitAllowedCommand: (run: string) => Promise<KitAllowedCommandsRead>;
  onListKitServers: () => Promise<KitServersRead>;
  onAddKitServer: (adding: AddKitServer) => Promise<KitServersRead>;
  onForgetKitServer: (name: string) => Promise<KitServersRead>;
  onSetKitServerReach: (name: string, drones: ReachesDrones) => Promise<KitServersRead>;
  onSetManifestServerReach: (name: string, reach: ManifestReach | null) => Promise<KitServersRead>;
  /** Remove for skills and plugins, and Undo for a skill. */
  onRemoveKitPiece: (body: RemoveKitPiece) => Promise<KitRemoveRead>;
  onRestoreKitPiece: (body: RestoreKitPiece) => Promise<KitRestoreRead>;
};

/** What a Job started from a dropped link is asked, in the person's place. */
export const kitSourceJobRequest = (source: string): string =>
  `Set up ${source} in my Kit. Find out whether it is a plugin or a skill, install it, and report what it added.`;

/** What a Job started to fix a failed install is asked: the link, the command that failed and what it printed. */
export function kitInstallFixRequest(source: string, failed: { command: string; error: string }): string {
  return `Install ${source} in my Kit. Armada ran \`${failed.command}\` and it failed with:\n\n${failed.error}\n\nFind out why, install it, and report what you changed.`;
}

/** What a Job already open is sent when a dropped link is added to it, as a message from the person. */
export function kitSourceJobMessage(source: string, found: KitSourceFound | undefined): string {
  const what = found === undefined || found.found === "unknown" ? "" : ` It is the ${found.name} ${found.found}, from ${found.repository}.`;
  return `Use ${source} for this work, and set it up first if it needs it.${what}`;
}

const changed = new Set<() => void>();

/** Something outside the Kit pages changed what Kit holds: an install from a dropped link. Every open Kit page reads again. */
export function kitChanged(): void {
  changed.forEach((one) => one());
}

/** Remove and Undo, as the Kit pages draw them. Add opens the link panel, which is the shell's. */
export type KitPieces = {
  onRemove: (body: RemoveKitPiece) => void;
  /** The skill last moved to the Trash, while Undo can still bring it back. */
  trashed: { name: string } | null;
  onUndo: () => void;
};

/**
 * An address a person typed, turned into the shape the wire carries.
 *
 * **A program's arguments are split on whitespace here and nowhere else.**
 * There is no shell between Bridge and the spawn, so something has to do it,
 * and doing it once beside the form is what keeps the field a person reads and
 * the array fleet stores in step. `null` where the text carries neither.
 */
export function addressTyped(transport: "stdio" | "http", typed: string): ServerAddress | null {
  const text = typed.trim();
  if (text === "") return null;
  if (transport === "http") {
    return text.startsWith("http://") || text.startsWith("https://") ? { transport, url: text } : null;
  }
  const [command, ...args] = text.split(/\s+/);
  return command === undefined ? null : { transport, command, args };
}

/** An address, as a row draws it: the program and its arguments, or the URL. */
export function addressReads(address: ServerAddress): string {
  return address.transport === "http" ? address.url : [address.command, ...address.args].join(" ");
}

/**
 * Kit's servers, read once on opening and again off whatever an act answers
 * with.
 *
 * **`undefined` until the first read lands**, `useRepositoryAllows`' reason: an
 * empty list drawn before the read would say Kit holds nothing, which is the
 * one answer on this page nobody should be given by accident.
 */
export function useKit(slice: KitSlice): {
  setup: KitInventory | undefined;
  kit: KitServers | undefined;
  refused: Outcome | null;
  onAdd: (adding: AddKitServer) => void;
  onRemoveAllowed: (run: string) => void;
  onForget: (name: string) => void;
  onKitReach: (name: string, drones: ReachesDrones) => void;
  onManifestReach: (name: string, reach: ManifestReach | null) => void;
  pieces: KitPieces;
} {
  const [kit, setKit] = useState<KitServers | undefined>(undefined);
  const [setup, setSetup] = useState<KitInventory | undefined>(undefined);
  const [refused, setRefused] = useState<Outcome | null>(null);
  const [trashed, setTrashed] = useState<{ name: string; at: string } | null>(null);

  const took = (read: KitServersRead): void => {
    if (read.ok) {
      setKit(read.kit);
      setRefused(null);
      return;
    }
    // The `Outcome` and not a sentence. `copy.ts` reaches components, and a
    // value import from here would pull every `.tsx` that package exports into
    // main and preload's `--jsx`-less projects — `manifest-allows.ts`'s rule.
    setRefused(read.outcome);
  };

  useEffect(() => {
    void slice.onListKitServers().then((read) => {
      if (read.ok) setKit(read.kit);
    });
    // Read fresh on opening rather than kept: a person who installs a skill and
    // comes back expects to see it, and nothing in the app is told when they do.
    void slice.onReadKitInventory().then((read) => {
      if (read.ok) setSetup(read.setup);
    });
    // Once, on opening. Every act below folds its own answer.
  }, []);

  useEffect(() => {
    const again = () =>
      void slice.onReadKitInventory().then((read) => {
        if (read.ok) setSetup(read.setup);
      });
    changed.add(again);
    return () => {
      changed.delete(again);
    };
  }, [slice.onReadKitInventory]);

  // Folded off the answer, which is what the allowlist holds now: a row Fleet no longer
  // lists goes, and one it still lists stays. A refusal means the window was out of date,
  // so the inventory is read again and the row says what is true.
  const removed = (read: KitAllowedCommandsRead): void => {
    if (read.ok) {
      const left = new Set(read.commands.commands.map((one) => one.run));
      setSetup((was) =>
        was === undefined
          ? was
          : {
              ...was,
              kinds: was.kinds.map((one) =>
                one.kind === "allowlist" && one.read.what === "read"
                  ? { ...one, read: { ...one.read, items: one.read.items.filter((item) => left.has(item.name)) } }
                  : one,
              ),
            },
      );
      setRefused(null);
      return;
    }
    setRefused(read.outcome);
    void slice.onReadKitInventory().then((again) => {
      if (again.ok) setSetup(again.setup);
    });
  };

  // Each answers with the inventory as it is now; a refusal reads it again, `removed`'s reason.
  const refusedPiece = (outcome: Outcome): void => {
    setRefused(outcome);
    void slice.onReadKitInventory().then((again) => {
      if (again.ok) setSetup(again.setup);
    });
  };
  const pieces: KitPieces = {
    onRemove: (body) =>
      void slice.onRemoveKitPiece(body).then((read) => {
        if (!read.ok) return refusedPiece(read.outcome);
        setRefused(null);
        setSetup(read.removed.setup);
        setTrashed(read.removed.trashed === null ? null : { name: body.name, at: read.removed.trashed });
      }),
    trashed: trashed === null ? null : { name: trashed.name },
    onUndo: () => {
      if (trashed === null) return;
      void slice.onRestoreKitPiece({ trashed: trashed.at }).then((read) => {
        // Undo is offered once: a place Fleet will not put back now it will not put back on a second press either.
        setTrashed(null);
        if (!read.ok) return refusedPiece(read.outcome);
        setRefused(null);
        setSetup(read.setup);
      });
    },
  };

  return {
    setup,
    kit,
    pieces,
    refused,
    onAdd: (adding) => void slice.onAddKitServer(adding).then(took),
    onRemoveAllowed: (run) => void slice.onRemoveKitAllowedCommand(run).then(removed),
    onForget: (name) => void slice.onForgetKitServer(name).then(took),
    onKitReach: (name, drones) => void slice.onSetKitServerReach(name, drones).then(took),
    onManifestReach: (name, reach) => void slice.onSetManifestServerReach(name, reach).then(took),
  };
}
