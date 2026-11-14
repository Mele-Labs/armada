// A team store is this Worker with `TEAM` set: one Durable Object for everyone in a GitHub org, or in one team of it.
const ORG = /^[A-Za-z0-9-]{1,39}$/;
const SLUG = /^[A-Za-z0-9_-]{1,100}$/;

export type Team = {
  org: string;
  slug?: string;
};

/** `null` for a personal store, `"bad"` for a `TEAM` that is not an org or `org/team-slug`, so no request is served as a personal store by mistake. */
export function teamOf(env: Env): Team | null | "bad" {
  const text = env.TEAM;
  if (text === undefined || text === "") return null;
  const [org, slug, ...rest] = text.split("/");
  if (!org || !ORG.test(org) || rest.length > 0) return "bad";
  if (slug === undefined)
    return {
      org,
    };
  return SLUG.test(slug)
    ? {
        org,
        slug,
      }
    : "bad";
}

/** The name of the team's Durable Object, in lowercase because GitHub names are not case-sensitive. */
export const teamObjectName = ({ org, slug }: Team): string =>
  `team:${org}${slug === undefined ? "" : `/${slug}`}`.toLowerCase();
