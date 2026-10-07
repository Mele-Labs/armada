//! What the listener does about a boot that fails, or a stop that comes first.
//!
//! Fleet serves before it has reconciled, so the two end in a race: reconcile
//! finishing, reconcile failing, or somebody asking Fleet to stop. This is the
//! one place that says what each does, apart from the process so a test can
//! drive it.

use std::error::Error;
use std::future::Future;

use axum::Router;
use tokio::net::TcpListener;

/// Serve `app` while `boot` runs in a task, until `stop` resolves or `boot` fails.
///
/// - **`boot` fails**: the server stops and this returns that error.
/// - **`stop` first**: `boot` is ended where it stands, which is what a crash
///   leaves, and the next start repairs it. Returns `None`.
/// - **`boot` finishes**: serving carries on until `stop`, and this returns what
///   it made.
///
/// `boot` is dropped on a failure or an abort, so whatever it holds — the
/// `api::Reconciliation` a held command waits on — goes with it and the
/// command is answered. A holder kept outside it would leave graceful shutdown
/// waiting on a command nothing can release.
pub(crate) async fn serve_while_booting<T, B>(
    listener: TcpListener,
    app: Router,
    stop: impl Future<Output = ()> + Send + 'static,
    boot: B,
) -> Result<Option<T>, Box<dyn Error>>
where
    T: Send + 'static,
    B: Future<Output = Result<T, String>> + Send + 'static,
{
    let (failed, boot_failed) = tokio::sync::oneshot::channel::<()>();
    let booting = tokio::spawn(async move {
        let booted = boot.await;
        if booted.is_err() {
            let _ = failed.send(());
        }
        booted
    });
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::select! {
            () = stop => {}
            // One that finished drops its sender too, which is not a reason to stop.
            Ok(()) = boot_failed => {}
        }
    })
    .await?;
    match booting.is_finished() {
        true => Ok(Some(booting.await??)),
        false => {
            booting.abort();
            Ok(None)
        }
    }
}
