//! The System block's routes, each a request off the main thread on the port the Fleet connection read.

use gpui::{App, Task};
use ipc::{AnswerSystemAddition, SystemFolder, SystemMap, SystemPartAnswers};

use crate::link::percent_encoded;
use crate::settings::wire::{body, request, Ask, Refused};

/// `GET /projects/project/:id/system?part=&feature=`: one level of the system map. Neither is level one, `part` is level two and
/// `feature` is level three.
pub fn system(port: u16, project: &str, part: Option<&str>, feature: Option<&str>, cx: &App) -> Task<Result<SystemMap, Refused>> {
    let mut path = format!("/projects/project/{}/system", percent_encoded(project));
    let asked: Vec<String> = part.map(|part| format!("part={}", percent_encoded(part))).into_iter().chain(feature.map(|feature| format!("feature={}", percent_encoded(feature)))).collect();
    if !asked.is_empty() {
        path = format!("{path}?{}", asked.join("&"));
    }
    request(port, Ask::Get(path), "SystemMap", cx)
}

/// `GET /projects/project/:id/system/part?part=`: what Fleet knows of one part, in the nine sections.
pub fn system_part(port: u16, project: &str, part: &str, cx: &App) -> Task<Result<SystemPartAnswers, Refused>> {
    let path = format!("/projects/project/{}/system/part?part={}", percent_encoded(project), percent_encoded(part));
    request(port, Ask::Get(path), "SystemPartAnswers", cx)
}

/// `GET /projects/project/:id/system/folder?path=`: one folder of a part's code, read when the tree opens it.
pub fn system_folder(port: u16, project: &str, path: &str, cx: &App) -> Task<Result<SystemFolder, Refused>> {
    let path = format!("/projects/project/{}/system/folder?path={}", percent_encoded(project), percent_encoded(path));
    request(port, Ask::Get(path), "SystemFolder", cx)
}

/// `POST /projects/project/:id/system/librarian`: the librarian reads the project again, on his press. Level one comes back.
pub fn system_librarian(port: u16, project: &str, cx: &App) -> Task<Result<SystemMap, Refused>> {
    let path = format!("/projects/project/{}/system/librarian", percent_encoded(project));
    request(port, Ask::Post(path, String::from("{}")), "SystemMap", cx)
}

/// `POST /projects/project/:id/system/addition`: what he chose for a part the librarian added. Level one comes back.
pub fn system_addition(port: u16, project: &str, answer: &AnswerSystemAddition, cx: &App) -> Task<Result<SystemMap, Refused>> {
    let path = format!("/projects/project/{}/system/addition", percent_encoded(project));
    request(port, Ask::Post(path, body(answer)), "SystemMap", cx)
}
