// The picked repository's own surfaces, each window's: the Manifest file and Setup, picking and
// locating a repository, its always-allows and Kit. Out of `index.ts` for the gate's length.

import { BrowserWindow, dialog, type IpcMain } from "electron";
import type { EditManifest, EditManifestProposal, SaveManifestFile, WriteManifestProposal } from "@armada/protocol";
import type { SavingWorkflow } from "../shared/workflows";
import type { AddKitServer, ManifestReach, ReachesDrones } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { resolvedFolder } from "./locating";

type Hosts = {
  ipc: IpcMain;
  /** The open connection, or `null` before there is one. */
  connection: () => FleetConnection | null;
  /** The window an IPC call arrived from — `index.ts`'s own. */
  windowIdOf: (event: Electron.IpcMainInvokeEvent) => number;
};

export function handleRepositories({ ipc, connection, windowIdOf }: Hosts): void {
  // The Manifest file, read and saved, and Setup below it — each window's own, `connection.ts`'s
  // `editingFor`: Fleet resolves the path and guards the write against a file that moved; nothing
  // here composes either.
  ipc.handle(CHANNELS.readManifestFile, (event) => connection()?.editingFor(windowIdOf(event)).readFile());
  ipc.handle(CHANNELS.saveManifestFile, (event, body: SaveManifestFile) =>
    connection()?.editingFor(windowIdOf(event)).saveFile(body),
  );
  ipc.handle(CHANNELS.editManifest, (event, body: EditManifest) =>
    connection()?.editingFor(windowIdOf(event)).edit(body),
  );
  ipc.handle(CHANNELS.readManifestSpend, (event) => connection()?.editingFor(windowIdOf(event)).readSpend());
  ipc.handle(CHANNELS.readRepositoryScan, (event) => connection()?.editingFor(windowIdOf(event)).setup.readScan());
  ipc.handle(CHANNELS.readManifestProposals, (event) =>
    connection()?.editingFor(windowIdOf(event)).setup.readProposals(),
  );
  ipc.handle(CHANNELS.editManifestProposal, (event, body: EditManifestProposal) =>
    connection()?.editingFor(windowIdOf(event)).setup.edit(body),
  );
  ipc.handle(CHANNELS.writeManifestProposal, (event, body: WriteManifestProposal) =>
    connection()?.editingFor(windowIdOf(event)).setup.write(body),
  );
  // `null` is All repositories. Anything but a string or `null` is dropped here; a root Fleet does not list, in `Picked.pick`.
  ipc.handle(CHANNELS.pickRepository, (event, root: unknown) =>
    typeof root === "string" || root === null ? connection()?.repositories.pick(windowIdOf(event), root) : undefined,
  );
  // Locate. The folder dialog is sheeted to the window that asked, so it cannot be left behind it.
  ipc.handle(CHANNELS.chooseFolder, async (event) => {
    const window = BrowserWindow.fromWebContents(event.sender);
    const options: Electron.OpenDialogOptions = { properties: ["openDirectory", "createDirectory"] };
    const chosen = window === null ? await dialog.showOpenDialog(options) : await dialog.showOpenDialog(window, options);
    return chosen.canceled ? null : (chosen.filePaths[0] ?? null);
  });
  // A path in, its canonical folder out: the clone preview names what Fleet will. Reads nothing inside it.
  ipc.handle(CHANNELS.resolveFolder, (_event, path: unknown) => (typeof path === "string" ? resolvedFolder(path) : null));
  ipc.handle(CHANNELS.addRepository, (_event, path: unknown) =>
    typeof path === "string" ? connection()?.repositories.locating.add(path) : undefined,
  );
  ipc.handle(CHANNELS.cloneRepository, (_event, url: unknown, parent: unknown) =>
    typeof url === "string" && typeof parent === "string"
      ? connection()?.repositories.locating.clone(url, parent)
      : undefined,
  );
  // A repository-wide always-allow — Fleet's own table since protocol 13.5.
  // Neither takes a path or a job id: Fleet names the repository this window picked.
  ipc.handle(CHANNELS.listRepositoryAllowedCommands, (event) =>
    connection()?.repositoryAllowsFor(windowIdOf(event)).list(),
  );
  ipc.handle(CHANNELS.removeRepositoryAllowedCommand, (event, run: string) =>
    connection()?.repositoryAllowsFor(windowIdOf(event)).remove(run),
  );
  // Kit's MCP servers — #1275. Kit itself is machine-wide; the picked
  // repository is what scopes the Manifest tier, so none of these names one.
  ipc.handle(CHANNELS.readKitInventory, (event) =>
    connection()?.kitFor(windowIdOf(event)).inventory(),
  );
  ipc.handle(CHANNELS.listKitServers, (event) => connection()?.kitFor(windowIdOf(event)).list());
  ipc.handle(CHANNELS.addKitServer, (event, adding: AddKitServer) =>
    connection()?.kitFor(windowIdOf(event)).add(adding),
  );
  ipc.handle(CHANNELS.forgetKitServer, (event, name: string) =>
    connection()?.kitFor(windowIdOf(event)).forget(name),
  );
  ipc.handle(CHANNELS.setKitServerReach, (event, name: string, drones: ReachesDrones) =>
    connection()?.kitFor(windowIdOf(event)).setKitReach(name, drones),
  );
  ipc.handle(
    CHANNELS.setManifestServerReach,
    (event, name: string, reach: ManifestReach | null) =>
      connection()?.kitFor(windowIdOf(event)).setManifestReach(name, reach),
  );
  // The Workflow creator's list, one definition, and a save.
  ipc.handle(CHANNELS.readWorkflows, (event) => connection()?.workflowsFor(windowIdOf(event)).list());
  ipc.handle(CHANNELS.readWorkflowDefinition, (event, workflowId: string, source: string) =>
    connection()?.workflowsFor(windowIdOf(event)).definition(workflowId, source),
  );
  ipc.handle(CHANNELS.saveWorkflow, (event, saving: SavingWorkflow) =>
    connection()?.workflowsFor(windowIdOf(event)).save(saving.manifestId, saving.body),
  );
}
