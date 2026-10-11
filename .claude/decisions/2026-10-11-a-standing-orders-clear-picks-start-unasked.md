# Does a pick from a standing order wait for him before it runs?

**Decided 2026-10-11.**

A standing order is a repository told in plain words to work its own backlog:
*"work through the open issues that don't need design decisions, no more than
two at a time."* He sets it going at 11pm. At 11:05 it picks a flaky test and a
wrong label. Under the 3 Sep rule that he reads every plan, both would sit until
morning and nothing would run overnight.

**Chosen:** a pick the order is sure of starts on its own. A pick it is unsure
of — is this a design call? — is set aside for him to review, and the order
moves straight on to the next issue. An unsure pick never holds up the queue.

**Cost he took:** this narrows the 3 Sep rule for standing orders only. A bad
pick that the order was sure of spends up to one Job's cap before he sees it.

**What it says about him:** what he wants is work that runs without him and
brings him only what needs him. A doubt is something to set aside, not
something to stop for.

**Where it landed:** not built.
