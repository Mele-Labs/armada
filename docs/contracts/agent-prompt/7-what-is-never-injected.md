# 7. What is never injected

**Part of:** [Agent Prompt Contract](../agent-prompt.md).

| Never reaches | What | Enforced by |
| --- | --- | --- |
| Any invocation | Secrets, **at assembly**. `Secret<T>` has no `Debug`, `Display` or `Serialize`, so a prompt Armada builds containing one does not compile | Type system — but only for text Armada assembles. A credential sitting in a repository file is a plain String in the Drone's context and the type system is not involved. The three-sink `Redactor` and worktree confinement cover what it cannot reach. **Neither is a guarantee that a Drone's context is secret-free** |
| The Judge | The Drone's transcript or self-report — constitutional rule 2 | `verification` assembles the prompt; the Drone never touches it |
| A Judge in a panel | The other judges' verdicts — rule 5. Independence is what makes unanimity mean anything | Dispatch |
| A Drone | Another Job's context, evidence or transcript | One worktree per **Job**, which every Drone of that Job and no other shares; Fleet assembles per step, from that Job's record |
| Helm | Anything outside the selected Manifest | Fleet MCP scoping |
| A Drone | The operator's own MCP servers. v1 spawned without `--strict-mcp-config` and got 103 tools | `DroneSpawnConfig`, non-optional |
