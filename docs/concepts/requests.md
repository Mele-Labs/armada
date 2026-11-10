# Requests

**What it is:** An HTTP request sent through Fleet, by the person from Bridge or by a Job's agent through its tool, from collections of saved requests and environments of variables, some of them secret.

---

**Kind:** Concept.

Since 19 Oct 2026; collections and environments since 21 Oct 2026.
The wire is `docs/practices/protocol.md`, *Requests*, and the nine operations in `crates/ipc/operations/`: `get_requests`, `save_request_collection`, `save_request_environment`, `delete_request_environment`, `save_request_to_collection`, `send_request`, `send_requests`, `list_request_runs` and `get_request_run`.
Bridge's panel is `apps/desktop/src/renderer/src/shared-panel/requests.tsx`.

## Who uses it

| Who | Does |
|---|---|
| The person | Writes collections and environments, sends from the panel, opens a request from a thread |
| A Job's agent | Reads the names, sends, and saves a request into a collection |

An agent checking its own work against a running server or a staging address is the reason Requests exists.
Fleet sends every request, so the person and the agent meet the same rules.

## Collections

> **Rule.** A collection is a name, the headers every request in it shares, and the requests.
> Why: a base address and an `Authorization` header are written once and sent with each.

A request is a name, a method, an address, its own headers and a body.
Its own headers sit under the collection's, and a header of the same name in the request wins.
The panel draws the collection's headers read only above the request's own.

## Environments and secrets

> **Rule.** An environment is a name and variables, each a plain value or a secret.
> Why: the same request is sent to a Job's server, to staging or to a local address by changing only the environment.

A plain variable is the text the person typed, and it shows.
A secret is kept in the Keychain and nowhere else, and it is never shown back: the panel draws it as Set, with Replace beside it.
Fleet reaches the Keychain through `/usr/bin/security`, so a rebuilt Fleet reads what it kept without a prompt (`docs/spikes/034-can-fleet-keep-a-cookie-in-the-keychain.md`).

| Environment | From |
|---|---|
| Named by the person | The person's own file or the repository's |
| `server:<name>` | A server the Job runs, read only, with one variable, `base` |

A `server:` environment exists only with a Job open.

## What an agent sees and never reads

> **Rule.** An agent names a secret as `{{name}}`, and Fleet fills it in at the send.
> Why: an agent that could read a secret could send it to a server it runs.

| An agent may | An agent never |
|---|---|
| List collections, environments and variable names | Read a secret's value |
| Send a saved request or its own, naming an environment | Write an environment or a secret |
| Save a request into a collection | Delete a request, or change a collection's headers |

Fleet fills every `{{name}}` in the address, the headers and the body.
A `{{name}}` it cannot fill is sent as written, and the answer names it.
**A secret of six characters or more is hidden in everything answered, written or told**, and shows as `{{name}}`.
A shorter value is not hidden, so it is never matched inside ordinary words.

Redirects are not followed, so a secret goes to the host named and nowhere else.
The answer an agent reads drops `set-cookie` and is cut at 32 KiB.
Fleet's HTTP surface is open to any process on this Mac, so a secret is usable and never readable by anything local.

## Many at once

> **Rule.** An agent sends a body from a file with `body_file`, and many requests with `send_requests`, and the secret is still filled in Fleet.
> Why: a Job that cannot see a secret still has to try four hundred requests, and one call for each is four hundred turns.

`send_request` takes `body_file` in place of `body`: an absolute path that Fleet reads as the body.
**Fleet reads only the Job's scratch folder and its slot.**
A path is resolved first, so `..` and a link out are judged by where they land, and a path elsewhere, a file over 64 KiB or one that is not text is refused.

`send_requests` takes a template, a list of `body_files` and a `vary` of one name and its values, and sends one request for each file and each value.
Twenty files and twenty values are four hundred requests, and a call is at most 1000.
`{{<name>}}` in the address, the headers and each file is replaced by the value first, and every other `{{name}}` is then filled from the environment as in `send_request`.
Each answer is saved whole in the Job's scratch folder as `<file stem>-<value>`, secrets as `{{name}}`.
The answer is one short line for each request with its status and the saved file's name, and never a body.

| Limit | Default | Most |
|---|---|---|
| Requests in flight | 5 | 10 |
| Failures in a row before the rest are skipped | 5 | no most |

A failure is a 4xx, a 5xx or an error, counted as answers arrive, and a success starts the count again.
The requests not sent are listed as skipped, and the note says why.
A batch writes no `requested` row, since four hundred rows would bury the thread, and the Job is told nothing.

## Where each is kept

> **Rule.** Each repository has the person's own file and the repository's committed file, and the person's own wins.
> Why: it is the rule every piece of configuration follows (`docs/design/who-sees-each-piece-of-configuration.md`).

| File | Path | Written by |
|---|---|---|
| The person's own | `requests/<owner--name>.toml` at the repository's place | Bridge, and an agent's saved request |
| The repository's | `.armada/requests.toml`, committed | Nothing in Armada: a save never writes it |

The place is the repository's own, the one its tiles are kept in: `docs/concepts/kit.md`, *Flight controls*.
A repository's place is its only choice, and there is no separate keep on this Mac.
A file that will not read is named with Fleet's why and never drawn as empty, and Save is off until it reads.

Where both files name the same collection by `id`, or the same environment by `name`, the person's is listed and marked as hiding the repository's.
An agent that saves into the repository's collection gets a copy of it as the person's own, which then hides the original.
Commit to repository and Share carry a secret's name and never its value (`docs/concepts/kit.md`, *Commit to repository* and *Share*).
Commit to repository copies the chosen collections by `id` and environments by `name` into the branch's `.armada/requests.toml`, and Share makes them one `requests` piece, added into the colleague's own file with each secret named and not set.

## Runs

> **Rule.** Every send of a saved request is kept as a run, whoever sent it, and a run outlives the Job.
> Why: a request's history answers whether it passed yesterday, and the thread that held the row is gone when the Job ends.

A run is the request as sent, with every `{{name}}` filled but a secret's, and the answer: status, time, size, content type, headers, body and any error.
A secret shows as `{{name}}` in a run, as it does everywhere else.
Fleet keeps a request's newest 200 runs, and the oldest goes when a new one comes.
A request of its own method and address keeps none, and neither does a `send_requests` batch.

| Who reads | Reads |
|---|---|
| The person | The list, filtered by who sent it and by status, and one run whole |
| A Job's agent | The same through `list_request_runs` and `get_request_run`, with no `set-cookie` and a body cut at 32 KiB |

## In a thread

Two rows write into a Job's thread, and each opens in Requests for that Job.

| Row | Written when | The agent is |
|---|---|---|
| `requested` | The person sends with the Job open | Told, in the shape a run's end is told |
| `saved` | The agent saves a request into a collection | Not told, since it holds the answer |

A `requested` row reads You or Agent, the status, the method and the address.
A `saved` row names the request and its collection, and Open in Requests opens it there.
A request saved under a name the collection already holds replaces it and keeps its id.
A send that places no Job writes no row.

## The panel

| Part | Holds |
|---|---|
| Collections | A panel of its own at the left, open on every tab |
| Request | The editor, Send, and the Response under it |
| Environments | The variables, with Add environment |
| History | The Job's own sends, shown only with a Job open |

The Environment picker sits in the bar over the tabs and applies to every send.
With no Job open the bar carries a Repository picker, set to the Cockpit's repository filter where one is set and the last repository used otherwise.
Requests opens from the palette, `⌘K`, by name, and from a Job's Requests tile (`docs/concepts/cockpit.md`, *The Job's flight controls*).

## Walks

| Walk | Shows |
|---|---|
| `requests-collections-and-environments` | A saved request opened from the thread, an environment with a secret, a send, and the palette with no Job |
| `requests-repository-picker` | The Repository picker, and a repository whose own file will not read |

Both are in `apps/desktop/src/renderer/src/mock/walks/`.

## Not built

| Follow-up | Issue |
|---|---|
| Saved requests as Flight control tiles | https://github.com/Mele-Labs/armada/issues/2632 |
| Start a Job from Requests | https://github.com/Mele-Labs/armada/issues/2633 |
