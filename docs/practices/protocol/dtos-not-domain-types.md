# DTOs, not domain types

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`WireError` is a DTO like any other, and `docs/contracts/error-contract.md` is
what specifies it —
which fields are guaranteed, why `level` and `component` are not among them,
and why removing an error code moves the protocol ID like any wire change. The v0 lifeboat below is
deliberately outside that contract.

`ipc` speaks its own vocabulary. It does not re-export a `core_model` type and
put it on the wire. The conversion is explicit, one direction, and lives at
the Fleet boundary:

```rust
// crates/ipc/src/commanding.rs — the DTO. Only what Bridge is allowed to see.
pub struct AllowedCommandRow {
    pub run: String,
    pub allowed_at: Instant,
    pub by: Actor,
}

// The conversion, and the only place it happens.
impl From<&core_model::AllowedCommand> for AllowedCommandRow {
    fn from(allowed: &core_model::AllowedCommand) -> AllowedCommandRow {
        AllowedCommandRow {
            run: allowed.run.clone(),
            allowed_at: (&allowed.allowed_at).into(),
            by: allowed.by.into(),
        }
    }
}
```

The reason this conversion has to exist, rather than serializing the domain
type directly, isn't code cleanliness — it's that `From` is where someone has
to decide, field by field, what a Bridge (running on someone's laptop,
potentially screen-shared, potentially logged) is allowed to see. A domain type
will accrete fields as Fleet's needs grow: filesystem paths, adapter tokens,
internal retry state. If that type is `#[derive(Serialize)]` and put straight on
the wire, every new field is redacted or not by accident — whichever `serde`
does by default. A `From` impl forces a human to look at the new field and write
a line of code, one way or the other. A domain type on the wire is a redaction
decision nobody made.

This cuts the other way too: `ipc` types have no business back in
`core-model`. If Fleet-side code needs an `AllowedCommandRow` to build a
response, that's an argument for a thin builder function, not for teaching
`core-model` about the wire's shape.
