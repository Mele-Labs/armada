// Reported bugs on the Issues tab: where they are drawn, and what Start a Job puts in the sheet. **The band is Armada's own**: a
// report is about Armada, so it stands over the armada repository's issues, on All and when that repository is picked.

import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf, repositoryLabel } from "@armada/shell";

import type { ReportedBug } from "./model";

/** The Manifest id of the repository Armada is built in. */
const ARMADA = "armada";

/** The armada repository among those served, whether or not it is picked. */
export const armadaIn = (repositories: readonly RepositorySummary[]): { root: string; label: string } | undefined => {
  const one = repositories.find((it) => jobIdOf(it) === ARMADA);
  return one === undefined ? undefined : { root: one.root, label: repositoryLabel(one, repositories) };
};

/** Whether the band is drawn: under All, and with the armada repository picked. */
export const reportsShown = (picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): boolean => {
  const armada = armadaIn(repositories);
  return armada !== undefined && (picked === null || picked.root === armada.root);
};

/** A kept report as the brief of a Job: the title, what went wrong, then each part the agent gave. */
export function briefOf(report: ReportedBug): string {
  const part = (name: string, lines: readonly string[]) => (lines.length === 0 ? [] : ["", name, ...lines]);
  return [
    report.title,
    "",
    report.what,
    ...part("Steps", report.steps.map((one, at) => `${at + 1}. ${one}`)),
    ...part("Errors", report.errors),
    ...part("Logs", report.logs),
  ].join("\n");
}

/** The report a Job's mark opens: the first one not yet opened, else the first. */
export function reportOfJob(reports: readonly ReportedBug[], seen: readonly string[], sessionId: string): ReportedBug | undefined {
  const own = reports.filter((one) => one.session_id === sessionId);
  return own.find((one) => !seen.includes(one.id)) ?? own[0];
}
