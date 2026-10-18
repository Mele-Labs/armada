# Asking in the window

**Part of:** [Job](../session.md).

**A question about the walk opens in the walk.** A job that wants the person's answer about something on a page it showed calls `ask_in_window`, a tool on the agent's door, with the `question`, its `options` (each a `label` and an optional `description`), the page's `url` and a CSS `selector` for the place, with an optional `text` the element shows that picks one where the selector finds several.
The owner, 11 Oct 2026: "a question about the walk should open in the walk.
In fact it should take me where the agent is asking me about.
Highlight it like an annotation and show a card with the question and my options to answer.
If the question is not related to the walk it should wait."

| | |
|---|---|
| What it may ask about | An address on the origin of a window the job showed, or of any of its takes (the window swaps to that take), whether or not its walk is still open: a closed window is opened again. Anything else is refused with a reason that names `AskUserQuestion`, which Fleet keeps whole while he walks |
| What the call does | Keeps a `window_ask` row on the thread and answers at once. **It does not wait**: the job ends its turn, and the answer arrives as a message from the person |
| What Bridge does | Brings the job's walk window forward, opening it if it is closed, moves the page to the address, scrolls the place into view, outlines it, and draws the question on a card of the window's own bar. `docs/practices/capture-window.md` |
| How it waits | `window_ask:<row>` in the Needs you panel, with its options and its `place`. Its act is a walk of the window it was asked in, so a press there opens the window with the card |
| How it is answered | An option, words of the person's own, or both, through `answer_waiting`. The job gets `Re: <question>` with the option and the words beneath it, and the row keeps the answer. Being a message, it also settles the walk it was asked in. `dismiss_waiting` drops it with nothing sent |
| While sleep is on | Recorded as a walk is and never decided for him: it is about what is on the page |

Capture and Approve keep working while the card is open. The card folds into the bar so the page takes presses again, and the window keeps showing it until it is answered or dismissed.
