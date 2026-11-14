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

**Rule.** Bridge never calls the store. Fleet does, and signs every call.
Why: Bridge holds one connection, to Fleet.

| Held | Keyed by | Value |
|---|---|---|
| Device | device id | public key, machine name |
| Claim | repository hash, kind and key hash | holder |
| Holder | holder kind, id and machine | the person's name, the Job's title, when it last renewed |
| Owner | key hash | holder |
| Runs | repository hash | the repository's facts, as JSON |
| Nonce | device id and nonce | when seen |

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

All paths are under `/v1`. A body is JSON, at most 4096 bytes, and 131072 for `PUT /runs/{repo}`. Every request but `enrol` is signed.

| Request | Body | Answers |
|---|---|---|
| `POST /enrol` | `token`, `public_key`, `machine` | `200 {device}` |
| `DELETE /devices/{device}` | none | `204`, `404 no_device` |
| `POST /claims` | `repo`, `kind`, `key`, `holder` | `200 {holder, renewed}` won, `409 {holder, renewed}` held |
| `GET /claims/{repo}/{kind}/{key}` | none | `200 {holder, renewed}`, `404 no_claim` |
| `DELETE /claims/{repo}/{kind}/{key}` | none | `204` |
| `DELETE /claims/{repo}/{kind}` | none | `204` |
| `POST /renewals` | `holder` as `kind`, `id`, `machine` | `200 {claims}` |
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

**Rule.** `POST /renewals` renews every claim of one holder (kind, id and machine) in one call, and answers how many it still holds.
`0` is a holder whose claims lapsed, and it claims them again.
Why: a renewal writes one `holders` row, however many claims the Job holds, because the free plan counts rows written ([Cost](#cost)).

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
| 429 | `enrol_rate_limited` | The address used its enrolments for the minute, [Limits](#limits) |
| 413 | `devices_full` | The account already has 16 devices |

**Rule.** Any device of an account may sign out another device of that account.
Why: a machine that is lost cannot sign itself out.

## Limits

The store is the default for every Armada user, so no one account may run up the owner's bill or fill his storage.

| Limit | Number | Refused with |
|---|---|---|
| Enrolments per address per minute | 10 | `429 enrol_rate_limited` |
| Devices per account | 16 | `413 devices_full` |
| Claims per account | 10,000 | `413 claims_full` |
| Owners per account | 10,000 | `413 owners_full` |
| Repositories with runs per account | 1,000 | `413 runs_full` |
| Facts per repository | 200 | `400 invalid_body` |
| Body of `PUT /runs/{repo}` | 131072 bytes | `413 too_large` |
| Verified requests per device per minute | 120 | `429 rate_limited`, `Retry-After: 60` |
| A claim after its holder's last renewal | 30 minutes | Dropped, `404 no_claim` |

**Rule.** `enrol` counts the client address, from `CF-Connecting-IP`, before it asks the forge, using the Workers Rate Limiting binding `ENROL_LIMIT` in `wrangler.jsonc`.
Why: every enrolment that gets past the body checks costs a call to the forge, and the binding keeps its count in Cloudflare, so no object is made for an address.

**Rule.** A request counts toward a device's minute only once its signature verifies, and the nonces of the last minute are the count.
Why: a stranger who knows a device id must not be able to use up that device's requests, and no second table is needed.

**Rule.** A table that is full refuses a new row with `413` and a name, never drops one silently, and still answers for a row it has.
A claim that exists answers `409` with its holder, and a pull request owner or a repository's runs that exist are updated.
Why: `409` already means a claim is held, so a full table needs a status Fleet does not read as that answer. Fleet reads `413` and `429` as a refusal and refuses the take.

**Rule.** A full table makes room again when a row is deleted or a device signs out.

**Rule.** An account's Durable Object makes its tables on the first enrolment and never for a request that does not verify.
A request for an account or a device that never enrolled is refused `401 unknown_device` and writes no table, no row and no nonce.
Why: the Worker finds the object from the header alone, so anyone can name an account number, and a name must not cost the owner storage.

**Rule.** A table added after accounts enrolled is made `IF NOT EXISTS` by the first verified request that reaches it, as well as at enrolment. `runs` is the first, then `holders`.
Why: an account that enrolled before the table existed never enrols again, and a verified device is an enrolled one.

**Rule.** A `claims` table keyed `(repo, red)` is dropped by the first verified request, with its rows, and made again keyed `(repo, kind, key)`.
Why: those rows name no person or title and nothing renews them, so the lapse would drop them anyway; the Mac that held one claims it again.
The October store was never deployed, so no live account holds one, and `apps/store/test/store.test.ts` opens a store built on it.

## Cost

The store runs on the Workers Free plan, read from Cloudflare's pricing and limits pages on 13 Nov 2026 (`developers.cloudflare.com/workers/platform/pricing/`, `/durable-objects/platform/pricing/` and the two `limits/` pages).
Each daily allowance is per Cloudflare account and resets at 00:00 UTC.

| Free plan | Allowance a day | What the store spends |
|---|---|---|
| Worker requests | 100,000 | One per call |
| Durable Object requests | 100,000 | One per call that reaches the account's object, and one per alarm |
| SQLite rows written | 100,000 | About 4 per call for its nonce, kept and deleted with its index, 1 more for a renewal and about 3 more for a claim won |
| SQLite rows read | 5,000,000 | A few per call |
| SQLite storage | 5 GB in all | Well under 1 MB at the limits above |
| CPU per request | 10 ms | Signature check and one or two statements |

At 10 live Jobs renewing every 5 minutes, 2,880 renewals a day spend about 2,880 of each request allowance and about 14,400 rows written.
A renewal that wrote each claim instead would spend about two rows per claim, so ten Jobs holding twenty files each would come to about 115,000, over the allowance.
The 2,880 is arithmetic, not a measurement of a day of Jobs.

A Worker over its daily requests answers Cloudflare's error 1027, and a Durable Object over an allowance fails the operation with an error.
Fleet reads either as the store not answering ([Failure](#failure)).
The Workers Paid plan is at least $5 a month, with 10 million Worker requests, 1 million Durable Object requests and 50 million rows written a month included.
The pages do not say whether the Rate Limiting binding `enrol` uses is on the Free plan, so that is checked at the first deploy.

## Failure

**Rule.** A store that cannot be reached, or answers with a refusal, refuses the take. Fleet never proceeds on a claim it could not make.
Why: a fix nobody else can see is the double fix this exists to stop. Decided by the owner, 13 Oct 2026.

**Rule.** `409` is an answer, not a failure. It names the holder, and the take is refused as taken elsewhere.

**Rule.** A repository with the switch off, one bound to no profile (or to None), and one bound to a profile the machine has signed out of, make no call.
Each repository calls as its own profile, [Account](../concepts/account.md), *Repositories*.

**Rule.** A repository's runs call as its own profile whether or not its Manifest turns the pull request owners on: choosing the place `cloud` is the switch.
A repository with no remote, one bound to no profile and one with no store address set refuse the choice, `422 fleet.run_cloud_unsigned`.

**Rule.** A store that cannot be reached, or answers with a refusal, on the choice of `cloud` refuses the choice and the old place stays.
Later, a write that fails keeps the facts in Fleet's own record, marked waiting with the reason, and the next turn's end writes again.
Why: runs are a note of how a repository runs, not a lock, so a store that is down costs a delay and never a fact. The reverse of a claim.

**Rule.** Fleet does not retry a `replayed_nonce`. A retry of any request signs again with a new nonce and a new time.

## Not deployed by this repository

The owner deploys the Worker with his own Cloudflare account and registers the forge's OAuth app.
A self-hosted store keeps the limits above and may change the numbers in `apps/store/src/rows.ts` and the `ratelimits` entry of `wrangler.jsonc`.
`FORGE_USER_URL` overrides the user endpoint, which is how the tests answer for the forge.
