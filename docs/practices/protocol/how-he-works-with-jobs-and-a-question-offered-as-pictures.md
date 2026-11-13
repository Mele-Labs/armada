# How he works with Jobs, and a question offered as pictures

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: no operation, no event.
Every field below is absent by default, so a peer that never learned one reads the default and writes nothing for it.

## Preferences

`get_preferences` answers, and `save_preferences` saves one at a time, the twelve below.
Each is a key of settings.json under `bridge.`, in `crates/config/src/settings/bridge.rs`.

| Preference | Settings key | Kind | Default, and left out while it is | Saved in |
| --- | --- | --- | --- | --- |
| `take_in` | `bridge.takeIn` | `read`, `look`, `listen` | `read` | `text` |
| `answer_with` | `bridge.answerWith` | `type`, `talk`, `buttons` | `type` | `text` |
| `first_run_answered` | `bridge.firstRunAnswered` | switch | `false` | `value` |
| `read_results` | `bridge.readResults` | `note_then_page`, `always_page`, `thread_only` | `note_then_page` | `text` |
| `look_results` | `bridge.lookResults` | `picture_every`, `picture_screens`, `walk_visible` | `picture_every` | `text` |
| `look_questions` | `bridge.lookQuestions` | `pictures_mix`, `pictures_whole` | `pictures_mix` | `text` |
| `listen_end_summary` | `bridge.listenEndSummary` | switch | `true` | `value` |
| `listen_needs_you` | `bridge.listenNeedsYou` | switch | `true` | `value` |
| `listen_progress` | `bridge.listenProgress` | switch | `false` | `value` |
| `listen_operator` | `bridge.listenOperator` | switch | `false` | `value` |
| `listen_voice` | `bridge.listenVoice` | text, the Mac's voice name | empty, the Mac's default voice | `text` |
| `buttons_words` | `bridge.buttonsWords` | switch | `true` | `value` |

**The closed sets are enums on the wire in `ipc` and Choice entries in the registry.** The two spell the same words, and `the_registry_and_the_wire_spell_each_closed_set_the_same` in `fleet` holds them to each other.
A word outside a set is a 422 `fleet.unacceptable_choice` naming the preference and the word, with nothing stored.

**A switch that defaults on is saved off by writing `false`.** The shipped `true` is the registry's answer for a key the file lacks, so `false` in the file wins and sticks.
On the wire the field is left out while `true` and sent while `false`, and an absent field reads `true`.
This is the inverse of `draft_pull_requests`, which is left out while `false`.

## A question's option as a picture

| Where | Carries | Notes |
| --- | --- | --- |
| `waiting_for` item `options[]`, and `SessionRecord.waiting_for[].options[]` | `picture?`: `{ image, width, height, parts? }` | `image` is a URL or a path Fleet serves. `width` and `height` are the still's pixels. Absent on an option of words |
| `picture.parts[]` | `{ name, label, x, y, w, h }` | A box on the still, in the still's pixels from its top left. `name` is unique within the picture and is what the agent is told. `label` is what the person reads |
| `answer_waiting_parts`, a new operation: `POST /sessions/waiting/answer_parts` | `AnswerWaitingParts { session_id, item_id, parts_picked: [{ option, part }] }` | `parts_picked` is not empty. `option` is an index into the item's `options`, as `choice` is. `part` is a part's `name`. `AnswerWaiting` is unchanged |

**Fleet carries and stores; it does not draw or judge a picture.** `session_waiting` has a `pictures` column, on the open item and on the closed one, holding the JSON of an array as long as `options`, each entry null or one picture.
A reopen puts them back.

**A parts answer is its own operation, so `AnswerWaiting` keeps its body.**
Fleet settles it as it settles a whole option.
An `agent` item becomes one message to the Job: a line asking for a mix, then one line per part, the option's label and the part's name.
The thread row reads `Answered: a mix of parts`, and the item leaves the waiting list.
An empty `parts_picked`, an `option` the item lacks, a `part` that option's picture lacks, or an item with no pictures is a 422 `fleet.session_waiting_empty`, and nothing is sent.
An item nothing holds is a 409.
Words and a mode are not taken here; a person who adds words answers with `answer_waiting`.

**The agent's own tool description does not offer `picture` yet.** `waiting_for` takes it, and an agent is told of it when Bridge draws one.

## What Fleet does with them

No new operation or field: Fleet reads the five closed preferences and says them to the Job, and checks two of them.

| Where | What | Notes |
| --- | --- | --- |
| A Job's start note, and `get_logbook_for_subagent`'s `text` | One note: a header, the line for `take_in` and its `read_results` or `look_results`, `look_questions` where he looks, and the line for `answer_with`. At every default, the header and the read line | Read at each process start. A helper's brief ends with it, so `text` is never empty |
| A saved change to the note, `save_preferences` | The whole new note, once, as a `taught` notice to each Job with a running process | A save that leaves the note as it was tells none |
| `waiting_for`, `answer_with` = `buttons` | A question with no options or more than four is 422 `fleet.session_waiting_options`, naming the item | A run, an approval, a procedure, a pull request and a message are not questions |
| A reply that asks in words, `answer_with` = `buttons` | One `reply_claim` notice telling the Job to ask it again with options | Not when the Job asked through a tool, and not for the reply to the notice itself |
| `all_done`, `take_in` = `look` and `look_results` not `picture_screens` | 409 `fleet.picture_owed` until a sketch, a window or an image through `show_file` has been shown | Listed with the other reasons `all_done` is refused |
| A workflow's groups, handed to the Job | A step that ends on `plan_approved`, `you_approved`, `sketch_approved` or `document_approved`, or records the plan, ends with a block by `take_in`, and the buttons line where he answers by buttons | Added where the groups are built, so his own copy of a workflow gets it. None at every default, and no page-first block where `read_results` is `thread_only` |

## Event stream

Neither change adds an event.
A saved preference publishes the `settings.changed` that a save already does, and an answered item the `SessionChanged` it already does.
The unbounded sink is neither better nor worse.

Bridge's half is in `packages/protocol/src/preferences.ts` and `sessions.ts`, written by hand like the rest.
