# Protocol 17.3: one press allows every read Armada's door offers

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1518`. One variant added to `HelmCallAnswer` (`allow_every_read`) and one to `HelmCallSettled`
(`every_read_allowed`). Additive: nothing renamed, nothing retyped, and no field an older peer
already reads changed.

**A Bridge that does not know the answer draws the card with three buttons**, which is the
behaviour it had before this. `helmOfferedOf` already drops an offer this build has no words for
— the rule `offeredOf` set and the reason it was set — so the new one needs nothing of an older
Bridge but that it keep ignoring what it does not know.

**The settled variant is the direction that does not survive.** A Bridge ahead of Fleet is refused
on a minor mismatch anyway; a Bridge *behind* a Fleet that sends `every_read_allowed` reads it as a
string it has no wording for, in the record and nowhere a person is blocked by it.

**Fleet decides what the set is, and the wire never carries it.** The rules written are derived
from `operations/`'s own `kind = "query"` rows at the moment a person presses, so a Fleet whose
inventory grew allows more than one that had not, and neither one has to tell Bridge which.
