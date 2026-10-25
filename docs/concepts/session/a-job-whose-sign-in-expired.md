# A Job whose sign-in expired

**Part of:** [Job](../session.md).

**A Job whose agent CLI has lost its sign-in fails its turn at once, and the harness writes one line marked as an authentication failure.** Fleet reads the marker and never matches on the text.

**Every Job shares the account, so the Fleet signs out as one.**
The first Job heard signed out sets one Fleet-wide state with the time it happened, and a repeat failure in any Job keeps that time.
Fleet sends the state whole to Bridge, which shows one alert with a Sign in button and a Mac notification.
No Job carries an item of its own for it.
Each Job the failure stopped is marked, so Fleet knows which ones to wake.

**Pressing Sign in has Fleet run the CLI's own sign-in, with no terminal.**
It opens his browser, and the page calls the CLI back on a port of its own.
One runs at a time, and a second press while it waits starts nothing.
A run that fails, or that he leaves for ten minutes, leaves the Fleet signed out, and Bridge offers it again.

**A clean exit signs every Job in.** Fleet clears the state and every mark, and wakes each stopped Job with `Continue`, as the plan-limit hold wakes one (*A job that hit the plan's limit*).
**An answer anywhere counts as signed in.** The model's own words or call in any Job, while the Fleet is signed out, means the account works again, however he signed in.
Fleet clears everything the same way and wakes the other stopped Jobs.
A turn starting does not count, since the harness says `init` before a failed turn too.

**A Job in his own terminal is covered through the mod.** The mod reads the same failure from the turn and tells Fleet, which marks that Job and signs the Fleet out.
Its wake reaches the terminal the way any Fleet notice does, and its next finished turn counts as signed in.

**The mark and the state are kept in memory only.** A Fleet that restarts has neither, and the next failed turn sets them again.
