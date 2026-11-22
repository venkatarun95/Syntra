//! A forward simulator of the bursty CBR-delay link that carries the hidden
//! state (ideal service `I`, cumulative loss `L`) across RTTs.
//!
//! `NetworkModelNC::get_network_move_sim_policy` picks each service from the
//! QE interval of the newest window alone.  Every window's initial queue is
//! free in QE, so a sequence of such choices can be feasible window by window
//! but impossible overall (e.g. picking the smallest service each RTT starves
//! the link forever although K bounds the total delay).  This simulator
//! instead applies the model's equations (`qe/environment/bursty_cbr_delay.py`)
//! with D = 1:
//!
//! * `P[t] = A[t] - delta[t]`, `0 <= delta[t] <= K` (the policy picks delta),
//! * `I[t] = min(I[t-1] + C, P[t] - L[t])`,
//! * `L[t] = A[t] - I[t] - B` if `A[t] - L[t-1] - I[t] > B`, else `L[t-1]`,
//! * `max(S[t-1], I[t-1]) <= S[t] <= I[t]` (the policy picks where),
//! * the loss at `t` is observed once `S >= A[t] - L[t]`.
//!
//! Time `t` here is QE time: `A[t]` is the `a` of observation `t` and `S[t]`
//! the `s` of observation `t + 1`, as in `get_observation_vectors`.

use ds::*;

use crate::{AppSendFraction, LossObservation, NetworkActionNC};

#[derive(Clone, Debug)]
pub struct BurstyLinkSim {
    pub c: RealNumRep,
    pub b: RealNumRep,
    pub k: RealNumRep,
    /// A, I, L at every QE time so far.
    a: Vec<RealNumRep>,
    i: Vec<RealNumRep>,
    l: Vec<RealNumRep>,
    last_s: RealNumRep,
    /// Index of the first loss not yet reported.
    next_unobserved: usize,
}

/// Where in its allowed range each nondeterministic choice falls: 0 is the
/// smallest value, 1 the largest.
#[derive(Clone, Copy, Debug)]
pub struct LinkChoice {
    /// Fraction of K by which the arrivals are delayed (`delta / K`).
    pub burst: RealNumRep,
    /// Fraction of `[I[t-1], I[t]]` that has been delivered (`S`).
    pub service: RealNumRep,
}

impl BurstyLinkSim {
    /// A link that starts empty, matching the all-zero initial history of
    /// `get_initial_history` (`n_initial` observations).
    pub fn new(c: RealNumRep, b: RealNumRep, k: RealNumRep, n_initial: usize) -> Self {
        assert!(n_initial >= 1);
        let zero: RealNumRep = 0.into();
        Self {
            c,
            b,
            k,
            a: vec![zero; n_initial - 1],
            i: vec![zero; n_initial - 1],
            l: vec![zero; n_initial - 1],
            last_s: zero,
            next_unobserved: n_initial - 1,
        }
    }

    /// The network's response to the newest observation, whose cumulative
    /// arrivals were `a_prev` (i.e. the service of everything sent up to it).
    /// `history_len` is the number of observations so far.
    pub fn step(&mut self, a_prev: RealNumRep, history_len: usize, choice: LinkChoice) -> NetworkActionNC {
        let t = self.a.len();
        assert_eq!(t + 1, history_len, "simulator out of step with the history");
        let (i_prev, l_prev) = (*self.i.last().unwrap(), *self.l.last().unwrap());
        // Ideal service never decreases (S is non-decreasing and S <= I), so
        // a burst may delay arrivals only down to what was already served.
        let p = std::cmp::max(a_prev - self.k * clamp01(choice.burst), i_prev + l_prev);
        let (mut i, mut l) = (std::cmp::min(i_prev + self.c, p - l_prev), l_prev);
        if a_prev - l_prev - i > self.b {
            // Overflow.  P - L >= I + B - K > I, so the link is backlogged.
            i = i_prev + self.c;
            l = a_prev - i - self.b;
        }
        self.a.push(a_prev);
        self.i.push(i);
        self.l.push(l);

        let lo_s = std::cmp::max(self.last_s, i_prev);
        // Round down to a 1/16 grid to keep rational denominators bounded.
        let s = std::cmp::max(lo_s, lo_s + ((i - lo_s) * clamp01(choice.service) * 16).floor() / 16);
        let s = std::cmp::min(s, i);
        self.last_s = s;

        // Report every loss that has become observable, in order.  The
        // observation at index `history_len` reports L[tau] as `t` RTTs ago,
        // with `t = history_len - 1 - tau`; with nothing new it repeats the
        // newest observed loss, as the QE-based simulator does.
        let mut lo = vec![];
        while self.next_unobserved <= t && s >= self.a[self.next_unobserved] - self.l[self.next_unobserved] {
            let tau = self.next_unobserved;
            lo.push(LossObservation { t: history_len - 1 - tau, l: self.l[tau] });
            self.next_unobserved += 1;
        }
        if lo.is_empty() {
            let tau = self.next_unobserved - 1;
            lo.push(LossObservation { t: history_len - 1 - tau, l: self.l[tau] });
        }
        NetworkActionNC { s, lo, app_send_fraction: AppSendFraction::Full }
    }
}

fn clamp01(x: RealNumRep) -> RealNumRep {
    std::cmp::min(std::cmp::max(x, 0.into()), 1.into())
}
