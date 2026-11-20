# Central store

**Kind:** contract.
**Governs:** the Worker that holds an account's claims, pull request owners and the runs of its repositories: its wire, how a request is signed, what a row may hold, and what Fleet does when the Worker does not answer.
Both halves build to this page.
The Worker is `apps/store`; Fleet's side is its `Store` port, and `HttpStore` for runs.

Why it exists: *Fixing a red across machines* (`git show ecfffb2c1ebd:docs/capabilities/merge-line.md`), [Which machine owned a pull request](../concepts/session/what-is-kept.md#which-machine-owned-a-pull-request), and [What a repository runs](../concepts/kit.md#what-a-repository-runs), whose place `cloud` is this store.
The account is [Account](../concepts/account.md).

---

## Shape

**Rule.** One Durable Object holds one account, named by the account's numeric id.
Why: a Durable Object runs one request at a time, so a claim is atomic with no lock written.

**Rule.** A Worker deployed with the var `TEAM` is a team store: one Durable Object holds everyone who belongs to that GitHub org, or to one team in it, named `team:<org>` or `team:<org>/<team>` in lowercase.
`TEAM` is `org` or `org/team-slug`: an org of 1 to 39 letters, digits and `-`, a slug of at most 100 letters, digits, `-` and `_`.
A `TEAM` that is neither refuses every request `500 bad_team_config`, so a mistyped deploy is never open to everyone.
Every signed request on a team store goes to the team's object, whatever account the device id starts with.
Why: a team shares claims only if its members' devices sign into one object, and a device id keeps one shape on both kinds of store.
Decided by the owner, 13 Nov 2026 (`.claude/decisions/2026-11-13-claims-store.md`).

**Rule.** Bridge never calls the store. Fleet does, and signs every call.
Why: Bridge holds one connection, to Fleet.

| Held | Keyed by | Value |
|---|---|---|
| Device | device id | public key, machine name, kind (`mac` or `phone`), address |
| Claim | repository hash, kind and key hash | holder |
| Holder | holder kind, id and machine | the person's name, the Job's title, when it last renewed |
| Contender | repository hash, kind and key hash of a held claim, and the asker's holder kind, id and machine | the asker's name, title and `since`, when it last asked, and whether its claim's holder was told |
| Owner | key hash | holder |
| Runs | repository hash | the repository's facts, as JSON |
| Nonce | device id and nonce | when seen |
| Tally | the name of a table with a limit | how many rows it holds, kept by a trigger on each insert and delete |

## Rows

**Rule.** An owner holds opaque hashes and a holder, and nothing else.
A claim holds opaque hashes and a holder with the person's name and the Job's title in the clear.
Why: a work repository carries no trace of Armada, and the store is not trusted with the names of things.
Who holds a claim is readable so another Mac can say who, decided by the owner on 13 Nov 2026 (`.claude/decisions/2026-11-13-claims-store.md`).

**Rule.** The runs of a repository hold the commands and workspace paths its Jobs ran, in the clear, and nothing else about it: no name, no title, no token, no file content.
Why: how a repository runs should follow the signed-in account to every machine, as the Kit `runs` place `cloud`, and a hash cannot be run.
Decided by the owner, 19 Oct 2026. This reverses the earlier rule that the store holds nothing about any repository; the repository is still named only by its hash.

| Field | Holds | Refused |
|---|---|---|
| `repo`, `key` | A hash made on the machine, 32 to 128 characters of letters, digits, `_` and `-` | A name |
| `kind` | `item`, `file`, `test`, `red` or `pull_request` | Anything else |
| `holder.kind` | `session` or `job` | Anything else |
| `holder.id` | The short id, letters, digits, `.`, `_`, `-`, at most 64 | A title |
| `holder.machine` | The machine's name, at most 64 characters, no control characters | |
| `holder.since` | Whole seconds since the Unix epoch | A fraction, a negative, text |
| `holder.name` | On a claim only: the person, as their forge login, at most 64 characters, no control characters, empty where the Mac knows none | On an owner |
| `holder.title` | On a claim only: the Job's title, at most 200 characters, no control characters, empty where it has none | On an owner |

A fact is one thing a repository ran:

| Field | Holds | Refused |
|---|---|---|
| `workspace` | A relative path of 1 to 200 characters, `.` for the root, no control characters | An absolute path, a `..` segment |
| `kind` | `tests`, `lint`, `build`, `dev_server`, `typecheck` or `format` | Anything else |
| `command` | 1 to 400 characters, no control characters | A newline |
| `port` | Whole number, 1 to 65535. Absent when there is none | `0`, a fraction, text, `null` |
| `succeeded` | `true` or `false` | Anything else |
| `last_ran` | Whole milliseconds since the Unix epoch | A fraction, a negative, text |
| `source` | 1 to 32 characters of letters, digits, `_` and `-` | |

**Rule.** A body holds exactly the fields its request names, and so does a fact, except that `port` may be absent. A missing field and an extra one are both refused with `400 invalid_body`.

A pull request's key is a hash of its repository and number.
A claim is keyed by the repository's hash, its kind and the key's hash: for a red main, the commit main went red at.
The same key under two kinds is two claims.

## Wire

All paths are under `/v1`. A body is JSON, at most 4096 bytes, and 131072 for `PUT /runs/{repo}`. Every request but `enrol`, `sign_in` and `sign_in/poll` is signed.

| Request | Body | Answers |
|---|---|---|
| `POST /enrol` | `token`, `public_key`, `machine` | `200 {device}` |
| `POST /sign_in` | `client_id` | `200 {device_code, user_code, verification_uri, interval, expires_in}` |
| `POST /sign_in/poll` | `client_id`, `device_code`, `public_key`, `machine` | `200 {device}`, `202 {waiting}`, `410 expired`, `403 denied` |
| `GET /devices` | none | `200 {devices}`, each `device` and `machine`, oldest enrolment first |
| `PUT /presence` | `kind`, `address` | `204` |
| `GET /peers` | none | `200 {devices}`, each `device`, `machine`, `kind`, `address` and `public_key`, oldest enrolment first |
| `DELETE /devices/{device}` | none | `204`, `404 no_device` |
| `POST /claims` | `repo`, `kind`, `key`, `holder` | `200 {holder, renewed}` won, `409 {holder, renewed}` held |
| `GET /claims` | none | `200 {holders}`, each `holder`, `renewed`, `claims` and `waits` |
| `GET /claims/{repo}/{kind}/{key}` | none | `200 {holder, renewed}`, `404 no_claim` |
| `DELETE /claims/{repo}/{kind}/{key}` | none | `204` |
| `DELETE /claims/{repo}/{kind}` | none | `204` |
| `POST /renewals` | `holder` as `kind`, `id`, `machine` | `200 {claims, contested}` |
| `PUT /owners/{key}` | `holder` | `204` |
| `GET /owners?keys=a,b,c` | none | `200 {key: holder}` |
| `DELETE /owners/{key}` | none | `204` |
| `PUT /runs/{repo}` | `facts` | `204` |
| `GET /runs/{repo}` | none | `200 {facts}`, `404 no_runs` |

**Rule.** A claim is claim-if-free. The first call for a repository, kind and key wins and every later one is told the winner's holder, name, title and `renewed`.
Why: two machines that reach the same thing must not both work on it, and the one refused can name who has it.

**Rule.** Giving a claim back and taking it over are the same delete. A take over deletes the row, then claims.
Why: the store does not know whose claim it is; the machine does.

**Rule.** Deleting a row that is not there answers `204`.

**Rule.** A claim lapses 30 minutes after its holder last renewed.
A won claim renews its holder; so does `POST /renewals`.
An alarm on the account's object drops the lapsed holders and their claims, and every claims route reads a lapsed claim as gone before the alarm has run.
Why: a Mac that stops, sleeps or loses its network must not hold work forever.
A Mac asleep that long takes its claims again when it wakes.

**Rule.** A claims route or a renewal drops lapsed rows only once the earliest renewal or ask can have lapsed, and otherwise reads no holder or contender to find out.
The object keeps that time in memory, and an object that has just woken takes its alarm's time, which is never later.
Why: dropping lapsed rows reads every holder and contender, and on every call it spent the free plan's 5,000,000 rows read in a day ([Cost](#cost)).

**Rule.** `POST /renewals` renews every claim of one holder (kind, id and machine) in one call, and answers how many it still holds.
`0` is a holder whose claims lapsed, and it claims them again.
Why: a renewal writes one `holders` row, however many claims the Job holds, because the free plan counts rows written ([Cost](#cost)).

**Rule.** A `POST /claims` answered `409` keeps its asker as a contender of the claim, unless the asker is the holder itself (same holder kind, id and machine).
A renewal answers `contested`, a list of `{repo, kind, key, holder}` for every contender on a claim the renewing holder still holds that was not told yet, with `holder` spelled as a `409` spells it.
It marks those rows told and keeps them, so each contender is told once and stays while its asker keeps asking.
A contender that asks again updates its name, title and time and keeps its told flag, and no second row is kept.
A contender lapses 30 minutes after its asker last asked, and every claims route and `POST /renewals` read it as gone before the alarm has run.
The alarm drops it with the lapsed holders.
A contender's rows also go with its claim when the claim is given back, cleared by kind, taken over or lapsed.
An account holding 10,000 contenders keeps no new one, and still answers the `409`; a contender is a warning, so a refusal there would hide who holds the claim.
Why: when two Jobs claim the same file both are told the file and the other holder, and the first holder reaches the store only by renewing.
A Job that waits on a claim stays visible as waiting for as long as it asks, so the claims list can say what a pull request waits on.
Decided by the owner, 13 Nov 2026 (`.claude/decisions/2026-11-13-claims-store.md`).

**Rule.** `GET /claims` answers every live claim of the object in `holders`, one group for each holder (kind, id and machine) that holds a live claim or is a live contender.
A group is `holder` as `kind`, `id`, `machine`, `name` and `title`, `renewed`, `claims` and `waits`.
`renewed` is the holder row's renewal time. A group that only contends has no holder row, so it takes `renewed`, `name` and `title` from its newest contender.
`claims` holds every claim the holder holds as `repo`, `kind`, `key` and `since`, in order of kind, then `since`, then key.
`waits` holds every claim held by someone else that the holder is a live contender on, as `repo`, `kind`, `key` and `holder` spelled as a `409` spells it, in order of kind, then key.
Groups are in order of name, then machine, then holder kind, then id.
A lapsed holder, its claims and a lapsed contender are absent, and the read writes nothing.
Why: the claims list says who holds each thing and who waits on it, from one signed call.

**Rule.** Green on main clears every `red` claim under the repository, `DELETE /claims/{repo}/red`.
Its other claims stand.

**Rule.** `GET /owners` answers only the keys that have a row, at most 100 keys, and a key with none is absent.

**Rule.** `PUT /runs/{repo}` replaces every fact held for the repository. Fleet reads the set, merges its own into it and writes the whole set back, as it does for a file.
Why: a store that merged would need to know which facts a person dropped, and only the machine does.

**Rule.** A set holds at most 200 facts and no two with the same `workspace`, `kind` and `command`. A set that breaks either is refused whole, with `400 invalid_body`.

**Rule.** An empty set is accepted and removes the row, so a repository with no facts holds no row. Nothing else deletes runs.
Why: a table at its limit makes room again by the same call that empties a repository.

**Rule.** Facts are answered in the order they were put.
The body limit is the tighter bound: 200 facts that each use every character allowed come to about 140 KB, over 131072, and are refused `413 too_large`.

## Signing

**Rule.** The signature is ECDSA P-256 over SHA-256 of the signed text, written as the raw 64 bytes r then s, in lowercase hex.

**Rule.** The signed text is `method + path_and_query + time + nonce + hex(sha256(body))`, joined with nothing between.
The path and query are exactly as sent, and an empty body hashes as the empty string.

| Header | Holds |
|---|---|
| `X-Armada-Device` | The device id `enrol` answered |
| `X-Armada-Time` | Whole seconds since the Unix epoch, digits only |
| `X-Armada-Nonce` | 16 to 64 characters of letters, digits, `_` and `-`, new for each request |
| `X-Armada-Signature` | The signature in hex |

**Rule.** A request is refused when its time is more than 60 seconds from the Worker's clock, in either direction.

**Rule.** A nonce is kept for two minutes per device. A request that repeats one is refused, whatever its signature.
Why: the Worker holds the replay set, so no process map is needed on either side.
A device's request clears that device's old nonces, through an index on device and time, and the alarm clears those of a device that stopped calling.

**Rule.** A nonce is recorded only after the signature verifies.
Why: an unsigned flood cannot fill the table.

`apps/store/test-vectors/signing.json` carries one fixed request, its signed text, its body hash and one valid signature under a test-only key.
Both halves test against it, and a signature is checked by verifying it, since ECDSA signs differently each time.
The helpers are `apps/store/src/signing.ts`.

### Refusals

| Status | `error` | When |
|---|---|---|
| 401 | `unsigned` | A header is missing or malformed |
| 401 | `unknown_device` | The device is not enrolled, or was signed out |
| 401 | `stale_time` | The time is outside the skew |
| 401 | `bad_signature` | The signature does not verify |
| 401 | `replayed_nonce` | The nonce was used already |
| 400 | `invalid_body`, `invalid_key` | A body or a key breaks Rows |
| 404 | `no_runs` | The repository has no runs |
| 413 | `too_large` | The body is over the limit |
| 413 | `devices_full`, `claims_full`, `owners_full`, `runs_full` | The account holds its most of that row, [Limits](#limits) |
| 429 | `rate_limited` | The device used its requests for the minute, [Limits](#limits) |
| 404 | `no_route` | No such path |
| 405 | `bad_method` | The path does not take that method |

## Enrolling

**Rule.** `enrol` takes a device-flow access token once, reads the account's numeric id from the forge's user endpoint, registers the device's public key under that account, and drops the token.
Why: the identity is the numeric id, and a token the store never keeps cannot leak from it.

**Rule.** The token is never stored, logged or passed to the Durable Object, and the Worker keeps nothing else the forge answered.

**Rule.** `public_key` is the device's P-256 public key as base64 of its SPKI encoding.
The device id is the account id, a dot, and 32 hex characters, so the Worker finds the account's object from the header alone.

| Status | `error` | When |
|---|---|---|
| 401 | `forge_refused` | The forge rejected the token |
| 502 | `forge_unreachable` | The forge did not answer |
| 502 | `forge_unreadable` | The forge answered with no usable numeric id |
| 400 | `invalid_public_key` | The key is not a P-256 public key |
| 429 | `enrol_rate_limited` | The address used its enrolments and sign-in starts for the minute, [Limits](#limits) |
| 413 | `devices_full` | The account already has 16 devices, or the team store 256 |

**Rule.** Any device of an account may sign out another device of that account.
Why: a machine that is lost cannot sign itself out.

**Rule.** `GET /devices` answers every device of the object the caller signs into, with the Mac's name it enrolled as and never its key.
Why: the store setting says which Macs are signed in.

## Presence and peers

**Rule.** A device row holds `kind`, `mac` or `phone`, and `address`, where it can be reached.
`enrol` registers a `mac` with no address, and a phone signs in as a `phone` ([A phone's sign-in](#a-phones-sign-in)).
`PUT /presence` takes exactly `kind` and `address` and sets the caller's own row, and no other device's.
Why: a Mac's tunnel address changes, and the devices that are his find each other by it.

**Rule.** `address` is empty, or an origin: `https://`, a host, and a port if there is one from 1 to 65535, with no userinfo, path, query or fragment, at most 200 characters.
Anything else is refused `400 invalid_body`.
Why: the address is somewhere another device will call, so it holds nothing it could be told to call beyond a host.

**Rule.** `GET /peers` answers every device whose id starts with the caller's account id and a dot, which is the part of the caller's device id before its first `.`, except the caller, with its `public_key` as it enrolled.
A team store holds every member's devices in one object, and it never lists one member's devices to another.
Why: devices are listed only within the forge account that enrolled them, so a device that finds a peer and is vouched for by the store is his and nobody else's.
Decided by the owner, 13 Nov 2026 (`armada-zasx.5`).

## A phone's sign-in

**Rule.** The Worker runs the forge's device flow for a phone: `POST /sign_in` asks the forge for a code, and `POST /sign_in/poll` asks whether it was entered.
Why: the forge's sign-in endpoints answer a page on another origin with no CORS headers, so a phone's page cannot call them, and the token must not reach the phone either.

**Rule.** `sign_in` takes `client_id`, 1 to 64 letters, digits, `.`, `_` and `-`, posts it to the forge's device-code endpoint, and answers the forge's `device_code`, `user_code`, `verification_uri`, `interval` and `expires_in`.
It is counted by `ENROL_LIMIT` as an enrolment is, before it calls the forge.

**Rule.** `sign_in/poll` takes `client_id`, `device_code` of 1 to 128 printable characters, and `public_key` and `machine` as `enrol` does.
It asks the forge's access-token endpoint once per call, and the phone paces itself by `interval`.
A poll is not counted by `ENROL_LIMIT`.
Why: the forge's own `slow_down` bounds how often a code is asked about, and a limit of ten a minute would refuse a phone that is only waiting.

**Rule.** Once the forge answers a token, the poll does what `enrol` does from the token on: the account's id from the user endpoint, the membership check on a team store, and the device registered, with the kind `phone`.
It answers `200 {device}`.
The token is never stored, logged or passed to the Durable Object, and the phone never sees it.

| Status | `error` or `waiting` | When |
|---|---|---|
| 202 | `waiting: pending` | The code has not been entered yet |
| 202 | `waiting: slow_down`, with `interval` | The phone polled too soon: the forge's longer `interval`, or 10 seconds where it names none |
| 410 | `expired` | The code ran out |
| 403 | `denied` | The person refused |
| 400 | `forge_refused`, with `reason` where the forge gave one | The forge refused the client id or the code, such as `device_flow_disabled` |
| 502 | `forge_unreachable`, `forge_unreadable` | The forge did not answer, or answered with no usable code or token |

A poll that reaches a token answers what `enrol` answers for it, including `not_a_member` and `org_restricted` on a team store.

## CORS

**Rule.** Every answer under `/v1`, a refusal included, carries `Access-Control-Allow-Origin: *` and `Access-Control-Expose-Headers: Retry-After`.
An `OPTIONS` request to any `/v1` path answers `204` with `Access-Control-Allow-Methods: GET, POST, PUT, DELETE`, `Access-Control-Allow-Headers` naming `Content-Type` and the four `X-Armada-*` signing headers, and `Access-Control-Max-Age: 600`.
No answer allows credentials.
Why: a phone's page calls the store from another origin, every call is signed or holds a one-time code, none uses a cookie, and a refusal it cannot read is a failure it cannot name.

**Rule.** On a team store, enrol also asks the forge whether the token's account belongs: `GET /user/memberships/orgs/<org>` for an org, `GET /orgs/<org>/teams/<team>/memberships/<login>` for a team, at the origin of the user endpoint.
A member is a `200` whose `state` is `active`.
The token is read for these calls and dropped, as for the user endpoint, and a refused enrolment registers nothing.

| Status | `error` | When |
|---|---|---|
| 403 | `not_a_member` | The forge answered `404`, or a membership that is not active |
| 403 | `org_restricted` | The forge answered `403`: the org's OAuth app restrictions block the read until an org owner approves the app |
| 401 | `forge_refused` | The membership call answered `401` |
| 502 | `forge_unreachable` | The membership call did not answer, or failed |
| 502 | `forge_unreadable` | The user endpoint named no login, on a team store |
| 500 | `bad_team_config` | `TEAM` is not an org or `org/team` |

## Limits

The store is the default for every Armada user, so no one account may run up the owner's bill or fill his storage.

| Limit | Number | Refused with |
|---|---|---|
| Enrolments and sign-in starts per address per minute | 10 | `429 enrol_rate_limited` |
| Devices per account | 16 | `413 devices_full` |
| Devices per team store | 256 | `413 devices_full` |
| Claims per account | 10,000 | `413 claims_full` |
| Owners per account | 10,000 | `413 owners_full` |
| Contenders per account | 10,000 | None: the `409` is still answered and no row is kept |
| Repositories with runs per account | 1,000 | `413 runs_full` |
| Facts per repository | 200 | `400 invalid_body` |
| Body of `PUT /runs/{repo}` | 131072 bytes | `413 too_large` |
| Verified requests per device per minute | 120 | `429 rate_limited`, `Retry-After: 60` |
| A claim after its holder's last renewal | 30 minutes | Dropped, `404 no_claim` |
| A contender after its asker last asked | 30 minutes | Dropped, absent from `waits` |

**Rule.** `enrol` and `sign_in` count the client address, from `CF-Connecting-IP`, before they ask the forge, using the Workers Rate Limiting binding `ENROL_LIMIT` in `wrangler.jsonc`.
Why: every enrolment or sign-in start that gets past the body checks costs a call to the forge, and the binding keeps its count in Cloudflare, so no object is made for an address.

**Rule.** A Worker deployed without `ENROL_LIMIT` admits every enrolment and counts none.
A deploy goes without it only where Cloudflare refused it, and the project's file then says `joins_unlimited: true`.
Why: the owner chose to try the limit and go without it, 14 Nov 2026, since Cloudflare's pages do not say whether the account's plan has it.

**Rule.** A request counts toward a device's minute only once its signature verifies, and the nonces of the last minute are the count.
Why: a stranger who knows a device id must not be able to use up that device's requests, and no second table is needed.
The object reads a device's nonces of the last minute once, then keeps their times in memory.

**Rule.** A limit is checked against the table's tally, never by counting the table.
Why: a `COUNT(*)` reads every row, and the free plan counts each one ([Cost](#cost)).

**Rule.** A table that is full refuses a new row with `413` and a name, never drops one silently, and still answers for a row it has.
A claim that exists answers `409` with its holder, and a pull request owner or a repository's runs that exist are updated.
Why: `409` already means a claim is held, so a full table needs a status Fleet does not read as that answer. Fleet reads `413` and `429` as a refusal and refuses the take.

**Rule.** A full table makes room again when a row is deleted or a device signs out.

**Rule.** An account's Durable Object makes its tables on the first enrolment and never for a request that does not verify.
A request for an account or a device that never enrolled is refused `401 unknown_device` and writes no table, no row and no nonce.
Why: the Worker finds the object from the header alone, so anyone can name an account number, and a name must not cost the owner storage.

**Rule.** A table added after accounts enrolled is made `IF NOT EXISTS` by the first verified request that reaches it, as well as at enrolment. `runs` is the first, then `holders`, then `contenders`.
Why: an account that enrolled before the table existed never enrols again, and a verified device is an enrolled one.

**Rule.** A `contenders` table made before it had `told` gets `told INTEGER NOT NULL DEFAULT 0` added by the first verified request, and keeps its rows, none of them told.
Why: an account that enrolled before the column existed never enrols again, and a contender it holds is told once on its holder's next renewal.

**Rule.** A `devices` table made before it had `kind` and `address` gets `kind TEXT NOT NULL DEFAULT 'mac'` and `address TEXT NOT NULL DEFAULT ''` added by the first verified request, and keeps its rows, each a Mac with no address.
Why: an account that enrolled before the columns existed never enrols again, and its Macs are Macs.

**Rule.** A `claims` table keyed `(repo, red)` is dropped by the first verified request, with its rows, and made again keyed `(repo, kind, key)`.
Why: those rows name no person or title and nothing renews them, so the lapse would drop them anyway; the Mac that held one claims it again.
The October store was never deployed, so no live account holds one, and `apps/store/test/store.test.ts` opens a store built on it.

**Rule.** An account made before `tallies` gets the table, its triggers and one count of each table with a limit from the first verified request, and its nonce index on time alone becomes one on device and time.
Why: an account that enrolled before them never enrols again, and that one count is the last time the store reads a whole table to check a limit.
`apps/store/test/reads.test.ts` opens a store built before them.

## Cost

The store runs on the Workers Free plan, read from Cloudflare's pricing and limits pages on 13 Nov 2026 (`developers.cloudflare.com/workers/platform/pricing/`, `/durable-objects/platform/pricing/` and the two `limits/` pages).
Each daily allowance is per Cloudflare account and resets at 00:00 UTC.

| Free plan | Allowance a day | What the store spends |
|---|---|---|
| Worker requests | 100,000 | One per call |
| Durable Object requests | 100,000 | One per call that reaches the account's object, and one per alarm |
| SQLite rows written | 100,000 | About 4 per call for its nonce, kept and deleted with its index, 1 more for a renewal, about 4 more for a claim won and about 2 more for a new contender, each counting its tally, and 1 more for a `409` already kept |
| SQLite rows read | 5,000,000 | Fewer than 10 per claim or renewal, whatever the account holds, and every holder and contender once each time something lapses |
| SQLite storage | 5 GB in all | Well under 1 MB at the limits above |
| CPU per request | 10 ms | Signature check and one or two statements |

At 10 live Jobs renewing every 5 minutes, 2,880 renewals a day spend about 2,880 of each request allowance and about 14,400 rows written.
A renewal that wrote each claim instead would spend about two rows per claim, so ten Jobs holding twenty files each would come to about 115,000, over the allowance.
The 2,880 is arithmetic, not a measurement of a day of Jobs.

Rows read spent the allowance first, on 18 Nov 2026: Cloudflare warned at 90% of 5,000,000.
Each claims call and renewal counted the claims table and scanned the holders and contenders, about 5,200 rows a call at 2,000 claims each with a contender.
`apps/store/test/reads.test.ts` measures the rows a call reads now.

A Worker over its daily requests answers Cloudflare's error 1027, and a Durable Object over an allowance fails the operation with an error.
Fleet reads either as the store not answering ([Failure](#failure)).
The Workers Paid plan is at least $5 a month, with 10 million Worker requests, 1 million Durable Object requests and 50 million rows written a month included.
The pages do not say whether the Rate Limiting binding `enrol` uses is on the Free plan, so each deploy tries it, and goes again without it where Cloudflare refuses it ([Limits](#limits)).

## A store per project

**Rule.** A store is named by a project, never by the machine.
A project's file names it under `store:`, as `address`, and `team` where it is a team store, and `worker` and `deployed` where a Mac of his deployed it.
A project with none sends nothing.
Why: a work project points at the team's store and Armada at his own, decided by the owner on 13 Nov 2026 (`.claude/decisions/2026-11-13-claims-store.md`).

**Rule.** A project's `store:` block also names the login that joins it, and a repository may override the login.
A project with a store and no login sends nothing, and says so.
Claims go as that login, never as a profile matched through the repository's owner, [Account](../concepts/account.md), *Which login a project joins as*.

**Rule.** A project of this Mac's that names a store and no login takes the one profile here that already enrolled at that store, and Fleet writes it into the project's file once, where the file is kept, when it reads the projects.
None enrolled, or two or more, leaves it as it is and the sentence above stands, and a copy of another Mac's project is never written.
Why: every project file written before login naming has a store and no login, and without this each of them stopped sending claims the day it landed.

**Rule.** A repository calls the store the projects holding it name, read when the call is made.
A repository in no project, or in projects that name none, makes no call, and that holds for what was still to send when a store was taken out.
A repository in two projects that name different stores makes no call either, and Fleet says so on its log.
Why: nothing says which store the person meant, and a claim sent to the wrong one is a claim his other Macs never see.

**Rule (20 Nov).** A copy of another Mac's project counts as a project here for that rule: it keeps the holder's store and the login the holder picked, and a repository on this Mac that the copy lists calls the copy's store as that login.
A held project and a copy that name different stores for one repository make no call, as two held projects do.
Joining is per Mac: this Mac claims with its own profile for that login, enrolled at the store in its own `stores/<hash>` folder, and enrols it the first time a call or a read of the copy's store needs it, writing nothing to the project's file.
This Mac not signed in as that login sends nothing, and the copy's store setting says so.
Setting or deploying a copy's store stays the holder's.
Why: the owner shares one project between his Macs, and a repository that is only in the copy on one Mac sent no claim at all, with the setting reading "Not joined here".

**Rule.** A profile keeps a key and a device id for each store it enrolled with, in `stores/<the first 16 hex of the address's sha256>` under the profile's folder.
Why: one account enrolled with his own store and his team's is two devices, one in each.

## A feature library stays on this Mac

**Rule.** A project's feature library is kept in this Mac's own store and the Worker holds none of it.
Why: sharing it with teammates through a project's store has not been designed, and a table built before its owner has designed it is a table to unbuild.

**Rule.** A shared library would hold its words in the clear: every feature's area, slug, title and one line, its path prefixes and entry points, the four sections a Job wrote with the Job's id, the time and the commit, and the links to Jobs, pull requests, merges and artifacts by their ids and titles.
Why: the owner chose words in a shared store in the clear (`.armada/documents/feature-catalog.html`, *Settled so far*), so no field here is sealed and none is withheld.

Every row carries a project's id and an id minted whole, never a position, so the rows can be copied to a store unchanged.
Fleet's side is `crates/store/src/features.rs`.
Nothing in `apps/store` reads or writes it.

## Deploying a store

A store is deployed from a project's store setting in Bridge, onto his own Cloudflare account.
Fleet uploads the Worker bundled in `apps/store/dist/worker.js`, turns on its `workers.dev` address and writes the store into the project's file, then joins this Mac to it.
`pnpm --dir apps/store bundle` makes the bundle, and the Check `bundle_current` in `apps/store/armada.yml` refuses one older than its source.

| Store | Worker | Durable Object |
|---|---|---|
| His own | `armada-store` | One per account |
| A team's | `armada-store-<org>[-<team>]`, with `TEAM` set to `org` or `org/team` | One for the team |

**Rule.** A deploy signs in to Cloudflare in this order: a token he pasted before for the tunnel's account, then the tunnel's own sign-in.
Where Cloudflare refuses every one, the deploy changes nothing and answers `needs_token` with Cloudflare's page that makes a token with the rights it needs.
A token he pastes is the only one tried, and is kept in the Keychain under `armada-cloudflare-token` for the account once a deploy with it works.
Why: the owner wanted the tunnel's sign-in reused and a second sign-in only as the fallback, 13 Nov 2026.
On 14 Nov 2026 the tunnel's sign-in was refused on every Workers endpoint, so the first deploy on a Mac asks for a token.

**Rule.** No token is written to a file, a log or an answer.
The tunnel's is read from `cloudflared`'s certificate when a deploy runs, and a pasted one lives in the Keychain alone.

**Rule.** A deploy of a Worker that is already up replaces its script and keeps its Durable Objects and their rows.

**Rule.** Joining a store enrols the login the project names and each login a repository of it overrides with, with `gh`'s token for the profile's login, read once and dropped.
A profile already enrolled there is left as it is.
A join that is refused writes nothing, and a deploy whose join is refused still names the store and says why.
Why: the owner chose `gh`'s sign-in for joins, 14 Nov 2026, since `gh`'s token carries `read:org` for a team store's check.
An org that restricts OAuth apps keeps that check from reading the membership, and the store refuses `org_restricted`.

**Rule.** Signing in to GitHub makes a profile and enrols with no store.
Signing out deletes the profile's device from every store it joined.

## Failure

**Rule.** A store that cannot be reached, or answers with a refusal, refuses the take. Fleet never proceeds on a claim it could not make.
Why: a fix nobody else can see is the double fix this exists to stop. Decided by the owner, 13 Oct 2026.

**Rule.** A Job's own claims, its tracker item, its files and its pull request, are the exception: a store that cannot be reached, or refuses, stops no Job.
Fleet keeps what each live Job holds and sends what the store has not taken once it answers, and the Job's record says its claims are not sent until then.
No call is logged to replay; a claim taken and given back while the store is down is never sent.
Why: the owner ruled on 13 Nov 2026 that Jobs carry on while the store is down (`.claude/decisions/2026-11-13-claims-store.md`).
The Job's claims and the waits they make are [Fleet](../concepts/fleet.md#claims-a-job-holds).

**Rule.** A Job's own claims, a red main's and a failing test's call the store of the projects holding its repository, whatever the Manifest says of pull request owners: naming a store in the project is the switch.
Why: a red main's or a flaky test's fix Job always starts, so two Macs that skipped the claim would both start one. The owner ruled on 13 Nov 2026 that the first claim wins.

**Rule.** `409` is an answer, not a failure. It names the holder, and the take is refused as taken elsewhere.

**Rule.** A repository with the switch off, one in no project that names a store, one in a project that names a store and no login, and one whose login the machine has signed out of, make no call.
Each repository calls as the login its project names, or its own override, [Account](../concepts/account.md), *Which login a project joins as*.

**Rule.** A repository's runs call as the login its project names, or its override, whether or not its Manifest turns the pull request owners on: choosing the place `cloud` is the switch.
A repository with no remote, one with no login and one in no project that names a store refuse the choice, `422 fleet.run_cloud_unsigned`.

**Rule.** A store that cannot be reached, or answers with a refusal, on the choice of `cloud` refuses the choice and the old place stays.
Later, a write that fails keeps the facts in Fleet's own record, marked waiting with the reason, and the next turn's end writes again.
Why: runs are a note of how a repository runs, not a lock, so a store that is down costs a delay and never a fact. The reverse of a claim.

**Rule.** Fleet does not retry a `replayed_nonce`. A retry of any request signs again with a new nonce and a new time.

## Not deployed by this repository

Nothing in this repository deploys the Worker to a shared account: each person deploys his own from Bridge, [Deploying a store](#deploying-a-store).
A self-hosted store keeps the limits above and may change the numbers in `apps/store/src/rows.ts` and the `ratelimits` entry of `wrangler.jsonc`.
`FORGE_USER_URL` overrides the user endpoint, and `FORGE_LOGIN_URL` the address a phone's sign-in starts at, which is how the tests answer for the forge.
