# A Job's reach past its slots: the folder answers

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/session.md`, *Modes*, and #2441. In `auto` a guarded read or a write outside the slots is put to the owner on the job's card, and two answers put a folder on a list the guard reads first.
The protocol ID moves with the files below, and there is no number to take: `armada need --status` showed nothing standing.

| Where | Side | Change |
| --- | --- | --- |
| `HelmCallAnswer` `always_allow`, `always_block` | Bridge sends, Fleet takes | Offered only on a card whose `reaches` is set, beside `allow_once` and `refuse` and never with `allow_and_remember`, whose rule matches a command and not a folder. Helm's dock never offers them |
| `HelmCallInFlight.reaches?` (`session.row`, the hosted record's `asked`) | Fleet writes, Bridge reads | The absolute path the guard named first: a tool's `file_path` or `path`, a shell line's first refused word with `~` and `$HOME` read as the home. Left out for any other call |
| `AnswerSessionAsk.folder?`, `AnswerHelmCall.folder?` (`answer_session_ask`) | Bridge sends, Fleet checks | The folder an "always" answer is for: `reaches` or a folder above it, compared a part at a time. Without one that holds `reaches` it is a 422 `fleet.session_answer_not_a_folder`, and the card stays |
| `SessionAskState` `always_allowed`, `always_blocked` (`session.row`) | Fleet writes, Bridge reads | How a card settled where the folder went on the list |
| `HelmCallAnswer` `allow_for_session` | Bridge sends, Fleet takes | Since 16 Oct 2026, offered beside the two "always" answers and carrying `folder` the same way. Fleet keeps the folder on the hosted job's in-memory state (`session_host::State::allowed_folders`), read by the guard after the owner's list, and clears it when the job is closed. Nothing is written to `features.reachFolders` |
| `HelmCallInFlight.reaches_to?` (`ReachesTo`: `read`, `write`) | Fleet writes, Bridge reads | Since 16 Oct 2026. Whether a reach card's call reads the path or writes to it: `Read`, `Grep`, `Glob`, `LS` and a shell segment of reading programs with no redirect are `read`, everything else `write`. Set where `reaches` is, and where the path could not be read as one. Left out for any other call |
| `SessionAskState` `allowed_for_session` (`session.row`) | Fleet writes, Bridge reads | How a card settled on `allow_for_session` |
| `HelmCallInFlight.kind?` (`CallKind`: `program`, `script?`) | Fleet writes, Bridge reads | Since 26 Oct 2026. The program and, for an interpreter, the script of the Bash segment whose reach the card is about: `node` and `drive.mjs`. Set only on a Bash reach card, and left out for any other call |
| `HelmCallAnswer` `allow_like_this_for_session` | Bridge sends, Fleet takes | Since 26 Oct 2026. Runs the call, and lets the job's calls of the kind the answer names run with no card until it is closed, narrowed to `folder` where one is given. Held in Fleet's memory only; a folder the owner blocked stays blocked, and a removal or a push is still read as one. Settles as `allowed_for_session`. Offered after `allow_for_session` on a Bash reach card that names `kind` and where the folder answers are offered. Fleet keeps the kind and folder on the job's in-memory state (`session_host::State::allowed_kinds`), read by the guard after the owner's list, and clears it when the job is closed |
| `AnswerSessionAsk.kind?`, `AnswerHelmCall.kind?` (`answer_session_ask`) | Bridge sends, Fleet reads | Since 26 Oct 2026. The width chosen for `allow_like_this_for_session`: the card's own `kind`, or the same with no `script`. Carried into `helm::Said::kind`. Any other kind, or none, is a 422 `fleet.session_answer_not_its_kind`, and a `folder` not holding `reaches` is a 422 `fleet.session_answer_not_a_folder`; the card stays |
| `ipc::reach_folders::ReachFolders` (`features.reachFolders` in `get_settings`, `save_settings`) | Both | `{allow?, block?}`, absolute paths, a `Json` setting in Features. Fleet writes it on an "always" answer; Settings removes an entry by saving the list without it, or `null` when both are empty. Read leniently: an entry that is not an absolute path is dropped |
| `ReachFolders`, `reachFoldersOf`, `REACH_FOLDERS_KEY`, the fields above | Bridge reads | `packages/protocol/src/reach-folders.ts`, `helm-calls.ts` and `hosted-sessions.ts`, written by hand like the rest |

**Additive.** Three variants on each of two enums, one new enum (`ReachesTo`), four optional fields left out when empty, and one setting.
No operation, event kind or route is new.
A Bridge before this draws a reaching card with Allow once and Refuse, since `helmOfferedOf` drops an offer it has no words for, and reads the settled states as words it has none for.
A Bridge before `reaches_to` draws the card with no read or write word.
Since 26 Oct 2026, one more variant, one new type (`CallKind`) and three more optional fields left out when empty; a Bridge before them drops the offer it has no words for.
The IDs differ, so the two are refused regardless.
No migration: the list is a settings.json key.
**The event stream is untouched** beyond the `session.row` and `settings.changed` an answer already publishes, so the back-pressure risk above is neither better nor worse.
The lifeboat is untouched.
