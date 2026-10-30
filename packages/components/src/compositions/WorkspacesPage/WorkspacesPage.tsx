// System > Workspaces (armada-cwj): every level an autonomy default can be set at, as a list on the left, and the picked level on the right. Presentational: the host holds the Workspaces and their layers.

import { FolderGit2, Globe, Layers, Plus } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { AutonomyPanel, autonomyRows } from "../WorkflowManager/Autonomy";
import type { AutonomyKind, AutonomyLayers, AutonomyLevel, AutonomyTiers } from "../WorkflowManager/Autonomy";
import { WorkspaceForm } from "../WorkflowManager/WorkspaceForm";
import type { Autonomy, Repository } from "../LaneCanvas/model";

/** A Workspace as the page lists it: the repositories in it by id, and where it is committed if it is not kept on this Mac. */
export type WorkspaceRow = { id: string; name: string; repositories: readonly string[]; home?: string };

/** The level the page shows. `new` is the empty form. */
export type WorkspacesPick = { kind: "everywhere" } | { kind: "workspace"; id: string } | { kind: "repository"; id: string } | { kind: "new" };

export type WorkspacesPageProps = {
  /** Every repository Fleet serves. */
  repositories: readonly Repository[];
  workspaces: readonly WorkspaceRow[];
  /** What each level sets, under `everywhere`, `workspace:<id>` and `repository:<id>`. */
  layers: AutonomyLayers;
  picked: WorkspacesPick;
  /** Fleet's words for what it refused, where the press was. */
  notice?: string | undefined;
  onPick: (pick: WorkspacesPick) => void;
  onSet: (key: string, kind: AutonomyKind, level: Autonomy | undefined) => void;
  onCreate: (name: string, repositories: readonly string[]) => void;
  onSave: (id: string, name: string, repositories: readonly string[]) => void;
  onRemove: (id: string) => void;
  /** Opens System > Workflows, where a Workspace's lifecycles and rules are. */
  onOpenWorkflows: () => void;
};

const EVERYWHERE: AutonomyLevel = { key: "everywhere", name: "Everywhere" };

/** The layer key a level is kept under. */
export const layerKey = (pick: Exclude<WorkspacesPick, { kind: "new" }>): string => (pick.kind === "everywhere" ? "everywhere" : `${pick.kind}:${pick.id}`);

export function WorkspacesPage({ repositories, workspaces, layers, picked, notice, onPick, onSet, onCreate, onSave, onRemove, onOpenWorkflows }: WorkspacesPageProps) {
  const name = (id: string) => repositories.find((one) => one.id === id)?.name ?? id;
  const inNone = repositories.filter((one) => !workspaces.some((held) => held.repositories.includes(one.id)));
  const workspace = picked.kind === "workspace" ? workspaces.find((one) => one.id === picked.id) : undefined;

  const row = (key: string, on: boolean, label: string, glyph: "everywhere" | "workspace" | "repository", pick: WorkspacesPick, member = false) => (
    <button type="button" className="armada-workspaces__row" data-nav-item="" data-member={member || undefined} aria-current={on ? "true" : undefined} tabIndex={on || (picked.kind === "new" && glyph === "everywhere") ? 0 : -1} onClick={() => onPick(pick)} key={key}>
      {glyph === "everywhere" ? <Globe size={16} strokeWidth={2} aria-hidden="true" /> : glyph === "workspace" ? <Layers size={16} strokeWidth={2} aria-hidden="true" /> : <FolderGit2 size={16} strokeWidth={2} aria-hidden="true" />}
      <span className="armada-workspaces__name">{label}</span>
    </button>
  );
  const repositoryRow = (id: string, member: boolean, key: string) => row(key, picked.kind === "repository" && picked.id === id, name(id), "repository", { kind: "repository", id }, member);

  /** The levels the picked one reads, nearest first: itself, the Workspaces a repository is in as peers, then Everywhere. */
  const tiers = (): AutonomyTiers => {
    if (picked.kind === "everywhere" || picked.kind === "new") return [[EVERYWHERE]];
    if (picked.kind === "workspace") return [[{ key: `workspace:${picked.id}`, name: workspace?.name ?? picked.id }], [EVERYWHERE]];
    const held = workspaces.filter((one) => one.repositories.includes(picked.id)).map((one): AutonomyLevel => ({ key: `workspace:${one.id}`, name: one.name }));
    return [[{ key: `repository:${picked.id}`, name: name(picked.id) }], ...(held.length === 0 ? [] : [held]), [EVERYWHERE]];
  };
  const autonomy =
    picked.kind === "new" ? null : (
      <section className="armada-workspaces__autonomy" aria-label="Autonomy">
        <h3 className="armada-workspaces__caption">Autonomy</h3>
        <AutonomyPanel rows={autonomyRows(layers, tiers())} onSet={(kind, level) => onSet(layerKey(picked), kind, level)} />
      </section>
    );

  return (
    <div className="armada-workspaces">
      <section className="armada-workspaces__list" aria-label="Levels" data-col="list">
        <Button variant="secondary" size="sm" ground="card" onClick={() => onPick({ kind: "new" })}>
          <Plus size={16} strokeWidth={2} aria-hidden="true" />
          New Workspace
        </Button>
        <div className="armada-scroll-area armada-workspaces__scroll">
          <ul className="armada-workspaces__items">
            <li>{row("everywhere", picked.kind === "everywhere", "Everywhere", "everywhere", { kind: "everywhere" })}</li>
            {workspaces.map((one) => (
              <li key={one.id} role="group" aria-label={one.name}>
                {row(`workspace:${one.id}`, picked.kind === "workspace" && picked.id === one.id, one.name, "workspace", { kind: "workspace", id: one.id })}
                {one.repositories.map((id) => repositoryRow(id, true, `${one.id}/${id}`))}
              </li>
            ))}
            {inNone.length === 0 ? null : (
              <li role="group" aria-label="In no Workspace" className="armada-workspaces__none">
                {inNone.map((one) => repositoryRow(one.id, false, one.id))}
              </li>
            )}
          </ul>
        </div>
      </section>

      <section className="armada-workspaces__detail" aria-label="Detail" data-col="detail">
        {notice === undefined ? null : (
          <p className="armada-workspaces__said" role="alert" data-refused>
            {notice}
          </p>
        )}
        {picked.kind === "new" ? (
          <div className="armada-workspaces__panel">
            <WorkspaceForm key="new" mode="new" repositories={repositories} onSubmit={onCreate} />
          </div>
        ) : null}
        {workspace === undefined ? null : (
          <div className="armada-workspaces__panel">
            <WorkspaceForm
              key={`${workspace.id}/${workspace.name}/${workspace.repositories.join()}`}
              mode="edit"
              repositories={repositories}
              name={workspace.name}
              ticked={workspace.repositories}
              {...(workspace.home === undefined ? {} : { home: name(workspace.home) })}
              onSubmit={(next, ids) => onSave(workspace.id, next, ids)}
              onRemove={() => onRemove(workspace.id)}
            />
          </div>
        )}
        {autonomy}
        {workspace === undefined ? null : (
          <Button variant="ghost" size="sm" onClick={onOpenWorkflows}>
            Lifecycles and rules
          </Button>
        )}
      </section>
    </div>
  );
}
