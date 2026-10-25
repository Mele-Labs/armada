# A Job whose sign-in expired

**Part of:** [Job](../session.md).

**A hosted Job whose agent CLI has lost its sign-in fails its turn at once, and the harness writes one line marked as an authentication failure.** Fleet reads the marker and never matches on the text.
It raises one Needs you item on the Job, `Sign in to Claude`, whose act runs the CLI's sign-in command where the Job works. Bridge's Run opens it in the Job's run pane.

**The mark is kept in memory only.** A Fleet that restarts raises the item again on the Job's next failed turn. A turn that starts clears it.

**A clean exit of the sign-in run clears the item on every Job.**
The Job the run was started from is told of the run, as for any run item, and that starts its turn.
Every other Job the expiry stopped is woken with `Continue`, as the plan-limit hold wakes one (*A job that hit the plan's limit*).

**A Job in the person's own terminal is not covered.** The CLI shows its own error there, and he signs in there.
