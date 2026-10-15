// The list's repository headings. On All with several repositories served the list is grouped by repository, as the Sessions
// screen is (#2305): the listing's order, a last group for what has no listed repository, rows keeping their order inside.

import type { RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import type { Subject } from "./ship";

/** A repository a group is drawn for. */
export type Banding = { manifest: string; root: string; name: string };
export type Band = { key: string; label: string; subjects: Subject[] };

/** The group a subject with no listed repository falls into: #2305's own label. */
export const OTHER_BAND = "Other";

/** Only on All with more than one repository served. One picked, or one served, draws the list as it was. */
export function bandingOf(picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): readonly Banding[] | undefined {
  if (picked !== null) return undefined;
  const served = repositories.flatMap((one) => (one.manifest === undefined ? [] : [{ manifest: one.manifest.id, root: one.root, name: repositoryLabel(one, repositories) }]));
  return served.length > 1 ? served : undefined;
}

/** A pull request has no Manifest id, so its root says which repository it is in. */
const fits = (subject: Subject, one: Banding) => (subject.manifest === undefined ? subject.kind === "pr" && subject.root === one.root : subject.manifest === one.manifest);

/** The subjects in groups, in the listing's order; a repository with none is not drawn. */
export function bandsOf(subjects: readonly Subject[], banding: readonly Banding[]): Band[] {
  const listed = banding.map((one) => ({ key: one.manifest, label: one.name, subjects: subjects.filter((subject) => fits(subject, one)) }));
  const rest = subjects.filter((subject) => !banding.some((one) => fits(subject, one)));
  return [...listed, { key: "", label: OTHER_BAND, subjects: rest }].filter((band) => band.subjects.length > 0);
}
