// Kit's MCP servers, read for the Manifest surface and changed there. #1275.
//
// Beside `manifest-allows.ts`, whose shape this is: the wire half and the hook
// that draws from it, so main can import the read type without a component in
// reach. One type for every act — each answers with the whole list, so a caller
// folding one answer has already folded the other.

import { useEffect, useState } from "react";
import type {
  AddKitServer,
  InstallKitPiece,
  KitAllowedCommands,
  KitInventory,
  KitPieceInstalled,
  KitPieceRemoved,
  KitSourceFound,
  LookAtKitSource,
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

/** `look_at_kit_source`, read into the app. Not on the wire yet: the mock alone answers it. */
export type KitLookRead = { ok: true; found: KitSourceFound } | { ok: false; outcome: Outcome };

/** A Job started to set a source up, by the title it was started under. */
export type KitJobStarted = { ok: true; title: string } | { ok: false; outcome: Outcome };

/** `install_kit_piece`, read into the app. Not on the wire yet: the mock alone answers it. */
export type KitInstallRead = { ok: true; installed: KitPieceInstalled } | { ok: false; outcome: Outcome };

/** `remove_kit_piece`, read into the app. Not on the wire yet. */
export type KitRemoveRead = { ok: true; removed: KitPieceRemoved } | { ok: false; outcome: Outcome };

/** `restore_kit_piece`: the inventory with the skill back. Not on the wire yet. */
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
  /** Add and Remove for skills and plugins. Absent where the host has no way to run them, and then neither is drawn. */
  onLookAtKitSource?: ((body: LookAtKitSource) => Promise<KitLookRead>) | undefined;
  /** A Job that takes the source, sets it up and reports back. */
  onStartKitSourceJob?: ((source: string) => Promise<KitJobStarted>) | undefined;
  onInstallKitPiece?: ((body: InstallKitPiece) => Promise<KitInstallRead>) | undefined;
  onRemoveKitPiece?: ((body: RemoveKitPiece) => Promise<KitRemoveRead>) | undefined;
  onRestoreKitPiece?: ((body: RestoreKitPiece) => Promise<KitRestoreRead>) | undefined;
};

/** What a Job started from Add is asked, in the person's place. */
export const kitSourceJobRequest = (source: string): string =>
  `Set up ${source} in my Kit. Find out whether it is a plugin or a skill, install it, and report what it added.`;

/** Add and Remove, as the Kit pages draw them. */
export type KitPieces = {
  onLook: (body: LookAtKitSource) => Promise<KitLookRead>;
  onStartJob: (source: string) => Promise<KitJobStarted>;
  /** Answers with what the install came to, the inventory already holding the new rows where it installed. */
  onInstall: (body: InstallKitPiece) => Promise<KitInstallRead>;
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
  pieces: KitPieces | undefined;
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

  const look = slice.onLookAtKitSource;
  const startJob = slice.onStartKitSourceJob;
  const install = slice.onInstallKitPiece;
  const remove = slice.onRemoveKitPiece;
  const restore = slice.onRestoreKitPiece;
  const pieces: KitPieces | undefined =
    look === undefined || startJob === undefined || install === undefined || remove === undefined || restore === undefined
      ? undefined
      : {
          onLook: look,
          onStartJob: startJob,
          onInstall: async (body) => {
            const read = await install(body);
            if (read.ok && read.installed.state === "installed") setSetup(read.installed.setup);
            return read;
          },
          onRemove: (body) =>
            void remove(body).then((read) => {
              if (!read.ok) {
                setRefused(read.outcome);
                return;
              }
              setRefused(null);
              setSetup(read.removed.setup);
              setTrashed(read.removed.trashed === null ? null : { name: body.name, at: read.removed.trashed });
            }),
          trashed: trashed === null ? null : { name: trashed.name },
          onUndo: () => {
            if (trashed === null) return;
            void restore({ trashed: trashed.at }).then((read) => {
              if (!read.ok) {
                setRefused(read.outcome);
                return;
              }
              setSetup(read.setup);
              setTrashed(null);
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
