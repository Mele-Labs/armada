// A restart a Session hands the person as a command is the Fleet panel's build switch, pressed. A
// shell run is Fleet's own child: it dies with the Fleet it stops, before Bridge is reopened, and
// leaves Bridge on the old protocol (13 Oct 2026). `change_fleet_build` starts it detached instead.

import type { BuildSource } from "@armada/protocol";

export type BuildChange = { build: BuildSource; adopt: boolean; label: string };

const SCRIPTS: Record<string, { build: BuildSource; flag: string; label: string }> = {
  preview: { build: "preview", flag: "--restart", label: "Refresh preview" },
  restart: { build: "main", flag: "--main", label: "Update to main" },
};

/** The build switch `command` asks for, or `undefined` where it is any other command, a dry run among them. */
export function buildChangeOf(command: string): BuildChange | undefined {
  const [script = "", ...flags] = command.trim().split(/\s+/);
  const name = /^(?:.*\/)?scripts\/(preview|restart)$/.exec(script)?.[1];
  const known = name === undefined ? undefined : SCRIPTS[name];
  if (known === undefined || !flags.includes(known.flag)) return undefined;
  if (flags.some((flag) => flag !== known.flag && flag !== "--adopt")) return undefined;
  return { build: known.build, adopt: flags.includes("--adopt"), label: known.label };
}

/** The build switch a waiting item's act asks for. **The one reading every surface of a waiting item shares**, so a restart is the same key on the Session's list and on its call card. */
export function restartOf(act: { kind: string; target: string } | undefined): BuildChange | undefined {
  return act?.kind === "run" ? buildChangeOf(act.target) : undefined;
}
