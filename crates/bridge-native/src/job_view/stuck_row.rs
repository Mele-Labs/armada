//! The acts under a stuck merge queue's Needs you row: Queue again, Tell Jobs, Repair Job and Open log, as the
//! merge strip's stuck card has them. Each asks Fleet what the card asks (`merge_strip::stuck`), from the facts
//! on the merge line when pressed.

use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString};
use ipc::{NeedsYouAct, NeedsYouRow};

use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::merge_strip::stuck::{self, Ask};
use crate::titlebar::parts::{tip, Btn, Weight};

impl JobView {
    /// The row's acts, where the merge line for its root still says the queue is stuck: when it moved, none.
    pub(super) fn stuck_acts(&self, row: &NeedsYouRow, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let NeedsYouAct::QueueStuck { root } = &row.act else { return None };
        let held = self.fleet.as_ref()?.read(cx).held();
        let line = held.merge_lines.iter().flatten().find(|line| line.root == *root)?;
        let facts = line.queue_stuck.as_ref()?;
        let told = stuck::held_by_a_job(facts, &line.pulls);
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let (item, root) = (row.id.clone(), root.clone());
        let again = {
            let (item, root) = (item.clone(), root.clone());
            labelled(
                Btn::new(SharedString::from(format!("stuck-queue-again:{item}")), "Queue again", Weight::Secondary)
                    .filled(hue)
                    .draw(look, cx)
                    .tooltip(tip("Puts each pull request back in the merge queue now"))
                    .on_click(cx.listener(move |this, _, _, cx| this.stuck_queue_again(&item, &root, cx))),
                Role::Button,
                "Queue again",
            )
        };
        let tell = {
            let (item, root) = (item.clone(), root.clone());
            labelled(
                Btn::new(SharedString::from(format!("stuck-tell-jobs:{item}")), "Tell Jobs", Weight::Secondary)
                    .disabled(!told)
                    .draw(look, cx)
                    .tooltip(tip(if told { "Sends each Job that holds one of these the cause, since when and the log" } else { "No Job holds these pull requests" }))
                    .when(told, |button| button.on_click(cx.listener(move |this, _, _, cx| this.stuck_tell_jobs(&item, &root, cx)))),
                Role::Button,
                "Tell Jobs",
            )
        };
        let repair = {
            let (item, root) = (item.clone(), root.clone());
            labelled(
                Btn::new(SharedString::from(format!("stuck-repair-job:{item}")), "Repair Job", Weight::Secondary)
                    .draw(look, cx)
                    .tooltip(tip("Starts a Job with the cause, the pull requests and the log written in"))
                    .on_click(cx.listener(move |this, _, _, cx| this.stuck_repair_job(&root, cx))),
                Role::Button,
                "Repair Job",
            )
        };
        let log = facts.log_url.clone().map(|url| {
            labelled(
                Btn::new(SharedString::from(format!("stuck-open-log:{item}")), "Open log", Weight::Secondary)
                    .draw(look, cx)
                    .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                Role::Button,
                "Open log",
            )
        });
        Some(
            div()
                .id(SharedString::from(format!("stuck-acts:{item}")))
                .flex()
                .flex_wrap()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(again)
                .child(tell)
                .child(repair)
                .children(log)
                .into_any_element(),
        )
    }

    /// Queue again: each stuck pull request goes back in the merge queue, as the card's press asks.
    fn stuck_queue_again(&mut self, item: &str, root: &str, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let held = fleet.read(cx).held();
        let Some(numbers) = held.merge_lines.iter().flatten().find(|line| line.root == root).and_then(|line| line.queue_stuck.as_ref()).map(|facts| facts.pulls.clone()) else { return };
        let manifest = stuck::manifest_of(held.repositories.as_deref().unwrap_or_default(), root);
        let asks = numbers.into_iter().map(|number| stuck::queue_ask(manifest.as_deref(), root, number)).collect();
        self.stuck_ask(item, asks, stuck::NOT_QUEUED, cx);
    }

    /// Tell Jobs: each Job that holds a stuck pull request is sent the facts, as the card's press sends them.
    fn stuck_tell_jobs(&mut self, item: &str, root: &str, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let held = fleet.read(cx).held();
        let Some(line) = held.merge_lines.iter().flatten().find(|line| line.root == root) else { return };
        let Some(facts) = line.queue_stuck.as_ref() else { return };
        let asks = stuck::tells_of(facts, &line.pulls).into_iter().map(|(session, message)| stuck::tell_ask(root, &session, message)).collect();
        self.stuck_ask(item, asks, stuck::NOT_SENT, cx);
    }

    /// Repair Job: the dispatch sheet, drafted from the facts and naming the repository.
    fn stuck_repair_job(&mut self, root: &str, cx: &mut Context<Self>) {
        let Some(fleet) = self.fleet.clone() else { return };
        let held = fleet.read(cx).held();
        let Some(brief) = held.merge_lines.iter().flatten().find(|line| line.root == root).and_then(|line| line.queue_stuck.as_ref()).map(stuck::brief_of) else { return };
        let repository = held.repositories.as_deref().unwrap_or_default().iter().find(|one| one.root == root).map(stuck::name_of);
        crate::dispatch::screen::open_drafted(brief, repository, cx);
    }

    /// Ask Fleet off the main thread. A refusal stands under the row in Fleet's words and raises the failure toast,
    /// as the card's does; pressing again clears it.
    fn stuck_ask(&mut self, item: &str, asks: Vec<Result<Ask, (String, String)>>, headline: &'static str, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let key = format!("item:{item}");
        let port = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port());
        if let Some(dock) = self.dock_mut() {
            dock.said.remove(&key);
        }
        cx.notify();
        for ask in asks {
            let refused = {
                let (job, key) = (job.clone(), key.clone());
                move |this: &mut JobView, why: SharedString, cx: &mut Context<JobView>| {
                    if let Some(page) = this.pages.get_mut(&job) {
                        page.dock.said.insert(key, why);
                    }
                    cx.notify();
                }
            };
            let ask = match ask {
                Ok(ask) => ask,
                Err((_, why)) => {
                    refused(self, why.into(), cx);
                    continue;
                }
            };
            let Some(port) = port else {
                refused(self, "Fleet is not connected.".into(), cx);
                continue;
            };
            cx.spawn(async move |this, cx| {
                let Ask { path, body, .. } = ask;
                let answer = cx.background_executor().spawn(async move { stuck::post_blocking(port, &path, body) }).await;
                if let Err(why) = answer {
                    let _ = this.update(cx, |this, cx| {
                        let told = why.clone();
                        cx.defer(move |cx| told.tell_refusal(headline, cx));
                        refused(this, why.to_string().into(), cx);
                    });
                }
            })
            .detach();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use gpui::{AppContext as _, Bounds, Pixels, Role};
    use ipc::{MergeLineView, NeedsYou};

    use super::super::parts_fold::{drawn, redraw};
    use super::super::JobView;
    use crate::fleet::FleetConnection;

    const JOB: &str = "mock-01";
    const ROOT: &str = "/Users/user/Development/armada";

    /// A stand-in Fleet that answers every request 200 and sends each one's method, path and body down `seen`.
    fn stand_in_fleet(seen: Sender<(String, String, String)>) -> u16 {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let port = listener.local_addr().expect("its address").port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let seen = seen.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().expect("the stream"));
                    let mut first = String::new();
                    if reader.read_line(&mut first).is_err() {
                        return;
                    }
                    let mut length = 0usize;
                    let mut line = String::new();
                    while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                            length = value.trim().parse().unwrap_or(0);
                        }
                        line.clear();
                    }
                    let mut sent = vec![0u8; length];
                    let _ = reader.read_exact(&mut sent);
                    let mut words = first.split_whitespace();
                    let (method, path) = (words.next().unwrap_or_default().to_string(), words.next().unwrap_or_default().to_string());
                    // What the open Job reads on the way, answered in shape so it does not read again at once.
                    let answer = match path.as_str() {
                        path if path.starts_with("/watches") => r#"{"session_id":"mock-01","watches":[]}"#,
                        "/settings" => r#"{"path":"/tmp/settings.json","refused":null,"settings":[]}"#,
                        _ => "{}",
                    };
                    let _ = seen.send((method, path, String::from_utf8_lossy(&sent).into()));
                    let mut stream = stream;
                    let _ = write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{answer}", answer.len());
                });
            }
        });
        port
    }

    /// Fleet's Needs you for the Job: the one row it raises for a stuck queue on `ROOT`.
    fn needs_with_stuck_row() -> NeedsYou {
        let wire = format!(
            r#"{{"session_id":"{JOB}","sections":[{{"kind":"questions","label":"Questions","rows":[{{"id":"queue-stuck:{ROOT}","source":"fleet_call","text":"Merge queue stuck","since":"2026-11-27T10:00:00Z","glyph":{{"kind":"waiting","said":"Merge queue stuck"}},"act":{{"kind":"queue_stuck","root":"{ROOT}"}},"dismiss":"plain"}}]}}],"fold":true,"review_in_band":false,"glyphs":[]}}"#
        );
        ipc::decode("NeedsYou", wire.as_bytes()).expect("the view decodes")
    }

    /// The Job on screen over a Fleet that holds `line` and the stuck row, with its Needs you open.
    fn opened<'a>(cx: &'a mut gpui::TestAppContext, line: MergeLineView) -> (gpui::Entity<FleetConnection>, &'a mut gpui::VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let jobs = crate::mock::jobs("ways-buttons").expect("the fixture decodes");
        let threads = crate::mock::threads("ways-buttons").expect("the threads decode");
        let waiting = jobs.iter().find(|job| job.id.as_str() == JOB).expect("the Job").waiting_for.clone();
        let fleet = cx.new(FleetConnection::idle);
        fleet.update(cx, |fleet, _| {
            fleet.hold_threads(threads.values().cloned());
            fleet.hold_merge_lines(vec![line], crate::mock::repositories());
            fleet.hold_needs(JOB, needs_with_stuck_row(), waiting);
        });
        let list = cx.new(|cx| crate::jobs::JobsList::mock_over(&jobs, fleet.clone(), cx));
        let (view, cx) = cx.add_window_view({
            let list = list.clone();
            move |window, cx| JobView::new(list, Some(threads), window, cx)
        });
        cx.simulate_resize(gpui::size(gpui::px(1400.), gpui::px(1200.)));
        list.update(cx, |list, cx| list.open(JOB.into(), cx));
        redraw(cx);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        (fleet, cx)
    }

    fn bounds_of(cx: &mut gpui::VisualTestContext, name: &str) -> Bounds<Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == Role::Button && one.name == name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    fn press(cx: &mut gpui::VisualTestContext, name: &str) {
        let at = bounds_of(cx, name).center();
        cx.simulate_click(at, gpui::Modifiers::none());
        redraw(cx);
    }

    /// Every POST the stand-in Fleet was sent, once the presses' background work has run.
    fn posted(seen: &Receiver<(String, String, String)>, cx: &mut gpui::VisualTestContext) -> Vec<(String, String)> {
        for _ in 0..50 {
            cx.run_until_parked();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        seen.try_iter().filter(|(method, ..)| method == "POST").map(|(_, path, body)| (path, body)).collect()
    }

    #[gpui::test]
    fn a_stuck_row_draws_main_cards_acts_under_its_words_and_a_moved_queue_draws_none(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = opened(cx, crate::merge_strip::stuck_view());
        for name in ["Queue again", "Tell Jobs", "Repair Job", "Open log"] {
            assert!(drawn(cx, Role::Button, name), "{name} is drawn");
        }
        let (again, tell, repair) = (bounds_of(cx, "Queue again"), bounds_of(cx, "Tell Jobs"), bounds_of(cx, "Repair Job"));
        assert!(again.right() <= tell.left() && tell.right() <= repair.left() && (again.top() - tell.top()).abs() < gpui::px(1.), "side by side in one line: {again:?} {tell:?} {repair:?}");
        let mut moved = crate::merge_strip::stuck_view();
        moved.queue_stuck = None;
        fleet.update(cx, |fleet, cx| {
            fleet.hold_merge_lines(vec![moved], crate::mock::repositories());
            cx.notify();
        });
        redraw(cx);
        cx.update(|_, cx| cx.set_global(crate::labels::LabelBook::default()));
        redraw(cx);
        assert!(!drawn(cx, Role::Button, "Queue again"), "the queue moved: no buttons");
    }

    #[gpui::test]
    fn queue_again_puts_every_stuck_pull_request_back_as_main_cards_does(cx: &mut gpui::TestAppContext) {
        let (fleet, cx) = opened(cx, crate::merge_strip::stuck_view());
        let (sent, seen) = channel();
        let port = stand_in_fleet(sent);
        fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
        let _ = posted(&seen, cx);
        press(cx, "Queue again");
        let mut paths: Vec<String> = posted(&seen, cx).into_iter().map(|(path, body)| (path, assert!(body.is_empty(), "no body")).0).collect();
        paths.sort();
        let mut want: Vec<String> = [3395, 3392, 3390, 3376].iter().map(|number| format!("/pull_requests/armada/{number}/auto_merge")).collect();
        want.sort();
        assert_eq!(paths, want);
    }

    #[gpui::test]
    fn tell_jobs_sends_each_holding_job_the_facts_main_card_sends(cx: &mut gpui::TestAppContext) {
        let line = crate::merge_strip::stuck_view();
        let want = crate::merge_strip::stuck::tells_of(line.queue_stuck.as_ref().expect("stuck"), &line.pulls);
        assert_eq!(want.len(), 2, "two Jobs hold stuck pull requests");
        let (fleet, cx) = opened(cx, line);
        let (sent, seen) = channel();
        let port = stand_in_fleet(sent);
        fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
        let _ = posted(&seen, cx);
        press(cx, "Tell Jobs");
        let mut got: Vec<(String, String)> = posted(&seen, cx)
            .into_iter()
            .map(|(path, body)| {
                assert_eq!(path, "/sessions/message");
                let body: ipc::SendSessionMessageBody = ipc::decode("SendSessionMessageBody", body.as_bytes()).expect("the body decodes");
                (body.message.session_id.as_str().to_string(), body.message.text)
            })
            .collect();
        // Each is its own request, so they may arrive in either order.
        got.sort();
        let mut want: Vec<(String, String)> = want.into_iter().map(|(session, text)| (session.to_string(), text)).collect();
        want.sort();
        assert_eq!(got, want);
    }

    #[gpui::test]
    fn tell_jobs_is_off_where_no_job_holds_a_stuck_pull_request(cx: &mut gpui::TestAppContext) {
        let mut line = crate::merge_strip::stuck_view();
        for pull in &mut line.pulls {
            pull.held_by = None;
        }
        let (fleet, cx) = opened(cx, line);
        let (sent, seen) = channel();
        let port = stand_in_fleet(sent);
        fleet.update(cx, |fleet, cx| fleet.connect_at(port, cx));
        let _ = posted(&seen, cx);
        press(cx, "Tell Jobs");
        assert!(posted(&seen, cx).is_empty(), "nothing is sent");
    }
}
