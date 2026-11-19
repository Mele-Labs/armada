//! The graph's numbers and words: bars from buckets, labels that stay inside their bands, the cursor, the tooltip and the states.

use chrono::{DateTime, Duration, Utc};
use ipc::{JobTimeline, TimelineBucket, TimelineContext, TimelinePause};

use super::model::{fit, long_said, share_fine, Graph, HEADROOM};

fn at(minute: i64) -> String {
    (DateTime::parse_from_rfc3339("2026-11-18T09:00:00Z").expect("a stamp").with_timezone(&Utc) + Duration::minutes(minute)).to_rfc3339()
}

/// Ten one-minute buckets from 09:00, the tallest of them the fourth; a wait on the person from 09:04 to 09:08 that found the cache cold;
/// a compaction at 09:08. Units are 2.0% of a week of 80,000.
fn timeline() -> JobTimeline {
    let main = [100., 200., 300., 400., 0., 0., 0., 0., 50., 50.];
    let sub = [0., 0., 100., 400., 0., 0., 0., 0., 0., 0.];
    JobTimeline {
        session_id: "job-1".into(),
        started: at(0),
        ended: Some(at(10)),
        bucket_seconds: 60,
        total_units: 1600.,
        week_units: 80_000.,
        buckets: (0..10).map(|n| TimelineBucket { at: at(n as i64), main_units: main[n], subagent_units: sub[n] }).collect(),
        context: [(0, 10_000), (1, 20_000), (2, 30_000), (3, 40_000), (8, 45_000), (9, 46_000)].into_iter().map(|(minute, tokens)| TimelineContext { at: at(minute), tokens }).collect(),
        compactions: vec![at(8)],
        pauses: vec![TimelinePause { from: at(4), to: at(8), waited_on: "you".into(), label: "Waited on you".into(), cold: true }],
        peak_context: 46_000,
        cold_wakes: 1,
        subagent_share: 500. / 1600.,
    }
}

#[test]
fn a_bucket_is_a_bar_as_tall_as_its_share_of_the_tallest() {
    let graph = Graph::new(&timeline());
    assert_eq!(graph.bars.len(), 10);
    assert_eq!((graph.bars[3].main, graph.bars[3].sub), (0.5, 0.5), "the tallest bucket is the whole of the height and half of it is each thread's");
    assert_eq!(graph.bars[0].main, 0.125);
    assert_eq!((graph.bars[4].main, graph.bars[4].sub), (0., 0.), "a bucket with no burn is no bar");
    assert_eq!((graph.bars[3].x, graph.bars[3].w), (0.3, 0.1), "a tenth of the window each, the fourth after three");
}

#[test]
fn bars_in_pixels_leave_the_headroom_and_stay_inside_the_plot() {
    let graph = Graph::new(&timeline());
    let rects = graph.bar_rects(500., 100.);
    let tallest = &rects[3];
    assert_eq!((tallest.x, tallest.w), (150., 49.), "50 wide, a pixel left between it and the next");
    assert!((tallest.main_h + tallest.sub_h - 100. * HEADROOM).abs() < 1e-3, "the tallest bar stops at the headroom, which is where a compaction's label stands");
    assert!(rects.iter().all(|rect| rect.x + rect.w <= 500. + 1e-3 && rect.main_h + rect.sub_h <= 100. * HEADROOM + 1e-3));
    let narrow = graph.bar_rects(20., 100.);
    assert_eq!(narrow[3].w, 2., "a bar too narrow to spare a pixel keeps it");
    assert!(graph.bar_rects(5., 100.).iter().all(|rect| rect.w >= 1.), "and none is narrower than a pixel");
}

#[test]
fn a_window_with_no_burn_has_no_height_to_scale_by() {
    let mut quiet = timeline();
    quiet.buckets.iter_mut().for_each(|bucket| (bucket.main_units, bucket.subagent_units) = (0., 0.));
    let graph = Graph::new(&quiet);
    assert!(graph.bar_rects(500., 100.).iter().all(|rect| rect.main_h == 0. && rect.sub_h == 0.));
}

#[test]
fn the_pause_is_a_band_over_its_stretch_and_a_cold_one_has_its_dot_on_the_context_line() {
    let graph = Graph::new(&timeline());
    let band = &graph.bands[0];
    assert_eq!((band.x0, band.x1), (0.4, 0.8));
    assert_eq!((band.label.as_str(), band.duration.as_str(), band.on_person, band.cold), ("Waited on you", "4 min", true, true));
    let (across, share) = band.wake.expect("a cold wake has a dot");
    assert_eq!(across, 0.8, "where the Job woke");
    assert!((share - 45_000. / 46_000.).abs() < 1e-4, "on the line, at what the context was then");
    assert_eq!(graph.marks, [0.8], "one mark, where it was compacted");
}

/// Six pixels a letter.
fn measured(text: &str) -> f32 {
    text.chars().count() as f32 * 6.
}

#[test]
fn a_label_is_cut_to_the_room_it_has_and_never_runs_over() {
    assert_eq!(fit("Waited on you", 100., &measured).as_deref(), Some("Waited on you"), "it fits whole");
    for room in 0..120 {
        for text in ["Waited on you", "52 min", "Subagent ran"] {
            if let Some(shown) = fit(text, room as f32, &measured) {
                assert!(measured(&shown) <= room as f32, "{shown:?} in {room}px");
            }
        }
    }
    assert_eq!(fit("Waited on you", 8., &measured), None, "a band too narrow for a letter and its ellipsis draws no label");
    assert!(fit("Waited on you", 50., &measured).is_some_and(|shown| shown.ends_with('…')), "a cut one says it was cut");
}

#[test]
fn the_labels_of_a_graphs_bands_fit_their_bands_at_every_width() {
    let graph = Graph::new(&timeline());
    let pad = 4.;
    for width in [60., 120., 240., 480., 960., 1920.] {
        for band in &graph.bands {
            let room = (band.x1 - band.x0) * width - pad * 2.;
            for said in [band.label.as_str(), band.duration.as_str()] {
                if let Some(shown) = fit(said, room, &measured) {
                    assert!(measured(&shown) <= room.max(0.), "{shown:?} over a band {room}px wide at {width}px");
                }
            }
        }
    }
}

#[test]
fn the_cursor_moves_a_bucket_and_is_held_at_either_end() {
    let graph = Graph::new(&timeline());
    assert_eq!(graph.step(None, 1), Some(0), "the first move right lands on the first bucket");
    assert_eq!(graph.step(None, -1), Some(9), "and the first move left on the last");
    assert_eq!(graph.step(Some(3), 1), Some(4));
    assert_eq!(graph.step(Some(3), -1), Some(2));
    assert_eq!(graph.step(Some(9), 1), Some(9), "held at the end");
    assert_eq!(graph.step(Some(0), -1), Some(0), "and at the start");
    let mut nothing = timeline();
    nothing.buckets.clear();
    assert_eq!(Graph::new(&nothing).step(None, 1), None, "no bucket, no cursor");
}

#[test]
fn a_pointer_over_the_chart_is_on_the_bucket_under_it() {
    let graph = Graph::new(&timeline());
    assert_eq!(graph.bucket_at(0.0), Some(0));
    assert_eq!(graph.bucket_at(0.35), Some(3));
    assert_eq!(graph.bucket_at(1.0), Some(9));
    assert_eq!(graph.bucket_at(2.0), Some(9), "past the edge is the edge");
}

#[test]
fn the_tooltip_names_the_time_the_share_of_the_week_who_spent_it_and_the_context() {
    let graph = Graph::new(&timeline());
    let tip = graph.tip_in(3, &Utc).expect("a bucket");
    assert_eq!(tip.time, "09:03 to 09:04");
    assert_eq!(tip.share, "1.0% of your week");
    assert_eq!(tip.split, "Main thread 0.50%, subagents 0.50%");
    assert_eq!(tip.context.as_deref(), Some("Context 40k"));
    assert_eq!(tip.pause, None);
    assert_eq!(tip.said(), "09:03 to 09:04, 1.0% of your week, Main thread 0.50%, subagents 0.50%, Context 40k");
}

#[test]
fn the_tooltip_of_a_bucket_in_a_pause_says_what_it_waited_on_and_that_the_cache_went_cold() {
    let graph = Graph::new(&timeline());
    let tip = graph.tip_in(5, &Utc).expect("a bucket");
    assert_eq!(tip.pause.as_deref(), Some("Waited on you, 4 min. The cache went cold"));
    assert!(tip.said().ends_with("Waited on you, 4 min. The cache went cold"));
    assert_eq!(graph.tip_in(20, &Utc), None, "no bucket there");
}

#[test]
fn the_header_gives_the_clock_the_length_and_the_share_of_the_week() {
    let graph = Graph::new(&timeline());
    assert_eq!(graph.header_in(&Utc), "09:00 to 09:10, 10 min, 2.0% of your week");
    let mut running = timeline();
    running.ended = None;
    assert!(Graph::new(&running).header_in(&Utc).starts_with("09:00 to now"), "a Job still running has no end yet");
}

#[test]
fn four_figures_stand_under_the_chart() {
    let figures = Graph::new(&timeline()).figures();
    assert_eq!(figures.each_ref().map(|(name, _)| *name), ["Burn an hour", "Peak context", "Cold wakes", "In subagents"]);
    assert_eq!(figures[0].1, "12%", "2.0% of the week in ten minutes");
    assert_eq!(figures[1].1, "46k");
    assert_eq!(figures[2].1, "1");
    assert_eq!(figures[3].1, "31%");
}

#[test]
fn words_for_lengths_and_for_slivers_of_a_week() {
    assert_eq!([long_said(52. * 60.), long_said(6. * 3600. + 28. * 60.), long_said(2. * 3600.)], ["52 min", "6h 28m", "2h"]);
    assert_eq!([share_fine(0.0), share_fine(0.0063), share_fine(0.041)], ["0%", "0.63%", "4.1%"]);
}

#[test]
fn the_hours_across_the_axis_are_whole_hours_inside_the_window() {
    let mut long = timeline();
    long.started = at(-12);
    long.ended = Some(at(150));
    let ticks = Graph::new(&long).ticks_in(&Utc);
    assert_eq!(ticks.iter().map(|(_, said)| said.as_str()).collect::<Vec<_>>(), ["09:00", "10:00", "11:00"]);
    assert!(ticks.iter().all(|(across, _)| (0.0..=1.0).contains(across)));
}

#[test]
fn a_job_that_spent_nothing_is_an_empty_graph_with_nothing_to_move_over() {
    let graph = Graph::new(&super::mock::empty());
    assert_eq!(graph.bars.len(), 1);
    assert!(graph.bars[0].main == 0. && graph.line.is_empty() && graph.bands.is_empty());
}

#[test]
fn the_mock_job_is_the_one_the_sketch_drew() {
    let graph = Graph::new(&super::mock::timeline());
    assert_eq!(graph.bands.iter().map(|band| (band.label.as_str(), band.duration.as_str(), band.cold)).collect::<Vec<_>>(), [("Command ran", "14 min", false), ("Waited on you", "52 min", true)]);
    assert_eq!(graph.marks.len(), 2);
    assert!(graph.header_in(&chrono::Local).starts_with("09:12 to 15:40, 6h 28m, 4.1% of your week"), "{}", graph.header_in(&chrono::Local));
}
