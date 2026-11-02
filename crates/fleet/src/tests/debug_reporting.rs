//! `get_debug_report` over a Fleet with one hosted Job and one terminal Job live: each has its block,
//! and the hosted Job's tools and skills are the names its opening line gave.

use adapter_traits::{DroneEvent, HarnessReading};
use api::Sessions;
use ipc::{SessionFact, SessionId, SessionOrigin, SessionReport};

use super::session_host::{eventually, rig_placing};

fn opening_line() -> Vec<DroneEvent> {
    vec![DroneEvent::Started {
        session: String::new(),
        model: String::from("stand-in"),
        mcp_servers: 1,
        harness: HarnessReading {
            name: "The agent CLI".into(),
            update: "agent update".into(),
            version: "2.1.310".into(),
            mod_loaded: true,
            door_connected: true,
            task_tools: true,
            door: "connected".into(),
            tools: vec!["Bash".into(), "Read".into(), "mcp__armada-fleet__waiting_for".into()],
            skills: vec!["armada:reading-a-brief".into()],
        },
    }]
}

#[tokio::test]
async fn the_report_has_a_block_for_a_live_hosted_job_and_one_for_a_live_terminal_job() {
    let rig = rig_placing(None);
    let hosted = rig.start().await;
    rig.send(&hosted, "hello").await;
    rig.stand_in.says(0, opening_line());
    eventually(|| async { rig.fleet.store().lock().await.init_of(hosted.as_str()).unwrap().is_some() }).await;

    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("bbbb2222-0000"),
            fact: SessionFact::Started {
                cwd: rig.root.clone(),
                title: Some("Fix the flaky checkout test".into()),
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .expect("a terminal job starts");

    let text = rig.fleet.debug_report(Some(rig.root.clone())).await.expect("it answers").text;

    assert!(text.starts_with("Armada debug report\n\nInstall\n"), "{text}");
    let harness = format!("  {:<18}2.1.310\n", adapters::debug_facts::HARNESS);
    assert!(text.contains(&harness), "the version the opening line gave: {text}");
    assert!(text.contains("\n\nThis repository  "), "{text}");
    let hosted_block = format!(
        "  id        {}\n  runs      hosted\n  mod       none\n  door      connected\n  missing   nothing\n  tools     Bash, Read\n            mcp__armada-fleet__waiting_for\n  skills    armada:reading-a-brief",
        &hosted.as_str()[..8]
    );
    assert!(text.contains(&hosted_block), "{text}");
    let terminal_block = "Job  Fix the flaky checkout test\n  id        bbbb2222\n  runs      terminal\n  mod       none\n  door      not read\n  missing   mod\n  tools     not read\n  skills    not read";
    assert!(text.contains(terminal_block), "{text}");
    let last = text.lines().last().expect("a last line");
    assert!(last.starts_with("fleet protocol ") && last.contains("  taken 20"), "{last}");
}
