# A page served sealed: `get_artifact_page`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. One kept revision of a Job's page as one HTML answer a client with a web view opens with no network. `docs/concepts/artifact.md`, *A page served sealed*.
**Additive only**: one operation, one route and one refusal code. No event kind, no body and no field on an existing type.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand: the query and the messages both ways. Classic does not call it, and native and Pocket host the page later.

| Where | Side | Carries |
| --- | --- | --- |
| `get_artifact_page` (`GET /sessions/artifacts/page?session_id=&artifact=&revision=`, `agent_access = "Bridge only"`) | Bridge reads | `artifact` is the page's path, its `JobArtifact` id. `revision` is a kept revision's number, the newest where it is left out |
| Its answer | Fleet writes, Bridge reads | 200, the page itself as `text/html; charset=utf-8`, with `X-Content-Type-Options: nosniff` and the page's policy again as `Content-Security-Policy`. No JSON around it |
| Refusals | Fleet | 422 `fleet.artifact_not_held` for an artifact the Job does not hold, or one that is a sketch or a draft; `fleet.artifact_revision_unknown` for a revision never kept, or one whose file is gone; `fleet.session_unnamed` |

**The answer is the page, not JSON.** `get_session_file` and `get_studio_frame` answer their bytes as themselves, and a web view loads a page from an address, so `ArtifactPage { revision, html }` would only be unwrapped again.
The revision served is the one the kit's script names in every message it posts.

**An archived page still serves.** Reading changes nothing and publishes nothing.

## The sealed page

The kept revision, `r<n>.html`, changed in these places only:

| What | Becomes |
| --- | --- |
| The `<head>` | Its first child is `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; script-src 'nonce-<n>'; connect-src 'none'; form-action 'none'; base-uri 'none'">`, then a `<style>` of every design token at its shipped value and the kit's own look. A page with no `<head>` gets one |
| `<link rel="stylesheet" href="armada-kit.css">` | A `<style>` holding the kit's rules, where the link was, so the page's own style after it still wins |
| Any other `<link>`, any `<base>`, any `<meta http-equiv>` | Dropped |
| A relative `src`, `href`, `srcset`, `poster`, `action`, `formaction`, `data`, `xlink:href`, `background` or `ping` | Taken off its tag. Relative is no scheme and not a `#` fragment. An `http`, `https`, `mailto` or `data:` address stays, and the policy refuses any fetch of it but a `data:` image |
| A `nonce` the page wrote | Taken off its tag |
| The page's own `<script>`s | **Kept, with no nonce, so the policy refuses them.** This is what sealed means |
| The end of the `<body>` | Two scripts with the nonce: the record, then the kit's script |

`<n>` is 16 random bytes in base64, fresh for each answer.
The tokens are the base theme's, the dark one Bridge ships, and a mod's theme is not applied.
A page's own scripts carry nothing a page needs: the page rules say nothing runs, and the one `<script>` they use is a sketch's `<script type="application/json">` scene, which is data and is kept as written.
An `<armada-sketch>` is not drawn on a sealed page.

**The record** is `<script type="application/json" id="armada-page-record" nonce="<n>">` holding `{ artifact, revision, presses, votes }`: each `PagePress` made on this revision, and the newest `VoteCast` per vote cast on it.
A press or a vote on another revision is not in it.
Every `<` in it is written `<`, so no text in it closes its script.

**The kit's script**, `crates/fleet/src/session_host/page_kit.js`, is the one script that runs.
It draws each `<armada-action>` as a labelled button and each `<armada-vote>` as a vote, draws the record, and reaches nothing but `window.parent.postMessage`.
Its look, `page_kit.css` beside it, is the one the owner approved on 14 Nov 2026.

## The messages

Every message is an object with `armada: 1`, posted with target origin `*`: nothing in one is secret, and the page's origin is opaque.
The kit reads a message only from `window.parent`.

| Way | `kind` | Carries | The other side |
| --- | --- | --- | --- |
| Page to host | `press` | `artifact, revision, action, words?` | The host calls `press_page_action` with the Job's `session_id` and these fields |
| Page to host | `vote` | `artifact, revision, vote, result`, `result` exactly a `VoteResult` | The host calls `cast_page_vote` with the Job's `session_id` and these fields |
| Host to page | `pressed` | `press`, the `PagePress` Fleet answered | The row turns green with a 3px success edge and reads `Sent to the Job: <label>`, then `, started Job <8 characters>` or `, answered`. Its buttons stay |
| Host to page | `voted` | `cast`, the `VoteCast` Fleet answered | The vote shows the result and reads `Sent to the Job` beside Send |
| Host to page | `refused` | `action?` or `vote?`, `code`, `message`, the refusal's | The row's line, or the vote's, shows `message`, and its button can be pressed again |

A `message` act opens a words field under its row, sent with Send or ⌘↵, which is `words`.
A `start` or `answer` act posts at once.

## The event stream

No new event, and nothing is published: the page is read once and draws what the host posts back.
The unmeasured sink risk is neither better nor worse.

## Skew

The route is new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
The messages are between the page and its host, and the page comes from the Fleet the host is talking to, so they cannot skew apart from the wire.
