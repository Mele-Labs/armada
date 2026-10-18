# Protocol 18.1: a server says which checkout answers it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1577` and `#1564`. One field added to `ServerState` — `checkout`, carrying the path the server
runs in, the branch where there is one, the commit it came up on, and how many commits have landed
in that checkout since. Additive: nothing renamed, nothing retyped, no field an older peer reads
changed.

**A new required field rather than an optional one, and it is still a minor.** What minor promises
is that nothing an older peer already reads changes; a field it does not know is one it ignores.
`ServerState` travels Fleet to Bridge only, so there is no direction in which an older Fleet is
asked to produce it.

**The three `server.*` kinds did not grow a fourth.** Each already carries the whole `ServerState`
and a reader replaces a row rather than patching it, so a row that goes behind is published again
under the kind its phase is. A fourth kind would have been a second way to say what one already
says, and every reader of the stream would have had to learn it.
