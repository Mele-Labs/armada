# What does Bridge do with a control whose Fleet route isn't built yet?

**Decided 2026-09-29.**

Pulse was to get a kill button on each process row, and Fleet has no route that kills one process. That route was filed as #1647. The owner: *"What I would love to happen is somehow for the app to know that issue exists … when I click on that button and the fleet route doesn't exist, it instead gives me an error that says 'Not Implemented' and the details of the error include the ticket where it will be implemented at. This way I can copy the debug info for that error and paste it to an agent or job and it can immediately start working on the ticket."*

**Chosen:** Bridge can ship a control before its Fleet route exists. The act names the issue that will build the route. Pressing it while Fleet doesn't serve the route raises a `Not implemented` error whose copyable debug info carries the issue's link, so the paste is a complete brief for an agent.

**Cost he took:** a control on screen that doesn't work yet, and a registry of unbuilt routes to keep in step with the issues.

**Then, asked how the ticket reads when the gate refuses the word `github` outside the adapters:** the full URL, with the registry file exempted from the vendor rule by name (`xtask/src/rules.rs`). He chose that over `NickMele/armada#1647`, a short form no one can click, and over a tracker URL served from a setting. The cost is the first named hole in that rule outside the adapters.

**Where it landed:** the Pulse rebuild on `pulse/from-its-board`, with the error contract and #1647.
