//! What was said in the Helm Session's thread, read back as text. `#1293`.

/// What was said in the Helm Session's thread: the person's asks and the session's prose, in
/// order. `#1293`.
///
/// **Fleet's own read and never a tool.** `get_helm_session` refuses every agent, so a scout
/// reaching a thread through a tool would be that door opened; a read-in hands over this text
/// instead.
///
/// Everything else in the thread is the session's working (its calls, the asks it put to the
/// person, where it sent words), which is not what was argued.
pub fn what_was_said(rows: &[ipc::SessionRow]) -> String {
    let mut said: Vec<String> = Vec::new();
    for row in rows {
        let ipc::SessionRow::Message { from, text, .. } = row else {
            continue;
        };
        let who = match from {
            ipc::SessionVoice::You => "The person",
            ipc::SessionVoice::Agent => "Helm",
            ipc::SessionVoice::Session { title, .. } => title.as_str(),
        };
        if !text.trim().is_empty() {
            said.push(format!("{who}: {}", text.trim()));
        }
    }
    said.join("\n\n")
}
