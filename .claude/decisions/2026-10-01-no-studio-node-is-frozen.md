# Should a Finding or an Outline still be `frozen`?

**Decided 2026-10-01.**

A Sketch lost `frozen` first. Asked whether a Sketch drawn on after placing should still be *done changing*, he answered: *"I hate this frozen shit. Its overcomplicating it."* (`2026-10-01-a-sketch-is-the-pad.md`). Then he was asked about the two kinds left holding it, a Finding (`proposed → gathering → frozen`) and an Outline (`draft → frozen`).

**Chosen: "Remove it everywhere."** `frozen` meant *done changing, still promotable*. Nothing gated on it: promotion, writing up and dispatch never read it.

- **A Finding whose scout has ended holds no state.** How it ended (answered, stopped or failed) is already on the Finding. `gathering` was the other option, and it would be a lie: Bridge pulses a gathering Finding, Stop is offered on it, and a restart settles it as failed.
- **An Outline stays `draft`**, its only state.
- **Store V86** moved every frozen Finding to no state and every frozen Outline to `draft`, then narrowed the `state` column's `CHECK` so `frozen` cannot be written again.
- **Protocol 21.0, a major.** `frozen` was a state value a client reads, and the version file's table calls a removed variant major.

**Where it landed:** `studios/no-frozen`.
