//! Replay a `cbrdelay-cca --external --record FILE` transcript and test every QE window under
//! each burstiness K, to find observations the bursty CBR-delay model cannot explain.
//!
//!     cargo run --release -p network_model_nc --example window_check -- FILE [K...]

use cc_common::NetworkModel;
use ds::*;
use network_model_nc::{BurstyQe, LossObservation, NetworkModelNC, ObservationNC};
use num_traits::ToPrimitive;
use std::sync::Arc;

fn real(x: &str) -> RealNumRep {
    let v: f64 = x.parse().unwrap();
    RealNumRep::new((v * 16.).round() as RealNumInt, 16)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let ks: Vec<RealNumInt> = if args.len() > 2 {
        args[2..].iter().map(|k| k.parse().unwrap()).collect()
    } else {
        vec![1, 4, 16, 64]
    };
    let mut nm = NetworkModelNC::new();
    let mut history = nm.get_initial_history();
    let n0 = history.len() as i64;
    let mut last_loss = (n0 - 1, RealNumRep::from(0));
    for (round, line) in std::fs::read_to_string(&args[1]).unwrap().lines().enumerate() {
        let mut it = line.split_whitespace();
        let sent = real(it.next().unwrap());
        let s = real(it.next().unwrap());
        let _app = it.next();
        let now = history.len() as i64;
        let mut lo: Vec<LossObservation> = it
            .map(|p| {
                let (r, l) = p.split_once(':').unwrap();
                let tau = n0 + r.parse::<i64>().unwrap();
                last_loss = (tau, real(l));
                LossObservation { t: (now - 1 - tau) as usize, l: real(l) }
            })
            .collect();
        if lo.is_empty() {
            lo.push(LossObservation { t: (now - 1 - last_loss.0) as usize, l: last_loss.1 });
        }
        let a = history.last().unwrap().a + sent;
        history.push(ObservationNC { a, s, lo });
        let window = &history[history.len() - 7..];
        let mut out = vec![];
        for &k in &ks {
            nm.set_qe(Arc::new(BurstyQe { k: k.into(), pert: RealNumRep::new(1, 2) }));
            out.push(match nm.try_window_belief(window) {
                Some(b) => format!(
                    "K={k}: c=[{:.2},{:.2}] b=[{:.2},{:.2}]",
                    b.min_c.to_f64().unwrap(),
                    b.max_c.to_f64().unwrap(),
                    b.min_b.to_f64().unwrap(),
                    b.max_b.to_f64().unwrap()
                ),
                None => format!("K={k}: INFEASIBLE"),
            });
        }
        println!("round {round:3} ({line}): {}", out.join("  "));
        if out.iter().all(|o| o.contains("INFEASIBLE")) {
            for o in window {
                println!("    {o}");
            }
        }
    }
}
