//! Bounded protocol workers keep network calls and native authorization off the UI thread.
use super::*;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
type Job = Box<dyn FnOnce() + Send>;

fn queue() -> &'static mpsc::SyncSender<Job> {
    static QUEUE: OnceLock<mpsc::SyncSender<Job>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::sync_channel::<Job>(8);
        let rx = Arc::new(Mutex::new(rx));
        for i in 0..4 {
            let rx = rx.clone();
            std::thread::Builder::new().name(format!("chrome-api-{i}")).spawn(move || loop {
                let job = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                match job { Ok(job) => job(), Err(_) => break }
            }).expect("chrome API worker");
        }
        tx
    })
}

pub(super) fn error(status: u16, message: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder().status(status).header(CONTENT_TYPE, "application/json")
        .header("Cache-Control", "no-store")
        .body(Cow::Owned(serde_json::json!({"ok":false,"error":message}).to_string().into_bytes())).unwrap()
}

pub(super) fn is_mutation(rel: &str) -> bool {
    ["zzzmail/api/send", "zzzmail/api/hood/register", "zzzmail/api/pns/register",
     "zzzmail/api/pns/deploy", "zzzmail/api/pns/set-addr", "zzzmail/api/pns/set-name"]
        .iter().any(|p| rel == *p || rel.strip_prefix(p).map(|s| s.starts_with('/')).unwrap_or(false))
}

pub fn serve_async(_id: wry::WebViewId<'_>, req: wry::http::Request<Vec<u8>>, responder: wry::RequestAsyncResponder) {
    if !req.uri().path().contains("/api") {
        responder.respond(serve("", req)); return;
    }
    if !crate::security::api_authorized(&req) {
        responder.respond(error(403, "Untrusted API caller")); return;
    }
    let response = Arc::new(Mutex::new(Some(responder)));
    let output = response.clone();
    let submitted = Instant::now();
    let task: Job = Box::new(move || {
        let result = if submitted.elapsed() > Duration::from_secs(15) {
            error(408, "Request expired; try again")
        } else { serve("", req) };
        if let Some(responder) = output.lock().unwrap_or_else(|e| e.into_inner()).take() {
            responder.respond(result);
        }
    });
    if queue().try_send(task).is_err() {
        if let Some(responder) = response.lock().unwrap_or_else(|e| e.into_inner()).take() {
            responder.respond(error(503, "Browser API is busy; try again"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_mutating_mail_routes_require_post() {
        for path in ["zzzmail/api/send", "zzzmail/api/pns/deploy", "zzzmail/api/pns/set-name/", "zzzmail/api/pns/register/alice", "zzzmail/api/hood/register/alice"] {
            assert!(is_mutation(path), "{path}");
            let req = wry::http::Request::builder().uri(format!("http://vapurr.localhost/{path}"))
                .header("x-vapurr-client", crate::security::api_token()).body(vec![]).unwrap();
            assert_eq!(serve("", req).status(), 405);
        }
    }
    #[test]
    fn external_private_api_reads_are_rejected_before_opening_profile() {
        let req = wry::http::Request::builder().uri("http://vapurr.localhost/zzzmail/api/inbox")
            .header("origin", "https://evil.invalid").body(vec![]).unwrap();
        assert_eq!(serve("", req).status(), 403);
    }
    #[test]
    fn dispatch_does_not_wait_for_network_work() {
        let (done, rx) = mpsc::channel();
        let start = Instant::now();
        queue().try_send(Box::new(move || { std::thread::sleep(Duration::from_millis(150)); let _ = done.send(()); })).unwrap();
        assert!(start.elapsed() < Duration::from_millis(100));
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
    }
}
