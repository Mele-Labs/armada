//! A walk or dev server of a Job on one of his Macs, opened on another. The Mac that holds the Job says which
//! loopback ports of a shared Job may be shown (`share_job_page`); the Mac that looks opens the page through
//! its Gateway (`open_other_mac_page`). The Gateway's half is `pocket::mac_pages`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{OpenOtherMacPage, OtherMacPageOpened, ServerPhase, SessionId, WireError, WireValue};

use crate::daemon::Fleet;
use crate::other_macs::MAC_UNREACHABLE;
use crate::phone::OtherMacPage;
use crate::session_host::NO_SUCH_SESSION;

const PAGE_NOT_SHOWN: &str = "fleet.other_mac_page_not_shown";
const MAC_ASLEEP: &str = "fleet.other_mac_asleep";

/// The port and the rest (path, query and fragment, `/` first) of a page on this Mac's loopback over `http`.
/// `None` for any other address.
pub(crate) fn on_loopback(url: &str) -> Option<(u16, String)> {
    let after = url.trim().strip_prefix("http://").or_else(|| url.trim().strip_prefix("HTTP://"))?;
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let (authority, rest) = after.split_at(end);
    let (host, port) = match authority.strip_prefix('[') {
        Some(bracketed) => {
            let (host, tail) = bracketed.split_once(']')?;
            (host, tail.strip_prefix(':'))
        }
        None => match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    if !["localhost", "127.0.0.1", "::1"].contains(&host.to_ascii_lowercase().as_str()) {
        return None;
    }
    let port = match port {
        Some(port) => port.parse().ok()?,
        None => 80,
    };
    let rest = match rest.starts_with('/') {
        true => rest.to_string(),
        false => format!("/{rest}"),
    };
    Some((port, rest))
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The loopback ports Job `job` shows: its windows that still stand with their takes, and the servers it
    /// started that are serving.
    async fn page_ports(&self, job: &SessionId) -> Result<Vec<u16>, Refusal> {
        let record = {
            let store = self.store().lock().await;
            let kept = store.session(job.as_str()).map_err(|why| self.ledger_fault(why))?;
            match kept {
                Some(kept) => Some(self.ledger_row(&store, &kept)?),
                None => None,
            }
        };
        let mut urls: Vec<String> = Vec::new();
        for window in record.iter().flat_map(|record| crate::session_host::walking::walk_windows(&record.attachments)) {
            urls.push(window.target.clone());
            let takes = window.detail.get("variations").and_then(|kept| ipc::decode::<Vec<ipc::WindowTake>>("a window's takes", kept.as_bytes()).ok());
            urls.extend(takes.unwrap_or_default().into_iter().map(|take| take.url));
        }
        let servers = self.servers_within(None)?.servers;
        let serving = servers.into_iter().filter(|one| one.phase == ServerPhase::Serving && one.session_id.as_ref() == Some(job));
        let mut ports: Vec<u16> = Vec::new();
        for server in serving {
            ports.extend(server.ports.iter().map(|port| port.port));
            urls.extend(server.links.into_iter().map(|link| link.url));
        }
        ports.extend(urls.iter().filter_map(|url| on_loopback(url)).map(|(port, _)| port));
        Ok(ports)
    }

    /// `share_job_page`: Job `job` is shared, and shows `port`.
    pub(crate) async fn job_page_shared(&self, job: SessionId, port: u16) -> Result<(), Refusal> {
        self.act_from_other_mac(&job).await?;
        if self.page_ports(&job).await?.contains(&port) {
            return Ok(());
        }
        Err(Refusal::NotFound(WireError::raised(
            PAGE_NOT_SHOWN,
            format!("Job {} shows no page on port {port}.", job.as_str()),
            self.run_id(),
        )))
    }

    /// `open_other_mac_page`: the page, through the link where another Mac holds the Job.
    pub(crate) async fn other_mac_page_opened(&self, open: OpenOtherMacPage) -> Result<OtherMacPageOpened, Refusal> {
        let job = open.session_id.as_str();
        let Some(mac) = self.mac_holding(job).await else {
            let here = self.store().lock().await.session(job).map_err(|why| self.ledger_fault(why))?.is_some();
            return match here {
                true => Ok(OtherMacPageOpened { url: open.url }),
                false => Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("No Job has the id {job}."), self.run_id()))),
            };
        };
        let Some((port, rest)) = on_loopback(&open.url) else {
            return Ok(OtherMacPageOpened { url: open.url });
        };
        let opened = self.phone_switch()?.open_other_mac_page(&mac, job, port, &rest).await;
        match opened.map_err(|why| Refusal::Fault(WireError::raised(MAC_UNREACHABLE, why, self.run_id())))? {
            OtherMacPage::Open(url) => Ok(OtherMacPageOpened { url }),
            OtherMacPage::Asleep { name, since } => Err(Refusal::IllegalMove(
                WireError::raised(MAC_ASLEEP, format!("{name} is asleep. Its pages open when it wakes."), self.run_id())
                    .with_field("mac", WireValue::Str(mac))
                    .with_field("since", WireValue::Int(since)),
            )),
            OtherMacPage::Refused { status, body } => Err(self.refusal_passed_on(&mac, status, &body)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::on_loopback;

    #[test]
    fn a_page_on_loopback_is_its_port_and_the_rest_of_its_address() {
        assert_eq!(on_loopback("http://localhost:5173/?walk=one"), Some((5173, String::from("/?walk=one"))));
        assert_eq!(on_loopback("http://127.0.0.1:8080"), Some((8080, String::from("/"))));
        assert_eq!(on_loopback("http://[::1]:3000/a/b#c"), Some((3000, String::from("/a/b#c"))));
        assert_eq!(on_loopback("http://LOCALHOST/x"), Some((80, String::from("/x"))));
        assert_eq!(on_loopback("http://localhost:4000?q=1"), Some((4000, String::from("/?q=1"))));
    }

    #[test]
    fn a_page_anywhere_else_is_not_on_loopback() {
        for url in ["https://localhost:5173/", "http://staging.example/", "http://10.0.0.2:5173/", "http://localhost:x/", "armada://walk/one", "http://localhost.evil:80/"] {
            assert_eq!(on_loopback(url), None, "{url}");
        }
    }
}
