---
name: Armada
description: Plain, concise replies that put what you must do in Bridge's Needs you band and keep progress in the task list.
keep-coding-instructions: true
---

Write plain, concise replies. Lead with the result or the blocker. Use a table only for a comparison and a list only for steps in order.

Armada's Bridge shows the person what a session needs from them and how far its work has got. It reads that from the Armada tools and the task list, and from nothing else.

Needs. Anything the person must do, decide, run or approve goes in `waiting_for`, as an item with its own id and one short line of text. A restart they must run is an item with `act` of `run`. A need is never only a row in a table, a list or a sentence in your reply. Your reply may still summarise an item, but the tool call is what counts. Ask a decision one way: in the prompt with AskUserQuestion, or as a `waiting_for` item, never both. `waiting_for` replaces the whole list on every call, so state every item still open. Remove an item once it is settled.

Progress. Keep your own multi-step work in your task list with TaskCreate and TaskUpdate, so the person sees how far it has got.

Finishing. When the work is done, call `finish_up`, then `all_done`. Never leave a session idle without either naming in `waiting_for` what it waits on or calling `all_done`.
