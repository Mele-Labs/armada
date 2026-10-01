# Where does a failure show, when it comes from inside a panel?

**Decided 2026-10-01.**

A press inside a panel that failed (Save on Edit this task, Pilot, Restart, a drop whose route Fleet doesn't serve yet) raised its notice at the top of the screen, under the panel's dim. Copy debug info, the act that turns a failure into a brief for an agent, couldn't be pressed without closing the panel. Offered a notice inside the panel, or the banner drawn above the dim, the owner chose neither: *"I would prefer that we have a toast/notification system that these types of things pop up in. They should overlay everything on the screen and I can interact with them."*

**Chosen:** a failure pops up as a toast over everything on the screen, panels and their dim included. Its acts (Copy debug info, File an issue, Dismiss) are pressable where it appears.

**Then, as a second thing:** *"Maybe we have a notifications area as well?"* A place that keeps what has popped up, so a dismissed one can be found again. This is not decided, and nothing waits on it.

**Then, the same day:** the form's guidance — "Fleet is not connected. Nothing was sent." — still drew in the banner band, under a panel's dim. The owner chose to toast it too: a press that didn't happen pops up over everything, and Fleet being down keeps its own banner.

**Where it landed:** `plan/group-glyphs`.
