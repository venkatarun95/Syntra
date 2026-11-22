//! Bounds on C from a whole history, not one QE window.
//!
//! A QE window has six transitions, and the link may serve up to one RTT
//! behind its ideal service curve, so one window proves C only to within
//! about 1/(T-1) of what it served: e.g. `C <= 12.5` for a link with C = 10.
//! Intersecting the windows (even jointly in (C, B)) does not help, since
//! every window's initial queue is free.  Over a long history the same lag
//! is amortized over many RTTs.  These bounds use only the equations of the
//! bursty CBR-delay model (`qe/qe/environment/bursty_cbr_delay.py`, D = 1;
//! the plain model is K = 0), applied to one link over the whole history:
//!
//! * `I[t] = min(I[t-1] + C, P[t] - L[t])`, `A[t] - K <= P[t] <= A[t]`,
//! * `I[t-1] <= S[t] <= I[t]`.
//!
//! Unrolling the first from `t0` to `t1`,
//!
//! `I[t1] = min(I[t0] + (t1-t0) C, min_{t0 < tau <= t1} P[tau] - L[tau] + (t1-tau) C)`.
//!
//! Lower bound: `S[t1] <= I[t1] <= I[t0] + (t1-t0) C <= S[t0+1] + (t1-t0) C`.
//!
//! Upper bound: `S[t1+1] >= I[t1]`, so some term of the min is at most
//! `S[t1+1]`.  With `I[t0] >= S[t0]`, `P >= A - K` and an upper bound on L
//! up to `t1` (L never decreases, so any later reported loss is one), every
//! term but the `tau = t1` one gives C at most a value, and that one does
//! not involve C: if `S[t1+1] < A[t1] - K - L[t1]`,
//!
//! `C <= max((S[t1+1] - S[t0]) / (t1-t0), max_{t0 < tau < t1} (S[t1+1] - A[tau] + K + L[tau]) / (t1-tau))`.
//!
//! No backlog assumption is needed: a gap in the backlog just makes some
//! `tau` term large.  Time `t` is QE time (`get_observation_vectors`).

use ds::lin::{widen, Wide};
use ds::RealNumRep;

/// `(lower, upper)` bounds on C (`upper` is `None` if the history proves
/// none), for the link with burstiness at most `k` behind the arrivals `a`
/// and services `s` (QE time), given the losses reported for some times
/// (`(t, L[t])`).  Not rounded.
pub fn c_bounds(a: &[RealNumRep], losses: &[(usize, RealNumRep)], s: &[RealNumRep], k: RealNumRep) -> (Wide, Option<Wide>) {
    let n = std::cmp::min(a.len(), s.len());
    let (a, s, k) = (
        a[..n].iter().map(|&x| widen(x)).collect::<Vec<_>>(),
        s[..n].iter().map(|&x| widen(x)).collect::<Vec<_>>(),
        widen(k),
    );
    // An upper bound on L[t]: the smallest loss reported for t or later.
    let mut l: Vec<Option<Wide>> = vec![None; n];
    for &(t, x) in losses {
        for u in 0..=std::cmp::min(t, n.saturating_sub(1)) {
            let x = widen(x);
            l[u] = Some(l[u].map_or(x, |y| std::cmp::min(x, y)));
        }
    }
    let span = |d: usize| Wide::from_integer(d as i128);

    let mut lo = Wide::from_integer(0);
    for t1 in 1..n {
        for t0 in 0..t1 {
            if t0 + 1 < n {
                lo = std::cmp::max(lo, (s[t1] - s[t0 + 1]) / span(t1 - t0));
            }
        }
    }

    let mut hi: Option<Wide> = None;
    for t1 in 0..n.saturating_sub(1) {
        match l[t1] {
            Some(l1) if s[t1 + 1] < a[t1] - k - l1 => {}
            _ => continue,
        }
        // Max of the `tau` terms with t0 < tau < t1, grown as t0 decreases.
        let mut taus: Option<Wide> = None;
        for t0 in (0..t1).rev() {
            let base = (s[t1 + 1] - s[t0]) / span(t1 - t0);
            let u = taus.map_or(base, |m| std::cmp::max(m, base));
            hi = Some(hi.map_or(u, |h| std::cmp::min(h, u)));
            // L[t0] <= L[t1], which is bounded.
            let tau = (s[t1 + 1] - a[t0] + k + l[t0].unwrap()) / span(t1 - t0);
            taus = Some(taus.map_or(tau, |m| std::cmp::max(m, tau)));
        }
    }
    (lo, hi)
}
