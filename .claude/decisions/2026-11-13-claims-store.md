# How should Jobs on every Mac and teammate know what the others hold?

**Said 2026-11-13.** On 13 Nov a Job on the work Mac had its PR thrown out of the merge queue by a flaky test, and a Job on the home Mac was already fixing that test (#3565).
Neither knew about the other.
He approved the page "Claims store: Jobs on every Mac know what the others hold", revision 3, on 13 Nov, and said "Go on from it as written".
The page's three open questions are settled as the page's picks.

## What he chose

| Asked | He chose | The cost he took |
|---|---|---|
| What the store is for | A meeting point for claims, on his own Cloudflare account (bring your own Worker), deployed from a Bridge button | A Cloudflare account to deploy to. The free plan's request limits were not measured, and the paid plan starts at $5 a month |
| Who opts in | Per project. Each project picks its store in its own settings, or none, and a project with none sends nothing | Every project that wants claims has to pick a store, one by one |
| Keeping work apart | A different store per project: a work project points at the team's store, Armada at his | One store to set up per project that does not share |
| How a team shares a store | Joined by membership of a GitHub org or team, through the GitHub sign-in Armada already has | The token may need the `read:org` scope, which was not checked. The store allows 16 devices today, low for a team |
| What the store can read | Repository, item and file ids are hashed. Who holds the claim, their Mac and the Job title are readable | The list names an item, file or test only where the Mac has the repository and can match the hash. A teammate's claim in a repository he does not have shows its Job title alone |
| What is claimed | The tracker item first, then files (from the plan, plus each file Fleet sees the Job write). On a project with no tracker, files only. Red main and flaky tests too | None named |
| A person's work | The beads assignee. Jobs avoid it and say so | Other trackers come later, and until then a person claims in Bridge |
| On a clash | Wait on a tracker item, warn on files. The wait shows on the Job, and the Job requeues when the claim ends | On overlapping files both Jobs carry on, and only warn |
| How long a claim lives | Each claim ends on its own event (item closed, PR merged or closed, fix merged and main green). It lapses 30 minutes after its renewals stop | A Mac asleep that long loses its claims and takes them again when it wakes |
| A flaky test or a red main | Fleet detects it, claims it and always starts the fix Job, with no Autopilot gate. The ejected Job judges only when its own diff reaches the test. First claim wins | A wrong "your change does not reach this test" from the test map sends a fix Job at a test the PR broke. How often the map is wrong was not measured |
| The store is down | Jobs carry on, and their claims are sent when the store answers again | Until then no other Mac knows what a Job holds, and the Job shows a line saying its claims are not sent yet |
| Which claims list | A, grouped by who holds them (the page's pick) | Finding a given test means scanning every holder |
| Where the claims list lives | On the project's page (the page's pick) | One more tab on the project |
| What Deploy signs in with | The tunnel's Cloudflare sign-in, with a second sign-in only if Cloudflare refuses (the page's pick) | Possibly a second browser sign-in the first time |
| How it is set up and tested | From native Bridge only. Native screens are built by the Native Rebuild Orchestrator's Jobs from the fields each stage sends, and no stage issue builds one. Classic stays as it is | The store setting, the wait line and the claims list are built later, from fields the stages send |

## What it costs

Four stages of build, each one Fleet PR, and a Cloudflare Worker plus one SQLite Durable Object per account or team.
At 10 live Jobs renewing every 5 minutes that is 2,880 renewal calls a day, plus one call per claim taken and per check.
The 2,880 is arithmetic, not a measurement of a day of Jobs.

**Measured in stage 1, 13 Nov, against Cloudflare's pricing and limits pages.**
The Workers Free plan allows, per account and day: 100,000 Worker requests, 100,000 Durable Object requests, 100,000 SQLite rows written, 5 million rows read, and 5 GB of storage in all.
The paid plan starts at $5 a month.
Rows written is the allowance that binds: a renewal that rewrote every claim of ten Jobs holding twenty files each would spend about 115,000 a day.
So a renewal writes one row per Job, and the 2,880 renewals spend about 14,400 rows written.
`docs/contracts/central-store.md`, *Cost*, has the table.

| Stage | Build |
|---|---|
| 1 | Store key `(repo, kind, key)`, the holder's name, Mac and title, batch renewal, the 30-minute lapse alarm. Store tests. Cloudflare's free-plan limits measured against the pricing page before it lands |
| 2 | The store address moves from the machine onto the project. Deploy from Fleet to the project's Cloudflare account. Team Durable Objects with membership checked at enrol. Whether the tunnel's token can upload a Worker, and whether the GitHub token carries `read:org`, checked first |
| 3 | Claims for tracker items and files, beads assignees as held, wait on items, warn on files, requeue when a claim ends, carry on while the store is down |
| 4 | Queue ejection, the test map check, the Job asked when its diff reaches the test, the fix Job always started, first claim wins |

## Where it landed

The epic `armada-zasx` reads as this decision.
Each stage depends on the one before it.

| Issue | What |
|---|---|
| `armada-zasx.1` | Stage 1: claims of every kind |
| `armada-zasx.2` | Stage 2: a store per project |
| `armada-zasx.3` | Stage 3: items and files |
| `armada-zasx.4` | Stage 4: flaky tests and red main |

## Not measured

- Whether the token from the tunnel's Cloudflare sign-in can upload a Worker.
- Whether Armada's GitHub sign-in token carries `read:org` for the team membership check.
- The real claim and renewal rate under a day of Jobs.
- How often the test map's answer is wrong for a failing test.
