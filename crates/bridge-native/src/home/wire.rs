//! The home's two routes, each a request off the main thread on the port the Fleet connection read.

use gpui::{App, Task};
use ipc::{BoardLayout, HomeLayout, SaveHomeLayout};

use crate::settings::wire::{body, request, Ask, Refused};

/// `GET /home/layout`: the layout he kept, none where he kept none.
pub fn read(port: u16, cx: &App) -> Task<Result<HomeLayout, Refused>> {
    request(port, Ask::Get("/home/layout".to_string()), "HomeLayout", cx)
}

/// `POST /home/layout`: keep the layout, whole, or forget it with none.
pub fn save(port: u16, layout: Option<BoardLayout>, cx: &App) -> Task<Result<HomeLayout, Refused>> {
    request(port, Ask::Post("/home/layout".to_string(), body(&SaveHomeLayout { layout })), "HomeLayout", cx)
}
