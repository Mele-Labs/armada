// The toast a deleted task or project, or an unlinked issue, leaves for a moment, with Undo: nothing on the tab asks before it goes.

import { useEffect } from "react";
import { Toast } from "@armada/components";

import { useProjects, useProjectsStore } from "./store";

/** How long the toast offers the row back. */
const UNDO_MS = 6000;

export function ProjectsUndo() {
  const store = useProjectsStore();
  const { removed } = useProjects();
  useEffect(() => {
    if (removed === undefined || store === undefined) return;
    const gone = window.setTimeout(store.forgetRemoved, UNDO_MS);
    return () => window.clearTimeout(gone);
  }, [removed, store]);
  if (removed === undefined || store === undefined) return null;
  const said = removed.kind === "project" ? `Deleted ${removed.project.name}` : removed.row.kind === "issue" ? `Unlinked ${removed.row.reference}` : `Deleted ${removed.row.title}`;
  return (
    <Toast actionLabel="Undo" onAction={store.undo}>
      {said}
    </Toast>
  );
}
