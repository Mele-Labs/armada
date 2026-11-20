//! The tabs a workflow step lists are the column's own: Fleet's `core_model::STEP_TABS` holds every id of `side::Zone`,
//! and `now` and `walks` beside them, which the zone list lacks. A zone added here without an id there is a tab
//! a workflow file could not name.

#[cfg(test)]
mod tests {
    use super::super::side::Zone;

    #[test]
    fn every_zone_id_is_a_tab_a_workflow_step_may_list() {
        for zone in Zone::ALL {
            assert!(core_model::is_step_tab(zone.id()), "`{}` is a zone a step cannot name: add it to core_model::STEP_TABS", zone.id());
        }
    }
}
