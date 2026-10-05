// The one way a scenario is built from fixtures: a connected Fleet holding their
// Jobs, each with its own reads. Apart from `scenario.ts` so a row in
// `scenarios/` can build on it without importing the list it is listed in.

import type { ManifestSummary, RepositorySummary } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { connected } from "./moment";
import type { Scenario } from "./moment";

/** One of each, by manifest and id — two fixtures on one workflow list it once. */
function distinct<T>(items: T[], key: (item: T) => string): T[] {
  return [...new Map(items.map((item) => [key(item), item])).values()];
}

/**
 * The repository a Manifest is read from. **Invented where a fixture names only
 * the Manifest**: the root is made up, and nothing a screen draws reads it
 * except the rail's label, which is the Manifest's own `repository`.
 */
export function servedFrom(manifest: ManifestSummary): RepositorySummary {
  return manifest.id === repository().manifest?.id
    ? repository()
    // Named for its repository, not its id: the recording's Manifest has the
    // id `armada`, which put it on the base repository's own folder and ticked
    // both in the picker.
    : { root: `/Users/user/${manifest.repository}`, records_root: manifest.records_root, manifest };
}

/**
 * A connected Fleet holding these fixtures' Jobs, each with its own reads.
 *
 * `alsoServed` is served beside what the Manifests imply. **Every repository a
 * fixture reaches is set up by construction** — `servedFrom` builds one per
 * Manifest — so a repository nobody set up can only arrive this way.
 */
export function holding(
  name: string,
  says: string,
  fixtures: JobFixture[],
  { opens, alsoServed = [] }: { opens?: string; alsoServed?: RepositorySummary[] } = {},
): Scenario {
  const manifests = distinct(fixtures.flatMap((one) => one.manifests), (one) => one.id);
  return {
    name,
    says,
    state: connected(
      fixtures.map((one) => one.job),
      distinct(fixtures.flatMap((one) => one.workflows), (one) => `${one.manifest_id}/${one.id}`),
      [...manifests.map(servedFrom), ...alsoServed],
    ),
    reads: Object.fromEntries(fixtures.map((one) => [one.job.id, one])),
    opens,
  };
}

/**
 * The fixture, moved onto another id, handle and title.
 *
 * **Every `build/` fixture is the same Job** — one narrative at many
 * moments, `base.ts` says why — so on one Board they would be one row. Each
 * read that names its Job is moved with it, or the detail would draw a
 * different Job's reads as "not this one's".
 *
 * **The title is given, never taken from the builder's `name`.** That name is
 * the state the fixture demonstrates, and a row built from it read `running —
 * the drone is waiting for a person to allow a command` where a Job's title
 * reads `Cache the manifest read`. `EVERY_STATE_TITLES` is where the row's own
 * title is written.
 */
export function asRow(fixture: JobFixture, at: number, slug: string, title: string): JobFixture {
  const id = `01M2C1TJ8G00${String(at).padStart(2, "0")}EVERYSTATE00`;
  const renamed = { id, handle: `${at}-${slug}`, title };
  const job = { ...fixture.job, ...renamed };
  const moved = <Read extends { state: string }>(read: Read): Read =>
    "jobId" in read ? { ...read, jobId: id } : read;
  const watched = moved(fixture.watched);
  return {
    ...fixture,
    job,
    watched:
      watched.state === "read"
        ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, ...renamed } } }
        : watched,
    observed: moved(fixture.observed),
    journalled: moved(fixture.journalled),
    resources: moved(fixture.resources),
    history: fixture.history === undefined ? undefined : moved(fixture.history),
    jobDrones: fixture.jobDrones === undefined ? undefined : moved(fixture.jobDrones),
    recorded: {
      footprint: moved(fixture.recorded.footprint),
      handed: moved(fixture.recorded.handed),
      evidence: moved(fixture.recorded.evidence),
      diff: moved(fixture.recorded.diff),
      remarks: moved(fixture.recorded.remarks),
    },
  };
}
