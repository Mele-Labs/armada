# A page served sealed: `get_artifact_page`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. One kept revision of a Job's page as one HTML answer a client with a web view opens with no network. `docs/concepts/artifact.md`, *A page served sealed*.
**Additive only**: one operation, one route and one refusal code.
No event kind, no body and no field on an existing type.
`host_kit`, added on 19 Nov 2026, is an optional query field that is `false` where it is left out.
Bridge's half is `classic-final:packages/protocol/src/job-artifacts.ts`, written by hand: the query and the messages both ways. Classic does not call it, and native and Pocket host the page later.

| Where | Side | Carries |
| --- | --- | --- |
| `get_artifact_page` (`GET /sessions/artifacts/page?session_id=&artifact=&revision=&host_kit=`, `agent_access = "Bridge only"`) | Bridge reads | `artifact` is the page's path, its `JobArtifact` id. `revision` is a kept revision's number, the newest where it is left out. `host_kit=true` asks for the page a host draws the kit on itself, `false` where it is left out |
| Its answer | Fleet writes, Bridge reads | 200, the page itself as `text/html; charset=utf-8`, with `X-Content-Type-Options: nosniff` and the page's policy again as `Content-Security-Policy`. No JSON around it |
| Refusals | Fleet | 422 `fleet.artifact_not_held` for an artifact the Job does not hold, or one that is a sketch or a draft; `fleet.artifact_revision_unknown` for a revision never kept, or one whose file is gone; `fleet.session_unnamed` |

**The answer is the page, not JSON.** `get_session_file` and `get_studio_frame` answer their bytes as themselves, and a web view loads a page from an address, so `ArtifactPage { revision, html }` would only be unwrapped again.
The revision served is the one the kit's script names in every message it posts.

**An archived page still serves.** Reading changes nothing and publishes nothing.

## The sealed page

The kept revision, `r<n>.html`, changed in these places only:

| What | Becomes |
| --- | --- |
| The `<head>` | Its first child is `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; script-src 'nonce-<n>'; connect-src 'none'; form-action 'none'; base-uri 'none'">`, then a `<style>` of every design token at its shipped value and the kit's own look. A page with no `<head>` gets one |
| `<link rel="stylesheet" href="armada-kit.css">` | A `<style>` holding the kit's rules, where the link was, so the page's own style after it still wins |
| The first `<link rel="stylesheet">` whose `href` is a repository path, and Fleet kept a style set for the page | A `<style>` holding the kept sheets' rules, after the kit's, each kept font an `@font-face` with its bytes in a `data:` address. Only the newest revision, since the row's `kept_styles` is the newest's |
| Any other `<link>`, any `<base>`, any `<meta http-equiv>` | Dropped |
| A relative `src`, `href`, `srcset`, `poster`, `action`, `formaction`, `data`, `xlink:href`, `background` or `ping` | Taken off its tag. Relative is no scheme and not a `#` fragment. An `http`, `https`, `mailto` or `data:` address stays, and the policy refuses any fetch of it but a `data:` image |
| A `nonce` the page wrote | Taken off its tag |
| The page's own `<script>`s | **Kept, with no nonce, so the policy refuses them.** With `host_kit`, see below |
| The end of the `<body>` | Two scripts with the nonce: the record, then the kit's script. With `host_kit`, the record alone, with no nonce |

`<n>` is 16 random bytes in base64, fresh for each answer.
**The repository's stylesheets** are checked and kept when the page is kept, as a `mock`'s `styles` are: each `href` that is not `armada-kit.css`, has no scheme and does not start `//` is a path from the repository's root.
Their hash is `detail.kept_styles["page"]`, a key no sketch's `<title>#<node id>` can be.
A sheet Fleet cannot keep leaves the page kept and no set, and the Job is told in the `document_sketch` notice: `the page's stylesheets will not draw: …`.
The policy's `font-src data:` is what lets those fonts load, and every fetch stays refused.
The tokens are the base theme's, the dark one Bridge ships, and a mod's theme is not applied.
Without `host_kit` a page's own scripts never run, and a sketch's `<script type="application/json">` scene is data and is kept as written.

**With `host_kit`, the page's own inline scripts run inside the box.** The native Bridge asks for it, and every other client gets the page above.
The policy's script source is the `'sha256-<hash>'` of each page `<script>` with no `src`, `href` or `xlink:href`, its text hashed as the browser reads it, or `'none'` where there is none.
Its every other wall is the same, so the page still fetches nothing, submits nothing and goes nowhere. A script with an address is kept and refused, as is every `on*=` handler.
**No nonce is served on this path.** A page script can read a nonce off any tag and hand it to a `<script src>` of its own, which the policy would then fetch.
A hash allows only the text it was made of.
The kit's script is left out: the host draws the kit itself in a content world the page cannot reach. The record stays.
The native Bridge runs `page_kit.js` in its `armada-kit` world, with the kit's handler and its answers in that world alone.
The kit counts only his own presses (`event.isTrusted`), so a page script's `click()` or key presses nothing; a walk's press reaches it through `armadaHostPress`, which only that world holds.
Bridge also takes WebRTC out of the page's world before its scripts run, and turns DNS prefetching off, since the policy governs neither.
An `<armada-sketch>` is not drawn on a sealed page.

**The record** is `<script type="application/json" id="armada-page-record" nonce="<n>">` holding `{ artifact, revision, presses, votes }`: each `PagePress` made on this revision or an earlier one whose action is still on it, and the newest `VoteCast` per vote cast on this revision or an earlier one where that vote is still on it in the same form with the takes the cast names.
Each keeps the `revision` it was made on.
A press or a vote on a later revision is not in it, nor one whose action or vote this revision dropped or changed.
Why: on 15 Nov 2026 the owner's votes read as never cast once the Job revised other sections of the page.
Every `<` in it is written `<`, so no text in it closes its script.

**The kit's script**, `crates/fleet/src/session_host/page_kit.js`, is the one script that runs without `host_kit`.
It draws each `<armada-action>` as a labelled button and each `<armada-vote>` as a vote, draws the record, and reaches nothing but the two channels below.
Its look, `page_kit.css` beside it, is the one the owner approved on 14 Nov 2026.
The same file opens with the house style, a calm default for every page under its own CSS: `:where()` rules only, so the page's own style and `armada-kit.css` win.
`docs/concepts/artifact.md`, *A page served sealed*.

## The messages

Every message is an object with `armada: 1`.
Nothing in one is secret, and the page's origin is opaque.

| Way | Channel | Reads it |
| --- | --- | --- |
| Page to host | `window.webkit.messageHandlers.armada.postMessage(message)`, where the host has that handler | Native: a page in a WKWebView is a top-level page with no parent frame, so `window.parent.postMessage` would only reach the page itself |
| Page to host | Otherwise `window.parent.postMessage(message, "*")`, target origin `*` | Classic: the page sits in an iframe |
| Host to page | `window.armadaHost(message)`, called with the same object | Native, with `evaluateJavaScript` |
| Host to page | A `message` event on the page's window, read only where `event.source` is `window.parent` | Classic's iframe |

The page picks its way to the host once, at load, and both ways in run one handler.
A message whose `armada` is not `1` is ignored by the page and refused by `ipc`.

**The Rust types** are in `crates/ipc/src/artifact.rs`: `PageToHost` and `HostToPage`, each an enum tagged by `kind`, and `PageMessage<T>`, which writes `armada: 1` beside the message and refuses any other number when it reads one.
They are not route bodies and have no operation file: the host's call to Fleet is `press_page_action` or `cast_page_vote`.
`PageToHost` and `HostToPage` in `classic-final:packages/protocol/src/job-artifacts.ts` mirror them, and `crates/ipc/src/tests/page_messages.rs` reads that file.
`apps/pocket/src/pageKit.test.ts` runs the kit's script against a fake DOM and holds which channel it picks.

| Way | `kind` | Carries | The other side |
| --- | --- | --- | --- |
| Page to host | `press` | `artifact, revision, action, words?` | The host calls `press_page_action` with the Job's `session_id` and these fields |
| Page to host | `vote` | `artifact, revision, vote, result`, `result` exactly a `VoteResult` | The host calls `cast_page_vote` with the Job's `session_id` and these fields |
| Host to page | `pressed` | `press`, the `PagePress` Fleet answered | The row turns green with a 3px success edge and reads `Sent to the Job: <label>`, then `, started Job <8 characters>` or `, answered`. Its buttons stay |
| Host to page | `voted` | `cast`, the `VoteCast` Fleet answered | The vote shows the result and reads `Sent to the Job` beside Send, or in its own status line where the page has Send picks |
| Host to page | `refused` | `action?` or `vote?`, `code`, `message`, the refusal's | The row's line, or the vote's, shows `message`, and its button can be pressed again |

A `message` act opens a words field under its row, sent with Send or ⌘↵, which is `words`.
A `start` or `answer` act posts at once.

## The event stream

No new event, and nothing is published: the page is read once and draws what the host posts back.
The unmeasured sink risk is neither better nor worse.

## Skew

The route is new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
The messages are between the page and its host, and the page comes from the Fleet the host is talking to, so they cannot skew apart from the wire.
