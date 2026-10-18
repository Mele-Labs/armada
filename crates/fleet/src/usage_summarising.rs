//! The store's sums as the wire carries them. `docs/practices/protocol.md`, *The usage summary*.

use ipc::{DayUnits, NamedUnits, PriorUsage, RestUsage, SessionUsageSummary, TokenUnits, UsageBreakdown, UsageSummary};
use store::{DayUsage, Named, UsageSplit, UsageSummaryRead};

fn names(rows: Named) -> Vec<NamedUnits> {
    rows.into_iter().map(|(name, units)| NamedUnits { name, units }).collect()
}

fn share(part: f64, whole: f64) -> f64 {
    if whole > 0.0 {
        part / whole
    } else {
        0.0
    }
}

fn day(one: DayUsage) -> DayUnits {
    DayUnits { day: one.day, units: one.units, by_model: names(one.by_model) }
}

fn breakdown(one: UsageSplit) -> UsageBreakdown {
    UsageBreakdown {
        units: one.units,
        subagent_units: one.subagent_units,
        above_200k_units: one.above_200k_units,
        above_300k_units: one.above_300k_units,
        by_token: TokenUnits {
            input: one.input_units,
            cache_creation: one.cache_creation_units,
            cache_read: one.cache_read_units,
            output: one.output_units,
        },
        by_model: names(one.by_model),
        by_first_tool: names(one.by_first_tool),
        by_bash_use: names(one.by_bash_use),
        by_day: one.by_day.into_iter().map(day).collect(),
        hour_grid: one.hour_grid,
    }
}

fn session(id: String, one: UsageSplit) -> SessionUsageSummary {
    SessionUsageSummary {
        session_id: id,
        peak_context: one.peak_context,
        subagent_share: share(one.subagent_units, one.units),
        models: one.by_model.iter().map(|(model, _)| model.clone()).collect(),
        usage: breakdown(one),
    }
}

pub(crate) fn wire(read: UsageSummaryRead, tz_offset_minutes: i32) -> UsageSummary {
    let prior = read.prior;
    let plain = |(day, units): (String, f64)| DayUnits { day, units, by_model: Vec::new() };
    UsageSummary {
        since: read.since,
        until: read.until,
        tz_offset_minutes,
        total: breakdown(read.window),
        sessions: read.sessions.into_iter().map(|(id, one)| session(id, one)).collect(),
        rest: read.rest.map(|(sessions, one)| RestUsage { sessions, usage: breakdown(one) }),
        prior: PriorUsage {
            since: prior.since,
            until: prior.until,
            subagent_share: share(prior.subagent_units, prior.units),
            above_200k_share: share(prior.above_200k_units, prior.units),
            units: prior.units,
            last_day: plain(prior.last_day),
            day_before: plain(prior.day_before),
        },
    }
}
