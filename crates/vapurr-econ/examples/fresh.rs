//! Fresh testnet book. Prints addresses and txs. No keys.

fn main() {
    let mut c = vapurr_econ::Client::open();
    let cmd = std::env::args().nth(1).unwrap_or_else(|| "snap".into());
    let r = match cmd.as_str() {
        "deploy" => c.run(vapurr_econ::EconCmd::Deploy),
        "house" => c.run(vapurr_econ::EconCmd::HouseDeploy),
        "bootstrap" => c.run(vapurr_econ::EconCmd::HouseBootstrap),
        "loop" => c.run(vapurr_econ::EconCmd::LoopDeploy),
        "swap" => c.run(vapurr_econ::EconCmd::SwapDeploy),
        "mint" => c.run(vapurr_econ::EconCmd::Mint("200".into())),
        "supply" => c.run(vapurr_econ::EconCmd::LoopOp {
            op: "supply".into(),
            amt: "50".into(),
            steps: "".into(),
        }),
        "collat" => c.run(vapurr_econ::EconCmd::LoopOp {
            op: "depositV".into(),
            amt: "200".into(),
            steps: "".into(),
        }),
        "eloop" => c.run(vapurr_econ::EconCmd::LoopOp {
            op: "loop".into(),
            amt: "".into(),
            steps: "3".into(),
        }),
        "swapv" => c.run(vapurr_econ::EconCmd::HouseSwap {
            sell_v: true,
            amt: "10".into(),
        }),
        "swapp" => c.run(vapurr_econ::EconCmd::HouseSwap {
            sell_v: false,
            amt: "8".into(),
        }),
        "pulse" => c.run(vapurr_econ::EconCmd::Pulse),
        _ => Ok(c.snapshot()),
    };
    match r {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => {
            eprintln!("{}: {}", e.which, e.msg);
            std::process::exit(1);
        }
    }
}
