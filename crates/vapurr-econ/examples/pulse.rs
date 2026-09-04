//! On-chain movement bot. House tape, Euler only if healthy. No keys printed.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn say(ok: bool, line: &str) {
    if ok {
        println!("{line}");
        let _ = io::stdout().flush();
    } else {
        eprintln!("{line}");
        let _ = io::stderr().flush();
    }
}

fn main() {
    let mut c = vapurr_econ::Client::open();
    let once = std::env::args().any(|a| a == "--once");
    loop {
        match c.run(vapurr_econ::EconCmd::Pulse) {
            Ok(v) => {
                let tx = v.get("tx").and_then(|x| x.as_str()).unwrap_or("");
                let p = v.get("pusd").and_then(|x| x.as_str()).unwrap_or("?");
                let vv = v.get("vapurr").and_then(|x| x.as_str()).unwrap_or("?");
                let notes = v.get("notes").and_then(|x| x.as_str()).unwrap_or("");
                let hf = v
                    .get("loop")
                    .and_then(|x| x.get("health"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("?");
                say(true, &format!("ok V={vv} P={p} hf={hf} {notes} tx={tx}"));
            }
            Err(e) => say(false, &format!("{}: {}", e.which, e.msg)),
        }
        if once {
            break;
        }
        thread::sleep(Duration::from_secs(32));
    }
}
