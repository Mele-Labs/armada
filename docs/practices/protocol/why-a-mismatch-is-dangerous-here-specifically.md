# Why a mismatch is dangerous here specifically

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Skew is usually a deploy-time annoyance: you restart the old thing, it's fine.
That's not what happens here, because **Fleet outlives Bridge by design.** Fleet
is a daemon; Bridge is a window someone closes to go to lunch. A Job runs
unattended, with agents spending real tokens against real API budgets, for
however long it takes — hours, sometimes. Fleet gets upgraded during that window
because that's when upgrades happen: nobody's watching.

So the window isn't "between deploys," it's "for the entire duration of
whatever Job happens to be running when someone updates one side." A mismatch
met mid-Session does not get a graceful restart: the connection that was streaming
its events goes bad while an agent is mid-tool-call, burning budget, with
nobody able to see what it is doing.

**Strict equality makes that more frequent, and it is accepted.** Before IDs, an
additive change was a banner and nothing else, and only a breaking one refused.
Now any change to the wire files refuses a Bridge relaunched after it until
Fleet is restarted, including a change that would have been additive. `scripts/restart --adopt` carries
working Jobs across the restart, so the cost is a restart rather than a lost
Job. What is gained is that no change is judged
"additive enough" by the person making it, which was the judgement a banner was
trusting.

**And the same lifetimes make one direction the likely one.** A running Fleet's
ID does not change when someone updates the app: the daemon started last week is
still speaking last week's protocol, and the Bridge relaunched after the update
is the newer of the two. The other direction is rarer, reached by restarting
Fleet without relaunching Bridge.
