use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use num_traits::ToPrimitive;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Bound;

use cc_common::*;
use ds::interval::{Interval, IntervalList};
use ds::lin::Lin;
use ds::*;
use lazy_static::lazy_static;
use network_model_nc_qe_output::*;
use std::sync::Arc;

pub mod chain;
pub mod exact;
pub mod generated;
pub mod link_sim;
pub mod qe_tables;
pub use link_sim::{BurstyLinkSim, LinkChoice};
pub use qe_tables::{BurstyQe, PlainQe, QeTables, UnionBurstyQe};

const N_DISCRETE_POINTS: usize = 2;
const EPS: RealNumRep = RealNumRep::new_raw(1, 1 << 4);

/// `NetworkModelNC::exact_pieces` is pure in the QE tables and the window,
/// and the minimax tree asks for the same windows many times.
#[derive(Clone, PartialEq, Eq, Hash)]
struct PiecesKey {
    /// The tables' `Debug` form (e.g. the K set), not their address: a
    /// freed `Arc`'s address can be reused by different tables.
    qe: String,
    t: i32,
    n: i32,
    obs: Vec<RealNumRep>,
}

const PIECES_CACHE_SIZE: usize = 200_000;

type JointKey = (String, u64, Vec<ObservationNC>, Option<RealNumRep>);

thread_local! {
    /// `NetworkModelNC::joint_pieces` per history prefix.
    static JOINT_CACHE: std::cell::RefCell<HashMap<JointKey, Vec<exact::Piece>>> =
        std::cell::RefCell::new(HashMap::new());
}

thread_local! {
    static PIECES_CACHE: std::cell::RefCell<HashMap<PiecesKey, Option<Vec<exact::Piece>>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// The closure's lower end of a bounded-below set: a sound outer bound
/// (unlike `my_min`, which moves inward from an open end and rounds).
fn outer_min(x: &IntervalList<RealNumRep>) -> RealNumRep {
    match x.min() {
        Bound::Included(v) | Bound::Excluded(v) => v,
        Bound::Unbounded => unreachable!("outer_min of a set unbounded below"),
    }
}

/// See `outer_min`.
fn outer_max(x: &IntervalList<RealNumRep>) -> RealNumRep {
    match x.max() {
        Bound::Included(v) | Bound::Excluded(v) => v,
        Bound::Unbounded => unreachable!("outer_max of a set unbounded above"),
    }
}

/// A generated function asserts the clauses of its formula that do not
/// mention its result; failing one means the inputs (e.g. a (C, B) that no
/// K fits) are infeasible, i.e. the result is the empty set.  `None` still
/// means that no table exists.
fn infeasible_is_empty(
    f: impl FnOnce() -> Option<IntervalList<RealNumRep>>,
) -> Option<IntervalList<RealNumRep>> {
    qe_tables::quietly_catch(f).unwrap_or_else(|| Some(IntervalList::new(vec![])))
}

fn bw_range() -> IntervalList<RealNumRep> {
    IntervalList::interval_bounded(Bound::Included(SMALLEST_BW.into()), Bound::Included(LARGEST_BW.into()))
}

lazy_static! {
    // The key is: (trace length, prior losses observed, total losses observed)
    pub static ref COMPUTE_L_OBS: HashMap<(i32, i32, i32), QeFun> =
        HashMap::from([
            ((2, 0, 0), compute_l_0_t_2_l_0 as QeFun),
            ((3, 0, 0), compute_l_0_t_3_l_0 as QeFun),
            ((4, 0, 0), compute_l_0_t_4_l_0 as QeFun),
            ((5, 0, 0), compute_l_0_t_5_l_0 as QeFun),
            ((6, 0, 0), compute_l_0_t_6_l_0 as QeFun),

            ((2, 1, 1), compute_l_1_t_2_l_1_nl_0 as QeFun),
            ((2, 0, 1), compute_l_1_t_2_l_0_nl_0 as QeFun),
            ((3, 1, 1), compute_l_1_t_3_l_1_nl_0 as QeFun),
            ((3, 0, 1), compute_l_1_t_3_l_0_nl_0 as QeFun),
            ((4, 1, 1), compute_l_1_t_4_l_1_nl_0 as QeFun),
            ((4, 0, 1), compute_l_1_t_4_l_0_nl_0 as QeFun),
            ((5, 1, 1), compute_l_1_t_5_l_1_nl_0 as QeFun),
            ((5, 0, 1), compute_l_1_t_5_l_0_nl_0 as QeFun),
            ((6, 1, 1), compute_l_1_t_6_l_1_nl_0 as QeFun),
            ((6, 0, 1), compute_l_1_t_6_l_0_nl_0 as QeFun),


            ((3, 2, 2), compute_l_2_t_3_l_2_nl_1 as QeFun),
            ((3, 1, 2), compute_l_2_t_3_l_1_nl_1 as QeFun),
            ((3, 0, 2), compute_l_2_t_3_l_0_nl_1 as QeFun),
            ((4, 2, 2), compute_l_2_t_4_l_2_nl_1 as QeFun),
            ((4, 1, 2), compute_l_2_t_4_l_1_nl_1 as QeFun),
            ((4, 0, 2), compute_l_2_t_4_l_0_nl_1 as QeFun),
            ((5, 2, 2), compute_l_2_t_5_l_2_nl_1 as QeFun),
            ((5, 1, 2), compute_l_2_t_5_l_1_nl_1 as QeFun),
            ((5, 0, 2), compute_l_2_t_5_l_0_nl_1 as QeFun),
            ((6, 2, 2), compute_l_2_t_6_l_2_nl_1 as QeFun),
            ((6, 1, 2), compute_l_2_t_6_l_1_nl_1 as QeFun),
            ((6, 0, 2), compute_l_2_t_6_l_0_nl_1 as QeFun),

            ((4, 3, 3), compute_l_3_t_4_l_3_nl_2 as QeFun),
            ((4, 2, 3), compute_l_3_t_4_l_2_nl_2 as QeFun),
            ((4, 1, 3), compute_l_3_t_4_l_1_nl_2 as QeFun),
            ((4, 0, 3), compute_l_3_t_4_l_0_nl_2 as QeFun),
            ((5, 3, 3), compute_l_3_t_5_l_3_nl_2 as QeFun),
            ((5, 2, 3), compute_l_3_t_5_l_2_nl_2 as QeFun),
            ((5, 1, 3), compute_l_3_t_5_l_1_nl_2 as QeFun),
            ((5, 0, 3), compute_l_3_t_5_l_0_nl_2 as QeFun),
            ((6, 3, 3), compute_l_3_t_6_l_3_nl_2 as QeFun),
            ((6, 2, 3), compute_l_3_t_6_l_2_nl_2 as QeFun),
            ((6, 1, 3), compute_l_3_t_6_l_1_nl_2 as QeFun),
            ((6, 0, 3), compute_l_3_t_6_l_0_nl_2 as QeFun),

            ((5, 4, 4), compute_l_4_t_5_l_4_nl_3 as QeFun),
            ((5, 3, 4), compute_l_4_t_5_l_3_nl_3 as QeFun),
            ((5, 2, 4), compute_l_4_t_5_l_2_nl_3 as QeFun),
            ((5, 1, 4), compute_l_4_t_5_l_1_nl_3 as QeFun),
            ((5, 0, 4), compute_l_4_t_5_l_0_nl_3 as QeFun),
            ((6, 4, 4), compute_l_4_t_6_l_4_nl_3 as QeFun),
            ((6, 3, 4), compute_l_4_t_6_l_3_nl_3 as QeFun),
            ((6, 2, 4), compute_l_4_t_6_l_2_nl_3 as QeFun),
            ((6, 1, 4), compute_l_4_t_6_l_1_nl_3 as QeFun),
            ((6, 0, 4), compute_l_4_t_6_l_0_nl_3 as QeFun),

            ((6, 5, 5), compute_l_5_t_6_l_5_nl_4 as QeFun),
            ((6, 4, 5), compute_l_5_t_6_l_4_nl_4 as QeFun),
            ((6, 3, 5), compute_l_5_t_6_l_3_nl_4 as QeFun),
            ((6, 2, 5), compute_l_5_t_6_l_2_nl_4 as QeFun),
            ((6, 1, 5), compute_l_5_t_6_l_1_nl_4 as QeFun),
            ((6, 0, 5), compute_l_5_t_6_l_0_nl_4 as QeFun),
        ]);
        pub static ref COMPUTE_Q: HashMap<(i32, i32), QeFun> = HashMap::from([
            ((6, 0), compute_q_5_t_6_l_0 as QeFun),
            ((6, 1), compute_q_5_t_6_l_1 as QeFun),
            ((6, 2), compute_q_5_t_6_l_2 as QeFun),
            ((6, 3), compute_q_5_t_6_l_3 as QeFun),
            ((6, 4), compute_q_5_t_6_l_4 as QeFun),
            ((6, 5), compute_q_5_t_6_l_5 as QeFun),
            ((6, 6), compute_q_5_t_6_l_6 as QeFun),
            ((5, 0), compute_q_4_t_5_l_0 as QeFun),
            ((5, 1), compute_q_4_t_5_l_1 as QeFun),
            ((5, 2), compute_q_4_t_5_l_2 as QeFun),
            ((5, 3), compute_q_4_t_5_l_3 as QeFun),
            ((5, 4), compute_q_4_t_5_l_4 as QeFun),
            ((5, 5), compute_q_4_t_5_l_5 as QeFun),
            ((5, 0), compute_q_4_t_5_l_0 as QeFun),
            ((5, 1), compute_q_4_t_5_l_1 as QeFun),
            ((5, 2), compute_q_4_t_5_l_2 as QeFun),
            ((5, 3), compute_q_4_t_5_l_3 as QeFun),
            ((5, 4), compute_q_4_t_5_l_4 as QeFun),
            ((5, 5), compute_q_4_t_5_l_5 as QeFun),
            ((4, 0), compute_q_3_t_4_l_0 as QeFun),
            ((4, 1), compute_q_3_t_4_l_1 as QeFun),
            ((4, 2), compute_q_3_t_4_l_2 as QeFun),
            ((4, 3), compute_q_3_t_4_l_3 as QeFun),
            ((4, 4), compute_q_3_t_4_l_4 as QeFun),
            ((3, 0), compute_q_2_t_3_l_0 as QeFun),
            ((3, 1), compute_q_2_t_3_l_1 as QeFun),
            ((3, 2), compute_q_2_t_3_l_2 as QeFun),
            ((3, 3), compute_q_2_t_3_l_3 as QeFun),
            ((2, 0), compute_q_1_t_2_l_0 as QeFun),
            ((2, 1), compute_q_1_t_2_l_1 as QeFun),
            ((2, 2), compute_q_1_t_2_l_2 as QeFun),
        ]);
        pub static ref COMPUTE_S: HashMap<(i32, i32), QeFun> = HashMap::from([
            ((6, 0), compute_s_5_t_6_l_0 as QeFun),
            ((6, 1), compute_s_5_t_6_l_1 as QeFun),
            ((6, 2), compute_s_5_t_6_l_2 as QeFun),
            ((6, 3), compute_s_5_t_6_l_3 as QeFun),
            ((6, 4), compute_s_5_t_6_l_4 as QeFun),
            ((6, 5), compute_s_5_t_6_l_5 as QeFun),
            ((5, 0), compute_s_4_t_5_l_0 as QeFun),
            ((5, 1), compute_s_4_t_5_l_1 as QeFun),
            ((5, 2), compute_s_4_t_5_l_2 as QeFun),
            ((5, 3), compute_s_4_t_5_l_3 as QeFun),
            ((5, 4), compute_s_4_t_5_l_4 as QeFun),
            ((4, 0), compute_s_3_t_4_l_0 as QeFun),
            ((4, 1), compute_s_3_t_4_l_1 as QeFun),
            ((4, 2), compute_s_3_t_4_l_2 as QeFun),
            ((4, 3), compute_s_3_t_4_l_3 as QeFun),
            ((3, 0), compute_s_2_t_3_l_0 as QeFun),
            ((3, 1), compute_s_2_t_3_l_1 as QeFun),
            ((3, 2), compute_s_2_t_3_l_2 as QeFun),
            ((2, 0), compute_s_1_t_2_l_0 as QeFun),
            ((2, 1), compute_s_1_t_2_l_1 as QeFun),
        ]);
}

const SIMPLE_DEN: RealNumInt = 1 << 10;
/// `x` rounded down to a multiple of 1/`SIMPLE_DEN`, computed in `i64` so a
/// large numerator cannot overflow.
fn simplify_down(x: RealNumRep) -> RealNumRep {
    let q = (*x.numer() as i64 * SIMPLE_DEN as i64).div_euclid(*x.denom() as i64);
    RealNumRep::new(q as RealNumInt, SIMPLE_DEN)
}

/// See `simplify_down`.
fn simplify_up(x: RealNumRep) -> RealNumRep {
    let n = *x.numer() as i64 * SIMPLE_DEN as i64;
    let d = *x.denom() as i64;
    let q = n.div_euclid(d) + if n.rem_euclid(d) != 0 { 1 } else { 0 };
    RealNumRep::new(q as RealNumInt, SIMPLE_DEN)
}

fn my_min(x: &IntervalList<RealNumRep>) -> RealNumRep {
    // The + or - EPS is workaround when the interval is open instead of
    // closed. The binary search earlier anyway had precision error. So
    // this EPS should ideally not create issues.

    // This workaround only works if the open point is the inf or sup of the
    // set. Ideally, since we believe that our sets are convex (along each
    // dimension), this should always be the case.

    // if x.is_point() {
    //     return x.min().unwrap();
    // }

    // let ret = x.min().unwrap() + EPS;
    // if let Some(x_max) = x.max() {
    //     if !(ret < x_max) {
    //         return (x.min().unwrap() + x.max().unwrap()) / 2;
    //     }
    // }
    // ret

    // x.min().unwrap()

    // if x.is_point() {
    //     return x.min().unwrap();
    // }
    // let ret = x.min().unwrap();
    // if ret.denom() > &SIMPLE_DEN {
    //     let try_ret = simplify_up(ret);
    //     if x.max().is_none() || try_ret <= x.max().unwrap() {
    //         return try_ret;
    //     }
    // }

    let xmin = match x.min() {
        Bound::Unbounded => unreachable!(),
        Bound::Included(a) => a,
        Bound::Excluded(a) => match x.max() {
            Bound::Unbounded => a + EPS,
            Bound::Included(b) | Bound::Excluded(b) => std::cmp::min(a + EPS, (a + b) / 2),
        },
    };

    // if xmin.denom() > &SIMPLE_DEN {
    let try_ret = simplify_up(xmin);
    if Interval::is_non_empty(Bound::Included(try_ret), x.max()) {
        return try_ret;
    }
    // }
    xmin
}

fn my_max(x: &IntervalList<RealNumRep>) -> RealNumRep {
    // if x.is_point() {
    //     return x.max().unwrap();
    // }
    // let ret = x.max().unwrap() - EPS;
    // if let Some(x_min) = x.min() {
    //     if !(ret > x_min) {
    //         return (x.min().unwrap() + x.max().unwrap()) / 2;
    //     }
    // }
    // ret

    // x.max().unwrap()

    // if x.is_point() {
    //     return x.max().unwrap();
    // }
    // let ret = x.max().unwrap();
    // if ret.denom() > &SIMPLE_DEN {
    //     let try_ret = simplify_down(ret);
    //     if x.min().is_none() || try_ret >= x.min().unwrap() {
    //         return try_ret;
    //     }
    // }

    let xmax = match x.max() {
        Bound::Unbounded => unreachable!(),
        Bound::Included(b) => b,
        Bound::Excluded(b) => match x.min() {
            Bound::Unbounded => b - EPS,
            Bound::Included(a) | Bound::Excluded(a) => std::cmp::max(b - EPS, (a + b) / 2),
        },
    };

    // if xmax.denom() > &SIMPLE_DEN {
    // TODO: Simplify denominator to nearest power of 2 when den < SIMPLE_DEN
    let try_ret = simplify_down(xmax);
    if Interval::is_non_empty(x.min(), Bound::Included(try_ret)) {
        return try_ret;
    }
    // }
    xmax
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LossObservation {
    // cum. bytes detected as lost at t time ago from now.
    pub t: usize,
    #[serde(with = "serde_real")]
    pub l: RealNumRep,
}

impl Debug for LossObservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {:.2})", self.t, self.l.to_f64().unwrap())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ObservationNC {
    #[serde(with = "serde_real")]
    pub a: RealNumRep, // cum. bytes arrived
    #[serde(with = "serde_real")]
    pub s: RealNumRep, // cum. bytes serviced
    pub lo: Vec<LossObservation>, // This vector should always be non-empty as it describes the rtt, and the loss sequence an rtt ago.
                                  // new fields for abr
}

impl Display for ObservationNC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a={:.2}, s={:.2}, lo={:?}",
            self.a.to_f64().unwrap(),
            self.s.to_f64().unwrap(),
            self.lo,
        )
    }
}

impl Observation for ObservationNC {
    fn get_ld(&self) -> RealNumRep {
        assert!(!self.lo.is_empty());
        self.lo.last().unwrap().l
    }

    fn get_qdel(&self) -> usize {
        assert!(!self.lo.is_empty());
        // We are returning the smallest qdel of the packets ACKed.
        self.lo.last().unwrap().t
    }

    fn get_s(&self) -> RealNumRep {
        self.s
    }

    fn get_a(&self) -> RealNumRep {
        self.a
    }

    fn get_lo(&self) -> Vec<(usize, RealNumRep)> {
        self.lo.clone().into_iter().map(|l| (l.t, l.l)).collect()
    }
}

#[derive(Clone, PartialEq, Debug, Eq, Hash, Serialize)]
pub struct NetworkActionNC {
    #[serde(with = "serde_real")]
    pub s: RealNumRep,
    pub lo: Vec<LossObservation>,
    pub app_send_fraction: AppSendFraction,
}

impl Display for NetworkActionNC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "s={:.2}, app={}, lo={:?}",
            self.s.to_f64().unwrap(),
            self.app_send_fraction,
            self.lo,
        )
    }
}

impl NetworkAction for NetworkActionNC {}

/// Amount of a CCA request actually supplied by the application.
#[derive(Clone, Copy, PartialEq, Debug, Eq, Hash, Serialize)]
pub enum AppSendFraction {
    Zero,
    Half,
    Full,
}

impl AppSendFraction {
    fn as_real(self) -> RealNumRep {
        match self {
            Self::Zero => 0.into(),
            Self::Half => RealNumRep::new(1, 2),
            Self::Full => 1.into(),
        }
    }
}

impl Display for AppSendFraction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Zero => "0",
                Self::Half => "1/2",
                Self::Full => "1",
            }
        )
    }
}

/// How the simulator picks the next service among those feasible for the
/// true (C, B) under `sim_qe`.  Losses are always the smallest feasible ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimPolicy {
    /// Largest feasible service: the link never withholds a burst.
    Max,
    /// Smallest feasible service: the link withholds as much as it may.
    Min,
    /// `min + u * (max - min)` for a caller-supplied `u` in [0, 1].
    Random,
}

#[derive(Clone, Debug)]
pub struct NetworkModelNC {
    sim_ideal: Option<bool>,
    sim_c: Option<RealNumRep>,
    sim_b: Option<RealNumRep>,
    app_send_choices: Vec<AppSendFraction>,
    qe: Arc<dyn QeTables>,
    /// QE evaluator of the simulated ground truth.  The CCA may change `qe`
    /// (e.g. its assumed burstiness K) without changing the simulated link.
    sim_qe: Option<Arc<dyn QeTables>>,
}

impl NetworkModel for NetworkModelNC {
    type O = ObservationNC;
    type NA = NetworkActionNC;

    fn compute_feasible_network_moves(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
    ) -> Vec<Self::NA> {
        let paths = self.get_discretized_paths(relevant_history, move_cca);
        let mut ret = Vec::new();
        assert!(HISTORY_SIZE == FEASIBLE_SIZE + 1);
        for path in paths {
            let (c, b) = path;
            //print!("{:?}\n", path);
            let pos = 1 + relevant_history.len() - HISTORY_SIZE;
            let feasible_moves = self.compute_feasible_network_moves_given_path(
                &relevant_history[pos..],
                move_cca,
                c,
                b,
            );
            for move_ in feasible_moves {
                for app_send_fraction in &self.app_send_choices {
                    let mut move_ = move_.clone();
                    move_.app_send_fraction = *app_send_fraction;
                    ret.push(move_);
                }
            }
        }

        ret
    }

    fn compute_min_c_b_sum(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
    ) -> Option<RealNumRep> {
        let mut min_sum: RealNumRep = LARGEST_BW.into();
        for (c, b) in self.get_discretized_paths(relevant_history, move_cca) {
            min_sum = std::cmp::min(min_sum, c + b);
        }
        Some(std::cmp::max(min_sum, ALPHA.into()))
    }

    fn compute_max_allowed_rate(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
        loss_tolerance_abs: RealNumRep,
        delay_tolerance_frac: RealNumRep,
        measurement_interval: u32,
    ) -> Option<RealNumRep> {
        //unimplemented!("Need to re-implement max allowed rate computation");
        assert!(*move_cca == None);
        let mut max_rate: RealNumRep = LARGEST_BW.into();
        let qdels = self.compute_queue_delay(relevant_history, move_cca);
        if qdels.is_empty() {
            // Nothing bounds the loss of any rate: allow none.
            warn!("max allowed rate: no (C, B) path for {:?}", relevant_history.iter().map(|o| (o.a, o.s)).collect::<Vec<_>>());
            return Some(ALPHA.into());
        }
        if std::env::var("CCA_DEBUG_RATE").is_ok() {
            warn!("max allowed rate paths: {:?}", qdels.iter().map(|(c, b, _, q)| format!("c={} b={} q={}", c, b, q)).collect::<Vec<_>>());
        }
        for (c, b, _, max_q) in qdels {
            assert!(max_q <= b);
            max_rate = std::cmp::min(max_rate, c + b - max_q + loss_tolerance_abs);
        }
        Some(std::cmp::max(max_rate, ALPHA.into()))

        /*
        assert!(*move_cca == None);
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(&relevant_history[1..], &Some(CCAAction{rate: 0.into()}));
        let tsteps = a.len() as i32;
        let n_losses_observed = l.len() as i32;

        let mut max_rate: RealNumRep = LARGEST_BW.into();
        for (c, b) in self.get_discretized_paths(relevant_history, move_cca) {
            let q_interval: IntervalList<RealNumRep> = if n_losses_observed == tsteps - 1 && a[tsteps as usize - 1] == a[tsteps as usize - 2] {
                IntervalList::interval_point(Bound::Included(0.into()))
            } else {
                COMPUTE_Q_5[&(tsteps, n_losses_observed)](&a, &l, &s, l0, c, b)
            };
            assert!(!q_interval.is_empty());
            assert!(my_max(&q_interval) <= b);
            //max_q = std::cmp::min(my_max(&q_interval), b);
            max_rate = std::cmp::min(max_rate, c + b - my_max(&q_interval) + loss_tolerance_abs);
        }
        Some(std::cmp::max(max_rate, ALPHA.into()))
        */
        // let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
        // assert_eq!(s.len(), a.len());
        // let tsteps = a.len() as i32 + 1;
        // let n_losses_observed = l.len() as i32;

        // assert_eq!(measurement_interval, 1);
        // assert_eq!(delay_tolerance_frac, 3.into());

        // let a_interval =
        //     COMPUTE_A_4_3[&(tsteps, n_losses_observed)](&a, &l, &s, loss_tolerance_abs);
        // let max_rate = my_min(&a_interval) - *a.last().unwrap();
        // info!(
        //     "a_interval={:?}, a.last={:?}, rate={:?}",
        //     a_interval,
        //     a.last().unwrap(),
        //     max_rate
        // );

        // // In the QE query, we wrote an existential quantifier instead of
        // // universal quantifier, as a result, this is the set of bad (high) A_4
        // // values. We have to take complement and then take the max.
        // // Equivalently, we just return the min.

        // // Since this function is supposed to return rate (not sequence
        // // number), we do the subtraction.
        // Some(max_rate)
    }

    fn try_compute_belief_bounds(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
    ) -> Option<BeliefBounds> {
        qe_tables::quietly_catch(|| self.compute_belief_bounds(relevant_history, move_cca))
    }

    fn compute_belief_bounds(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
    ) -> BeliefBounds {
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
        let tsteps = a.len() as i32;
        let n_losses_observed = l.len() as i32;

        let (c_interval, pieces) =
            self.compute_c_exact(relevant_history, tsteps, n_losses_observed, &a, &l, &s, l0);
        let c_interval = c_interval.intersection(&bw_range());
        assert!(!c_interval.is_empty(), "no C explains the window");
        // B's bounds are its extremes over every feasible C, from the exact
        // pieces: B is linear in C on each.
        let pieces = pieces.unwrap_or_else(|| panic!("no B result for T={} with {} losses", tsteps, n_losses_observed));
        let (min_b, max_b) = exact::b_range(&pieces, &c_interval, SMALLEST_BW.into(), LARGEST_BW.into())
            .expect("no feasible B for any feasible C");

        // Deprecated, we can always just have some dummy history in the
        // beginning so that all traces are long enough. TODO: Add smaller T as
        // well (or we can just start with history of all 0s), this actually
        // sounds easier to try out. TODO: Rename to q_last in the hashmap For
        // loss need to keep L_idx
        //let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(&relevant_history[1..], &Some(CCAAction{rate: 0.into()}));
        //let tsteps = a.len() as i32;
        //let n_losses_observed = l.len() as i32;

        //let mut min_q: RealNumRep = RealNumInt::max_value().into();
        //let mut max_q: RealNumRep = 0.into();
        // TODO: better way to do this? ^^^
        // TODO: avoid doing C, B multiple times here...
        //for (c, b) in self.get_discretized_paths(relevant_history, move_cca) {
        //    println!("{:?},{:?}!", c, b);
        //    let q_interval: IntervalList<RealNumRep> = if n_losses_observed == tsteps - 1 && a[tsteps as usize - 1] == a[tsteps as usize - 2] {
        //        IntervalList::interval_point(Bound::Included(0.into()))
        //    } else {
        //        COMPUTE_Q_5[&(tsteps, n_losses_observed)](&a, &l, &s, l0, c, b)
        //    };
        //COMPUTE_Q_5[&(tsteps, n_losses_observed)](&a, &l, &s, l0, c, b);
        //assert!(!q_interval.is_empty());
        //if n_losses_observed == tsteps {
        // In this case, due to our simplification process in QE, we remove
        // the Q=0 constraint.

        // TODO: Verify if this affects other QE outputs semantically.
        //q_interval = IntervalList::interval_point(Bound::Included(0.into()));
        //}
        //    assert!(!q_interval.is_empty());
        //    min_q = std::cmp::min(min_q, my_min(&q_interval));
        //    max_q = std::cmp::max(max_q, my_max(&q_interval));
        //}
        let mut min_q: RealNumRep = RealNumInt::max_value().into();
        let mut max_q: RealNumRep = 0.into();
        let qdels = self.compute_queue_delay(relevant_history, move_cca);
        for (_, _, local_min_q, local_max_q) in qdels {
            min_q = std::cmp::min(min_q, local_min_q);
            max_q = std::cmp::max(max_q, local_max_q);
        }
        // let l_interval = if n_losses_observed as usize == FEASIBLE_SIZE {
        //     IntervalList::interval_bounded(l[4], l[4])
        // } else {
        //     COMPUTE_L_4[&(tsteps, n_losses_observed)](&a, &l, &s)
        // };
        // assert!(!l_interval.is_empty());
        // let a_minus_s = a.last().unwrap() - s.last().unwrap();
        let last_loss = match l.last() {
            Some(lv) => *lv,
            None => l0,
        };
        let inflight = *a.last().unwrap() - *s.last().unwrap() - last_loss;
        // TODO: Better inflight estimate. Due to the knowledge of max C, we
        // may be able to compute losses even before we can detect them. So due
        // to beliefs we may have tighter bound on L.

        BeliefBounds {
            min_c: outer_min(&c_interval),
            max_c: outer_max(&c_interval),
            min_b,
            max_b,
            min_q,
            max_q,
            // min_if: a_minus_s - my_max(&l_interval),
            // max_if: a_minus_s - my_min(&l_interval),
            min_if: inflight,
            max_if: inflight,
            min_cb: exact::min_c_plus_b(&pieces, &c_interval, SMALLEST_BW.into(), LARGEST_BW.into()).unwrap_or(0.into()),
        }
    }

    fn joint_c_b_bounds(
        &self,
        windows: &[(&[Self::O], &Option<CCAAction>)],
        c_limit: Option<(RealNumRep, RealNumRep)>,
    ) -> Option<Option<BeliefBounds>> {
        let pieces = self.joint_pieces(windows)?;
        let mut c = exact::c_set(&pieces);
        if let Some((lo, hi)) = c_limit {
            if lo > hi {
                return Some(None);
            }
            c = c.intersection(&IntervalList::interval_bounded(Bound::Included(lo), Bound::Included(hi)));
        }
        if c.is_empty() {
            return Some(None);
        }
        let (min_b, max_b) = exact::b_range(&pieces, &c, SMALLEST_BW.into(), LARGEST_BW.into())?;
        let zero: RealNumRep = 0.into();
        Some(Some(BeliefBounds {
            min_c: outer_min(&c),
            max_c: outer_max(&c),
            min_b,
            max_b,
            min_q: zero,
            max_q: zero,
            min_if: zero,
            max_if: zero,
            min_cb: exact::min_c_plus_b(&pieces, &c, SMALLEST_BW.into(), LARGEST_BW.into()).unwrap_or(zero),
        }))
    }

    fn chained_c_bounds(&self, history: &[Self::O]) -> Option<(RealNumRep, RealNumRep)> {
        // As `get_observation_vectors`, but a speculative history can report
        // a loss without the ones before it within this slice.
        let a: Vec<RealNumRep> = history[..history.len() - 1].iter().map(|o| o.a).collect();
        let s: Vec<RealNumRep> = history[1..].iter().map(|o| o.s).collect();
        let mut losses = vec![];
        for (i, obs) in history.iter().enumerate().skip(1) {
            for lo in &obs.lo {
                if lo.t <= i - 1 {
                    losses.push((i - 1 - lo.t, lo.l));
                }
            }
        }
        let k = self.qe.k_values().into_iter().max().unwrap();
        let (lo, hi) = chain::c_bounds(&a, &losses, &s, k);
        Some((exact::round_down(lo), hi.map_or(LARGEST_BW.into(), exact::round_up)))
    }

    fn fresh_belief_bounds(&self, history: &[Self::O]) -> Option<BeliefBounds> {
        let (lo, hi) = self.chained_c_bounds(history)?;
        // Everything not yet served or reported lost may still be queued.
        let last = history.last()?;
        let in_flight = std::cmp::max(last.a - last.s - last.get_ld(), 0.into());
        Some(BeliefBounds {
            min_c: std::cmp::max(lo, SMALLEST_BW.into()),
            max_c: std::cmp::min(hi, LARGEST_BW.into()),
            min_b: SMALLEST_BW.into(),
            max_b: LARGEST_BW.into(),
            min_q: 0.into(),
            max_q: in_flight,
            min_if: in_flight,
            max_if: in_flight,
            min_cb: 0.into(),
        })
    }

    fn compute_queue_delay(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
    ) -> Vec<(RealNumRep, RealNumRep, RealNumRep, RealNumRep)> {
        let paths = self.get_discretized_paths(relevant_history, move_cca);
        let all_paths = paths.clone();
        assert!(HISTORY_SIZE == FEASIBLE_SIZE + 1);
        let mut new_history = vec![];
        for i in relevant_history {
            new_history.push(i.clone());
        }
        let dummy = &Some(CCAAction { rate: 0.into() });

        //let mut min_q: RealNumRep = RealNumInt::max_value().into();
        //let mut max_q: RealNumRep = 0.into();
        let mut ret = vec![];
        for path in paths {
            let (c, b) = path;
            //print!("{:?}\n", path);
            let pos = if relevant_history.len() < HISTORY_SIZE {
                1
            } else {
                1 + relevant_history.len() - HISTORY_SIZE
            };
            let feasible_moves = self.compute_feasible_network_moves_given_path(
                &relevant_history[pos..],
                &dummy,
                c,
                b,
            );
            for feasible_move in feasible_moves {
                new_history.push(self.compute_observation(
                    new_history.last().unwrap(),
                    &feasible_move,
                    &dummy,
                ));
                let (a, l, s, l0) =
                    NetworkModelNC::get_observation_vectors(&new_history[pos..], &None);
                let tsteps = a.len() as i32;
                let n_losses_observed = l.len() as i32;

                let mut q_interval = expect_qe("Q", tsteps, n_losses_observed, infeasible_is_empty(|| self.qe.q(tsteps, n_losses_observed, &a, &l, &s, l0, c, b)));
                if q_interval.is_empty() {
                    // No K fits this (C, B) and move; it is not a feasible path.
                    new_history.pop();
                    continue;
                }
                if n_losses_observed == tsteps {
                    // In this case, due to our simplification process in QE, we remove
                    // the Q=0 constraint.

                    // TODO: Verify if this affects other QE outputs semantically.
                    q_interval = IntervalList::interval_point(Bound::Included(0.into()));
                }
                assert!(!q_interval.is_empty());
                ret.push((c, b, my_min(&q_interval), my_max(&q_interval)));
                //min_q = std::cmp::min(min_q, my_min(&q_interval));
                //max_q = std::cmp::max(max_q, my_max(&q_interval));
                new_history.pop();
            }
        }
        if ret.is_empty() {
            // No sampled path had a feasible next move (possible under a
            // union of K).  The queue is at most what is in flight: sent,
            // not delivered, not reported lost.
            let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
            let inflight = *a.last().unwrap() - *s.last().unwrap() - l.last().copied().unwrap_or(l0);
            warn!("no Q result on any sampled path; bounding the queue by in-flight data {}", inflight);
            for (c, b) in all_paths {
                ret.push((c, b, 0.into(), std::cmp::min(b, std::cmp::max(inflight, 0.into()))));
            }
        }
        ret
    }

    fn get_network_move_sim(
        &self,
        relevant_history: &[Self::O],
        move_cca: &Option<CCAAction>,
        sim_ideal: bool,
    ) -> Option<Self::NA> {
        assert_eq!(self.sim_ideal.unwrap(), sim_ideal);
        assert!(!sim_ideal);
        self.get_network_move_sim_policy(relevant_history, move_cca, SimPolicy::Max, 0.into())
    }

    fn setup_network_move_sim(&mut self, sim_ideal: bool, sim_c: RealNumRep, sim_b: RealNumRep) {
        self.sim_ideal = Some(sim_ideal);
        assert!(!sim_ideal);
        self.sim_c = Some(sim_c);
        self.sim_b = Some(sim_b);
    }

    fn init(&mut self) {}

    fn get_initial_history(&self) -> Vec<Self::O> {
        let mut ret = Vec::new();
        for t in 0..HISTORY_SIZE {
            // Since I started adding C >= 5, don't need to add history that
            // tries to increase C.

            // let obs = ObservationNC {
            //     a: (10 * (t as i32 + 1)).into(),
            //     s: (10 * t as i32).into(),
            //     lo: vec![LossObservation { t: 1, l: 0.into() }],
            // };
            let obs = ObservationNC {
                a: 0.into(),
                s: 0.into(),
                lo: vec![LossObservation { t: 0, l: 0.into() }],
            };
            ret.push(obs);
        }
        return ret;
        let mut obs = vec![];
        obs.push(ObservationNC {
            a: 60.into(),
            s: 42.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 60.into(),
            s: 60.into(),
            lo: vec![
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 80.into(),
            s: 60.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 80.into(),
            s: 80.into(),
            lo: vec![
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 102.into(),
            s: 80.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 102.into(),
            s: 100.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });

        //return obs;
        let mut obs = vec![];
        obs.push(ObservationNC {
            a: 230.into(),
            s: 200.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 230.into(),
            s: 220.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 230.into(),
            s: 230.into(),
            lo: vec![
                LossObservation {
                    t: 2,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 230.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 250.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 260.into(),
            lo: vec![
                LossObservation {
                    t: 2,
                    l: 2.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 1,
                    l: 2.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 2.into(), // it is in discretization unit of packet
                },
            ],
        });
        return obs;
        let mut obs = vec![];
        obs.push(ObservationNC {
            a: 230.into(),
            s: 230.into(),
            lo: vec![
                LossObservation {
                    t: 2,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 230.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 250.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 262.into(),
            s: 260.into(),
            lo: vec![
                LossObservation {
                    t: 2,
                    l: 2.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 1,
                    l: 2.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 2.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 294.into(),
            s: 260.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 2.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 306.into(),
            s: 280.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 2.into(), // it is in discretization unit of packet
            }],
        });
        obs
    }

    fn compute_observation(
        &self,
        last: &Self::O,
        na: &Self::NA,
        move_cca: &Option<CCAAction>,
    ) -> Self::O {
        ObservationNC {
            a: last.a + move_cca.as_ref().unwrap().rate * na.app_send_fraction.as_real(),
            s: na.s,
            lo: na.lo.clone(),
        }
    }
}

impl Default for NetworkModelNC {
    fn default() -> Self {
        Self::new()
    }
}

/// Index-style lookup with a message naming the unsolved QE query.
fn expect_qe(
    quantity: &str,
    tsteps: i32,
    n_losses_observed: i32,
    bounds: Option<IntervalList<RealNumRep>>,
) -> IntervalList<RealNumRep> {
    bounds.unwrap_or_else(|| {
        panic!(
            "No QE result for {} at T={} with {} losses observed",
            quantity, tsteps, n_losses_observed
        )
    })
}

impl NetworkModelNC {
    pub fn new() -> Self {
        Self {
            sim_ideal: None,
            sim_c: None,
            sim_b: None,
            app_send_choices: vec![AppSendFraction::Full],
            qe: Arc::new(PlainQe),
            sim_qe: None,
        }
    }

    /// Use a different generated QE evaluator, e.g. `BurstyQe`.
    pub fn with_qe(self, qe: Arc<dyn QeTables>) -> Self {
        Self { qe, ..self }
    }

    /// Replace the evaluator the CCA's beliefs use, between searches.
    pub fn set_qe(&mut self, qe: Arc<dyn QeTables>) {
        self.qe = qe;
    }

    /// Simulate the ground truth with `sim_qe` instead of the CCA's `qe`.
    pub fn with_sim_qe(self, sim_qe: Arc<dyn QeTables>) -> Self {
        Self {
            sim_qe: Some(sim_qe),
            ..self
        }
    }

    fn sim_tables(&self) -> &dyn QeTables {
        self.sim_qe.as_deref().unwrap_or(&*self.qe)
    }

    /// The simulated link's response to `move_cca`, choosing the service by
    /// `policy` (`u` is only used by `SimPolicy::Random`).
    pub fn get_network_move_sim_policy(
        &self,
        relevant_history: &[ObservationNC],
        move_cca: &Option<CCAAction>,
        policy: SimPolicy,
        u: RealNumRep,
    ) -> Option<NetworkActionNC> {
        let sim_c = self.sim_c.unwrap();
        let sim_b = self.sim_b.unwrap();

        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
        let tsteps = a.len() as i32;
        let n_losses_observed = l.len() as i32;

        let s_interval = expect_qe(
            "S",
            tsteps,
            n_losses_observed,
            self.sim_tables()
                .s(tsteps, n_losses_observed, &a, &l, &s, l0, sim_c, sim_b),
        );
        assert!(s_interval.is_contiguous_interval());
        let min_s = my_min(&s_interval);
        let max_s = my_max(&s_interval);
        let next_s = match policy {
            SimPolicy::Max => max_s,
            SimPolicy::Min => min_s,
            // Round down to a 1/16 grid to keep rational denominators bounded.
            SimPolicy::Random => {
                min_s + ((max_s - min_s) * u * 16).floor() / 16
            }
        };

        let mut s_clone = s.clone();
        s_clone.push(next_s);

        // Compute loss observation
        let prior_loss = l.len() as i32;
        let lo_ret = self.get_sim_loss_observation(&a, &l, &s_clone, l0, prior_loss);
        let lo = if lo_ret.is_empty() {
            let time_ago = (tsteps - 1 - n_losses_observed) + 1;
            assert!(time_ago >= 0);
            vec![LossObservation {
                t: time_ago as usize,
                l: l.last().copied().unwrap_or(l0),
            }]
        } else {
            lo_ret
        };
        Some(NetworkActionNC {
            s: next_s,
            lo,
            app_send_fraction: AppSendFraction::Full,
        })
    }

    /// Whether some C and B explain `window` (a QE-sized window) under the
    /// current `qe`.  Unlike `compute_belief_bounds`, this never asserts, so
    /// it can test hypotheses such as a burstiness K: an infeasible window
    /// rules the hypothesis out.  Exact (`crate::exact`), so the true K is
    /// never ruled out by a missed narrow C range.
    pub fn window_feasible(&self, window: &[ObservationNC]) -> bool {
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(window, &None);
        let tsteps = a.len() as i32;
        let n_losses_observed = l.len() as i32;
        qe_tables::quietly_catch(|| {
            !self
                .compute_c_exact(window, tsteps, n_losses_observed, &a, &l, &s, l0)
                .0
                .intersection(&bw_range())
                .is_empty()
        })
        .unwrap_or(false)
    }

    /// The belief for `window` under the current `qe`, or `None` if the
    /// window is infeasible (see `window_feasible`).  Never panics.
    pub fn try_window_belief(&self, window: &[ObservationNC]) -> Option<BeliefBounds> {
        qe_tables::quietly_catch(|| self.compute_belief_bounds(window, &None))
    }

    /// The pieces of one window, clipped to every C result for it (e.g. the
    /// QE C query, when solved); empty if the window is infeasible.  `None`
    /// if the window has no B result.
    fn window_pieces(&self, window: &[ObservationNC], move_cca: &Option<CCAAction>) -> Option<Vec<exact::Piece>> {
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(window, move_cca);
        let (t, n) = (a.len() as i32, l.len() as i32);
        let Some((c, pieces)) = qe_tables::quietly_catch(|| self.compute_c_exact(window, t, n, &a, &l, &s, l0)) else {
            return Some(vec![]);
        };
        let c = c.intersection(&bw_range());
        Some(exact::intersect(&pieces?, &exact::from_c_set(&c)))
    }

    /// `window_pieces` intersected over `windows` (`exact::intersect`).  A
    /// history is folded one window at a time, so every prefix is cached:
    /// the committed history is folded once, and the minimax tree only
    /// extends it by its speculative windows.
    fn joint_pieces(&self, windows: &[(&[ObservationNC], &Option<CCAAction>)]) -> Option<Vec<exact::Piece>> {
        use std::hash::{Hash, Hasher};
        let qe = format!("{:?}", self.qe);
        // Chained hashes identify each prefix; the key also holds the newest
        // window, so a collision must also match it.
        let mut keys = Vec::with_capacity(windows.len());
        let mut h = 0u64;
        for (w, action) in windows {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            (h, &qe).hash(&mut hasher);
            for o in w.iter() {
                o.hash(&mut hasher);
            }
            action.map(|x| x.rate).hash(&mut hasher);
            h = hasher.finish();
            keys.push((qe.clone(), h, w.to_vec(), action.map(|x| x.rate)));
        }
        let mut start = 0;
        let mut acc: Option<Vec<exact::Piece>> = None;
        JOINT_CACHE.with(|c| {
            let c = c.borrow();
            for i in (0..windows.len()).rev() {
                if let Some(hit) = c.get(&keys[i]) {
                    acc = Some(hit.clone());
                    start = i + 1;
                    break;
                }
            }
        });
        for i in start..windows.len() {
            let next = self.window_pieces(windows[i].0, windows[i].1)?;
            let joined = match acc {
                None => next,
                Some(prev) => exact::intersect(&prev, &next),
            };
            JOINT_CACHE.with(|c| {
                let mut c = c.borrow_mut();
                if c.len() >= PIECES_CACHE_SIZE {
                    c.clear();
                }
                c.insert(keys[i].clone(), joined.clone());
            });
            acc = Some(joined);
        }
        acc
    }

    /// B's feasible set at a concrete C.  The generated function asserts
    /// the clauses that do not mention B; a C that fails one has no B.  So
    /// does `None`: for a union of K it means every K's assertion failed (a
    /// missing table would already have failed in `exact_pieces`).
    #[allow(clippy::too_many_arguments)]
    fn b_at(
        &self,
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
    ) -> IntervalList<RealNumRep> {
        match qe_tables::quietly_catch(|| self.qe.b(tsteps, n_losses_observed, a, l, s, l0, c)) {
            Some(Some(b)) => b,
            Some(None) | None => IntervalList::new(vec![]),
        }
    }

    /// The exact projection of this window's feasible (C, B) set onto C,
    /// as pieces with B linear in C (`crate::exact`), over C in the
    /// bandwidth range.  `None` if no B result exists for this key.
    #[allow(clippy::too_many_arguments)]
    fn exact_pieces(
        &self,
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> Option<Vec<exact::Piece>> {
        let key = PiecesKey {
            qe: format!("{:?}", self.qe),
            t: tsteps,
            n: n_losses_observed,
            obs: [a, l, s, &[l0]].concat(),
        };
        if let Some(hit) = PIECES_CACHE.with(|c| c.borrow().get(&key).cloned()) {
            return hit;
        }
        let ret = self.exact_pieces_uncached(tsteps, n_losses_observed, a, l, s, l0);
        PIECES_CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() >= PIECES_CACHE_SIZE {
                c.clear();
            }
            c.insert(key, ret.clone());
        });
        ret
    }

    #[allow(clippy::too_many_arguments)]
    fn exact_pieces_uncached(
        &self,
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> Option<Vec<exact::Piece>> {
        let missing = std::cell::Cell::new(false);
        let pieces = exact::pieces(
            |c| match self.qe.b_lin(tsteps, n_losses_observed, a, l, s, l0, c) {
                Some(b) => b.map(|b| {
                    b.intersection(&IntervalList::interval_bounded(
                        Bound::Included(Lin::from(RealNumRep::from(SMALLEST_BW))),
                        Bound::Included(Lin::from(RealNumRep::from(LARGEST_BW))),
                    ))
                }),
                None => {
                    missing.set(true);
                    None
                }
            },
            ds::lin::widen(SMALLEST_BW.into()),
            ds::lin::widen(LARGEST_BW.into()),
        );
        (!missing.get()).then_some(pieces)
    }

    /// Bounds on C for the window `relevant_history` (whose observation
    /// vectors the caller already computed as `a`, `l`, `s`, `l0`).
    ///
    /// Uses the QE result for the whole window when one was solved, which is
    /// always the case for the plain model.  Otherwise the C query for this
    /// window did not finish, so intersect every constraint on C that did:
    ///
    /// 1. C from shorter sub-windows (`c_from_sub_windows`).
    /// 2. The C-only clauses of this window's B query (`c_from_b`).
    /// 3. C is feasible for this window iff its B set is non-empty: the
    ///    exact projection (`exact_pieces`) eliminates B, which is what the
    ///    unfinished QE query would have done for every observation.  This
    ///    step also applies when the C query was solved.
    #[allow(clippy::too_many_arguments)]
    fn compute_c(
        &self,
        relevant_history: &[ObservationNC],
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> IntervalList<RealNumRep> {
        self.compute_c_exact(relevant_history, tsteps, n_losses_observed, a, l, s, l0).0
    }

    /// `compute_c`, and the exact pieces it used (`None` if this key has no
    /// B result).
    #[allow(clippy::too_many_arguments)]
    fn compute_c_exact(
        &self,
        relevant_history: &[ObservationNC],
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> (IntervalList<RealNumRep>, Option<Vec<exact::Piece>>) {
        let pieces = self.exact_pieces(tsteps, n_losses_observed, a, l, s, l0);
        let mut c = match self.qe.c(tsteps, n_losses_observed, a, l, s, l0) {
            Some(c) => c,
            None if pieces.is_some() => self
                .c_from_sub_windows(relevant_history, tsteps)
                .unwrap_or_else(|| IntervalList::new_interval(Interval::new(Bound::Unbounded, Bound::Unbounded))),
            None => self.c_from_partial_results(relevant_history, tsteps, n_losses_observed, a, l, s, l0).0,
        };
        if let Some(pieces) = &pieces {
            c = c.intersection(&exact::c_set(pieces));
        }
        (c, pieces)
    }

    #[allow(clippy::too_many_arguments)]
    fn c_from_partial_results(
        &self,
        relevant_history: &[ObservationNC],
        tsteps: i32,
        n_losses_observed: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> (IntervalList<RealNumRep>, bool) {
        let sub = self.c_from_sub_windows(relevant_history, tsteps);
        let from_b = self.qe.c_from_b(tsteps, n_losses_observed, a, l, s, l0);
        let mut c = IntervalList::interval_bounded(
            Bound::Included(0.into()),
            Bound::Included(LARGEST_BW.into()),
        );
        match (&sub, &from_b) {
            (None, None) => panic!(
                "No QE result constrains C at T={} with {} losses observed",
                tsteps, n_losses_observed
            ),
            (Some(sub), None) => c = c.intersection(sub),
            (None, Some(from_b)) => c = c.intersection(from_b),
            (Some(sub), Some(from_b)) => {
                let both = c.intersection(sub).intersection(from_b);
                c = if both.is_empty() {
                    // Discretized QE results can disagree slightly.  Values
                    // outside `from_b` would trip `b`'s assertions, so it wins.
                    debug!("C from sub-windows {:?} disagrees with C from B {:?}", sub, from_b);
                    c.intersection(from_b)
                } else {
                    both
                };
            }
        }
        (c, from_b.is_some())
    }

    /// Intersect C over shorter sub-windows of `relevant_history`, largest
    /// trace length first, using every sub-window whose loss count was
    /// solved and that is not inside a sub-window already used.  C is a
    /// persistent parameter, so each sub-window's bound is sound on its own.
    /// Sub-windows ignore the outstanding CCA action; dropping the newest
    /// arrival only loosens a bound.
    fn c_from_sub_windows(
        &self,
        relevant_history: &[ObservationNC],
        tsteps: i32,
    ) -> Option<IntervalList<RealNumRep>> {
        // (first, last) observation index of every sub-window used so far.
        let mut used: Vec<(usize, usize)> = vec![];
        let mut ret: Option<IntervalList<RealNumRep>> = None;
        for sub_t in (1..tsteps).rev() {
            let n_obs = sub_t as usize + 1;
            if n_obs > relevant_history.len() {
                continue;
            }
            for first in 0..=(relevant_history.len() - n_obs) {
                let last = first + n_obs - 1;
                if used.iter().any(|&(f, l)| f <= first && last <= l) {
                    continue;
                }
                let window = &relevant_history[first..=last];
                let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(window, &None);
                let Some(c) = self.qe.c(sub_t, l.len() as i32, &a, &l, &s, l0) else {
                    continue;
                };
                trace!(
                    "C fallback: T={} window [{}, {}] losses={} gives {:?}",
                    sub_t,
                    first,
                    last,
                    l.len(),
                    c
                );
                used.push((first, last));
                ret = Some(match ret {
                    None => c,
                    Some(prev) => {
                        let both = prev.intersection(&c);
                        if both.is_empty() {
                            // Generated QE is discretized, so independent
                            // windows can disagree slightly; keep the bound
                            // from the larger, earlier-chosen windows.
                            debug!(
                                "C fallback: ignoring disagreeing T={} window [{}, {}]",
                                sub_t, first, last
                            );
                            prev
                        } else {
                            both
                        }
                    }
                });
            }
        }
        ret
    }

    pub fn new_with_app_send_choices(choices: &[AppSendFraction]) -> Self {
        assert!(!choices.is_empty());
        Self {
            app_send_choices: choices.to_vec(),
            ..Self::new()
        }
    }

    fn discretize_interval(
        interval_list: &IntervalList<RealNumRep>,
        n_points: usize,
    ) -> Vec<RealNumRep> {
        //assert!(interval_list.is_contiguous_interval());
        let mut vals = Vec::new();
        // Get the total length
        // Could there be overlapping?
        let mut length: RealNumRep = 0.into();
        for interval in interval_list.get_intervals() {
            let singleton: IntervalList<RealNumRep> = IntervalList::new_interval(interval.clone());
            let min_v = my_min(&singleton);
            let max_v = my_max(&singleton);
            length = length + (max_v - min_v);
        }

        for interval in interval_list.get_intervals() {
            let singleton: IntervalList<RealNumRep> = IntervalList::new_interval(interval.clone());
            let min_v = my_min(&singleton);
            let max_v = my_max(&singleton);
            if min_v == max_v {
                vals.push(min_v);
            } else {
                let n_points: RealNumRep = ((n_points) as RealNumInt).into();
                let local_points: RealNumRep = (n_points * (max_v - min_v) / length).ceil();
                assert!(local_points.is_integer());
                if local_points == 1.into() {
                    vals.push(min_v);
                } else {
                    let den: RealNumRep = local_points - 1;
                    let chunk: RealNumRep = (max_v - min_v) / den;
                    for i in 0..local_points.to_integer() {
                        let this_n_chunks: RealNumRep = (i as RealNumInt).into();
                        let this_v: RealNumRep = min_v + chunk * this_n_chunks;
                        vals.push(this_v);
                    }
                }
            }
        }
        vals
    }

    fn get_discretized_paths(
        &self,
        relevant_history: &[ObservationNC],
        move_cca: &Option<CCAAction>,
    ) -> Vec<(RealNumRep, RealNumRep)> {
        //return self.get_discretized_paths_original(&relevant_history[relevant_history.len() - FEASIBLE_SIZE..relevant_history.len()], move_cca);
        let mut c_interval_all = IntervalList::interval_bounded(
            Bound::Included(SMALLEST_BW.into()),
            Bound::Included(LARGEST_BW.into()),
        );
        // 0 corresponding to the real trace
        assert!(HISTORY_SIZE == FEASIBLE_SIZE + 1);
        if relevant_history.len() < HISTORY_SIZE {
            return self.get_discretized_paths_original(&relevant_history, move_cca);
        }
        for ptr in [0, relevant_history.len() - HISTORY_SIZE] {
            let rh = &relevant_history[ptr..(ptr + HISTORY_SIZE)];
            // TODO: Can cache this or the whole function
            let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(rh, move_cca);
            // a.len() may not be equal to s.len() since we use this in belief
            // computation as well as network move computation.
            let tsteps = s.len() as i32;
            let n_losses_observed = l.len() as i32;

            let c_interval = self.compute_c(rh, tsteps, n_losses_observed, &a, &l, &s, l0);
            c_interval_all = c_interval_all.intersection(&c_interval);
            //assert!(!c_interval_all.is_empty());
            if c_interval_all.is_empty() {
                return self.get_discretized_paths_original(
                    &relevant_history
                        [relevant_history.len() - HISTORY_SIZE..relevant_history.len()],
                    move_cca,
                );
            }
        }
        let mut c_vals = NetworkModelNC::discretize_interval(&c_interval_all, N_DISCRETE_POINTS);
        // The ends of the exact C set can be infeasible (open ends, or
        // rounded outward); the lowest C is the worst case for loss, so also
        // take a C inside the lowest and highest pieces both windows allow.
        let first = &relevant_history[..HISTORY_SIZE];
        let last = &relevant_history[relevant_history.len() - HISTORY_SIZE..];
        // Without the outstanding action (T is the service count here, and
        // dropping an arrival only loosens C; `b_at` drops infeasible C).
        if let Some(joint) = self.joint_pieces(&[(first, &None), (last, &None)]) {
            let inside: Vec<RealNumRep> = exact::representatives(&joint)
                .into_iter()
                .filter(|&v| !c_interval_all.intersection(&IntervalList::interval_point(Bound::Included(v))).is_empty())
                .collect();
            c_vals.extend(inside.first().into_iter().chain(inside.last()).copied());
            c_vals.sort();
            c_vals.dedup();
        }

        let mut ret = Vec::new();
        for c_val in c_vals.clone() {
            let mut b_interval_all = IntervalList::interval_bounded(
                Bound::Included(SMALLEST_BW.into()),
                Bound::Included(LARGEST_BW.into()),
            );
            for ptr in [0, relevant_history.len() - HISTORY_SIZE] {
                let rh = &relevant_history[ptr..(ptr + HISTORY_SIZE)];
                let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(rh, move_cca);
                let tsteps = s.len() as i32;
                let n_losses_observed = l.len() as i32;

                let b_interval = self.b_at(tsteps, n_losses_observed, &a, &l, &s, l0, c_val);
                //print!("{:?}\n",b_interval);
                let old_b = b_interval_all.clone();
                b_interval_all = b_interval_all.intersection(&b_interval);
                /*if b_interval_all.is_empty() {
                    print!("{:?}\n", c_val);
                    print!("{:?}\n", c_interval_all);
                    print!("{:?}\n", c_vals.clone());
                    print!("{:?}\n", old_b);
                    print!("{:?}\n", b_interval);
                }*/

                //assert!(!b_interval_all.is_empty());
            }
            if !b_interval_all.is_empty() {
                ret.push((c_val, my_min(&b_interval_all)));
                ret.push((c_val, my_max(&b_interval_all)));
            }
        }
        //assert!(!ret.is_empty());
        if ret.is_empty() {
            return self.get_discretized_paths_original(
                &relevant_history[relevant_history.len() - HISTORY_SIZE..relevant_history.len()],
                move_cca,
            );
        }
        ret
    }

    fn get_discretized_paths_original(
        &self,
        relevant_history: &[ObservationNC],
        move_cca: &Option<CCAAction>,
    ) -> Vec<(RealNumRep, RealNumRep)> {
        // TODO: Can cache this or the whole function
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
        // a.len() may not be equal to s.len() since we use this in belief
        // computation as well as network move computation.
        let tsteps = s.len() as i32;
        let n_losses_observed = l.len() as i32;

        assert!(FEASIBLE_SIZE >= tsteps as usize);
        let (c_interval, pieces) =
            self.compute_c_exact(relevant_history, tsteps, n_losses_observed, &a, &l, &s, l0);
        let c_interval = c_interval.intersection(&IntervalList::interval_bounded(
            Bound::Included(0.into()),
            Bound::Included(LARGEST_BW.into()),
        ));
        if c_interval.is_empty() {
            // No link explains this (speculative) history.
            return vec![];
        }

        // TODO: Adapt N_DISCRETE_POINTS based on how close min_c and max_c are
        // (when both are divided by ALPHA).
        let mut c_vals = NetworkModelNC::discretize_interval(&c_interval, N_DISCRETE_POINTS);
        // The ends of the exact C set can be open (infeasible) ends of a
        // piece; a point inside the lowest and highest pieces is feasible.
        if let Some(pieces) = &pieces {
            let inside: Vec<RealNumRep> = exact::representatives(pieces)
                .into_iter()
                .filter(|&v| !c_interval.intersection(&IntervalList::interval_point(Bound::Included(v))).is_empty())
                .collect();
            c_vals.extend(inside.first().into_iter().chain(inside.last()).copied());
            c_vals.sort();
            c_vals.dedup();
        }

        let mut ret = Vec::new();
        for c_val in c_vals {
            let mut b_interval = self.b_at(tsteps, n_losses_observed, &a, &l, &s, l0, c_val);
            b_interval = b_interval.intersection(&IntervalList::interval_bounded(
                Bound::Included(0.into()),
                Bound::Included(LARGEST_BW.into()),
            ));
            // Empty when no static link explains the window (it straddles a
            // change in the link); the planner then avoids the search.
            if b_interval.is_empty() {
                continue;
            }
            ret.push((c_val, my_min(&b_interval)));
            ret.push((c_val, my_max(&b_interval)));
        }
        ret
    }

    fn compute_feasible_network_moves_given_path(
        &self,
        relevant_history: &[ObservationNC],
        move_cca: &Option<CCAAction>,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Vec<NetworkActionNC> {
        let (a, l, mut s, l0) = NetworkModelNC::get_observation_vectors(relevant_history, move_cca);
        assert_eq!(s.len(), a.len() - 1);
        let tsteps = a.len() as i32;
        let n_losses_observed = l.len() as i32;

        let s_interval = expect_qe("S", tsteps, n_losses_observed, infeasible_is_empty(|| self.qe.s(tsteps, n_losses_observed, &a, &l, &s, l0, c, b)));
        // Under a union of K, S can be several intervals; the extremes are
        // in the first and the last.
        let ivs = s_interval.get_intervals();
        if ivs.is_empty() {
            // No K fits this (C, B): not a feasible path.
            return vec![];
        }
        let min_s = my_min(&IntervalList::new_interval(*ivs.first().unwrap()));
        let max_s = my_max(&IntervalList::new_interval(*ivs.last().unwrap()));

        let mut s_vals = Vec::new();
        s_vals.push(min_s);
        if min_s != max_s {
            s_vals.push(max_s);
        }

        // Heuristics, add lowest s such that no delay
        //let extra_discret = a.get(a.len() - 2).unwrap().clone();
        //if extra_discret > min_s && extra_discret < max_s {
        //    s_vals.push(extra_discret);
        //}

        // // TODO: Adapt N_DISCRETE_POINTS based on how close min_s and max_s are
        // // (when both are divided by ALPHA).
        // let mut s_vals = Vec::new();
        // if min_s == max_s {
        //     s_vals.push(min_s);
        // } else {
        //     let den: RealNumRep = ((N_DISCRETE_POINTS - 1) as i32).into();
        //     let chunk: RealNumRep = (max_s - min_s) / den;
        //     for i in 0..N_DISCRETE_POINTS {
        //         let this_n_chunks: RealNumRep = (i as RealNumInt).into();
        //         let this_s: RealNumRep = min_s + chunk * this_n_chunks;
        //         s_vals.push(this_s);
        //     }
        // }

        let mut ret = Vec::new();
        for s_val in s_vals {
            s.push(s_val);
            trace!("Pushing S[{}]={}", tsteps, s_val);
            let prior_loss = l.len() as i32;
            let los = self.compute_loss_observations(&a, &l, &s, l0, c, b, prior_loss);
            for lo in los {
                let this_lo = if lo.is_empty() {
                    let time_ago = (tsteps - 1 - n_losses_observed) + 1;
                    assert!(time_ago >= 0);
                    let last_loss = match l.last() {
                        Some(lv) => *lv,
                        None => l0,
                    };
                    vec![LossObservation {
                        t: time_ago as usize,
                        l: last_loss,
                    }]
                } else {
                    lo
                };
                ret.push(NetworkActionNC {
                    s: s_val,
                    lo: this_lo,
                    app_send_fraction: AppSendFraction::Full,
                });
            }
            trace!("Popping S[{}]", tsteps);
            s.pop();
        }

        trace!("Found {} feasible network actions", ret.len());
        ret
    }

    fn is_loss_observed(a: &RealNumRep, l: &RealNumRep, s: &RealNumRep) -> bool {
        // Just check if RTT elapsed.
        *s >= *a - *l
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_loss_observations(
        &self,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
        prior_loss: i32,
    ) -> Vec<Vec<LossObservation>> {
        // Get the next loss values
        let tsteps = a.len() as i32;
        assert_eq!(a.len(), s.len());
        let n_losses_observed = l.len() as i32;
        if n_losses_observed == tsteps {
            return vec![vec![]];
        }

        trace!("Computing losses after: L={:?}", l);
        let time_ago = (tsteps - 1 - n_losses_observed) as usize;
        let l_intervals = expect_qe(
            "L",
            tsteps,
            n_losses_observed,
            infeasible_is_empty(|| self.qe.l_obs(tsteps, prior_loss, n_losses_observed, a, l, s, l0, c, b)),
        );
        trace!("Got l_intervals={:?}", l_intervals);
        // No loss is consistent with this (C, B, service): under a union of
        // K, the service can come from a K that does not fit (C, B).  Such a
        // network move is infeasible, so it has no continuations.
        if l_intervals.is_empty() {
            return vec![];
        }

        // Check if these losses will be observed
        let l_min = my_min(&l_intervals);
        let l_max = my_max(&l_intervals);

        // For each observed loss get the next loss value
        let mut l_clone = l.to_vec();
        let mut ret = Vec::new();

        let mut process_l = |this_l: &RealNumRep| {
            // trace!("Checking if observed L[{}]={}", n_observed_losses, this_l);
            if NetworkModelNC::is_loss_observed(
                &a[n_losses_observed as usize],
                this_l,
                s.last().unwrap(),
            ) {
                let base_lo = vec![LossObservation {
                    t: time_ago,
                    l: *this_l,
                }];

                trace!("Pushing L[{}]={}", n_losses_observed, this_l);
                l_clone.push(*this_l);
                let recur_lo =
                    self.compute_loss_observations(a, &l_clone, s, l0, c, b, prior_loss);
                trace!("Popping L[{}]", n_losses_observed);
                l_clone.pop();

                for lo in recur_lo {
                    let mut this_lo = base_lo.clone();
                    this_lo.extend(lo);
                    ret.push(this_lo);
                }
            }
        };

        let l_vec = vec![l_min, l_max];
        for this_l in l_vec.into_iter().unique().collect::<Vec<_>>() {
            process_l(&this_l);
        }

        // If there is no new observation, then this function should return
        // vec![vec![]]. I.e., vector with one empty vector.
        if ret.is_empty() {
            ret.push(Vec::new());
        }

        ret
    }

    fn get_observation_vectors(
        relevant_history: &[ObservationNC],
        move_cca: &Option<CCAAction>,
    ) -> (
        Vec<RealNumRep>,
        Vec<RealNumRep>,
        Vec<RealNumRep>,
        RealNumRep,
    ) {
        let mut a = Vec::new();
        let mut l = Vec::new();
        let mut s = Vec::new();
        let mut l0: RealNumRep = 0.into(); // The last loss obs made.
        for i in 1..relevant_history.len() {
            let obs = &relevant_history[i];
            // Use previous a
            a.push(relevant_history[i - 1].a);
            s.push(obs.s);
            for j in 0..obs.lo.len() {
                l0 = std::cmp::max(l0, obs.lo[j].l);
                if obs.lo[j].t > i - 1 {
                    continue;
                }
                let t = i - 1 - obs.lo[j].t;
                assert!(t <= l.len());
                if t == l.len() {
                    l.push(obs.lo[j].l);
                } else {
                    assert_eq!(l[t], obs.lo[j].l);
                }
            }
        }
        if let Some(cca) = move_cca {
            //a.push(*a.last().unwrap() + cca.rate);
            a.push(relevant_history.last().unwrap().a);
        }
        if !l.is_empty() {
            l0 = l.first().unwrap().clone();
        }
        (a, l, s, l0)
    }

    fn get_sim_loss_observation(
        &self,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        prior_loss: i32,
    ) -> Vec<LossObservation> {
        // TODO: Can consider merging this function with NetworkModelNC::compute_loss_observations
        let sim_ideal = self.sim_ideal.unwrap();
        let sim_c = self.sim_c.unwrap();
        let sim_b = self.sim_b.unwrap();

        let tsteps = a.len() as i32;
        assert_eq!(a.len(), s.len());
        let n_losses_observed = l.len() as i32;
        if n_losses_observed == tsteps {
            return vec![];
        }

        // let compute_l_hm: &HashMap<(i32, i32), QeFunSim> = if sim_ideal {
        //     &COMPUTE_SIM_IDEAL_L_OBS
        // } else {
        //     &COMPUTE_SIM_L_OBS
        // };
        let time_ago = (tsteps - 1 - n_losses_observed) as usize;
        let l_interval = expect_qe(
            "L",
            tsteps,
            n_losses_observed,
            self.sim_tables()
                .l_obs(tsteps, prior_loss, n_losses_observed, a, l, s, l0, sim_c, sim_b),
        );
        let this_l = my_min(&l_interval);

        let mut ret = Vec::new();

        if NetworkModelNC::is_loss_observed(
            &a[n_losses_observed as usize],
            &this_l,
            s.last().unwrap(),
        ) {
            ret.push(LossObservation {
                t: time_ago,
                l: this_l,
            });

            let mut l_clone = l.to_vec();
            l_clone.push(this_l);
            let recur_lo = self.get_sim_loss_observation(a, &l_clone, s, l0, prior_loss);
            ret.extend(recur_lo);
        }

        ret
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Once;

    use env_logger::Env;
    use log::{debug, error, info, trace};

    use super::*;

    // https://stackoverflow.com/a/43093371/5039326. We should init logger only
    // once per binary loading. Since each test calls this, we only want to do
    // it once when running multiple tests at once.
    static INIT: Once = Once::new();

    macro_rules! setup_test {
        ($nm:ident, $rh:ident, $mc:ident) => {
            INIT.call_once(|| {
                env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();
            });
            let $nm = NetworkModelNC::new();
            let mut $rh = get_dummy_history();
            let mut $mc = None;
        };
    }

    // `compute_c_from_b_t_6_l_*` for the plain model; see the file header.
    include!("plain_c_from_b_t6.rs");

    /// The plain QE tables with C unsolved above `max_t`, mimicking a QE run
    /// that did not finish.  With `with_c_from_b`, also provide the C-only
    /// clauses of the T=6 B results, as the bursty tables do.
    #[derive(Debug)]
    struct PlainWithoutC {
        max_t: i32,
        with_c_from_b: bool,
    }

    impl QeTables for PlainWithoutC {
        fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            if t > self.max_t {
                return None;
            }
            PlainQe.c(t, n, a, l, s, l0)
        }

        fn c_from_b(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            let f: fn(&[RealNumRep], &[RealNumRep], &[RealNumRep], RealNumRep) -> IntervalList<RealNumRep> =
                match (t, n) {
                    (6, 0) => compute_c_from_b_t_6_l_0,
                    (6, 1) => compute_c_from_b_t_6_l_1,
                    (6, 2) => compute_c_from_b_t_6_l_2,
                    (6, 3) => compute_c_from_b_t_6_l_3,
                    (6, 4) => compute_c_from_b_t_6_l_4,
                    (6, 5) => compute_c_from_b_t_6_l_5,
                    (6, 6) => compute_c_from_b_t_6_l_6,
                    _ => return None,
                };
            self.with_c_from_b.then(|| f(a, l, s, l0))
        }

        fn b(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep, c: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            PlainQe.b(t, n, a, l, s, l0, c)
        }

        fn b_lin(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep, c: Lin)
            -> Option<Option<IntervalList<Lin>>> {
            PlainQe.b_lin(t, n, a, l, s, l0, c)
        }

        fn q(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep, c: RealNumRep, b: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            PlainQe.q(t, n, a, l, s, l0, c, b)
        }

        fn s(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep, c: RealNumRep, b: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            PlainQe.s(t, n, a, l, s, l0, c, b)
        }

        fn l_obs(&self, t: i32, prior: i32, total: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep, c: RealNumRep, b: RealNumRep)
            -> Option<IntervalList<RealNumRep>> {
            PlainQe.l_obs(t, prior, total, a, l, s, l0, c, b)
        }
    }

    const SIM_C: RealNumInt = 20;

    /// Seven-observation windows from a plain-model simulation with C=20,
    /// B=10, driven by a fixed sequence of rates.
    fn simulated_windows() -> Vec<Vec<ObservationNC>> {
        let mut nm = NetworkModelNC::new();
        nm.setup_network_move_sim(false, SIM_C.into(), 10.into());
        let mut history = nm.get_initial_history();
        let mut ret = vec![];
        for rate in [5, 15, 10, 25, 30, 20, 35, 15, 25, 40, 10, 30, 45, 5, 20, 25] {
            let move_cca = Some(CCAAction { rate: rate.into() });
            let rh = &history[history.len() - FEASIBLE_SIZE..];
            let na = nm.get_network_move_sim(rh, &move_cca, false).unwrap();
            let obs = nm.compute_observation(history.last().unwrap(), &na, &move_cca);
            history.push(obs);
            ret.push(history[history.len() - HISTORY_SIZE..].to_vec());
        }
        ret
    }

    /// (exact T=6 C, C computed by `nm` without it), both clipped like a belief.
    fn exact_and_fallback_c(nm: &NetworkModelNC, window: &[ObservationNC]) -> (IntervalList<RealNumRep>, IntervalList<RealNumRep>) {
        let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(window, &None);
        let (t, n) = (a.len() as i32, l.len() as i32);
        let clip = IntervalList::interval_bounded(Bound::Included(0.into()), Bound::Included(LARGEST_BW.into()));
        let exact = PlainQe.c(t, n, &a, &l, &s, l0).unwrap().intersection(&clip);
        let fallback = nm.compute_c(window, t, n, &a, &l, &s, l0).intersection(&clip);
        (exact, fallback)
    }

    /// Falling back to shorter QE windows for C must be sound (contain the
    /// true C) and can only be looser than the full-window result.
    #[test]
    fn test_c_from_sub_windows_is_sound() {
        for max_t in [5, 4] {
            let nm = NetworkModelNC::new().with_qe(Arc::new(PlainWithoutC { max_t, with_c_from_b: false }));
            for window in simulated_windows() {
                let (exact, fallback) = exact_and_fallback_c(&nm, &window);
                assert!(!fallback.intersection(&IntervalList::interval_point(Bound::Included(SIM_C.into()))).is_empty());
                assert_eq!(exact.intersection(&fallback), exact, "max_t={} fallback={:?}", max_t, fallback);
            }
        }
    }

    /// Without the T=6 C result, the exact projection of the B results
    /// (with sub-window C results, if any) must contain the true C and the
    /// T=6 QE C.  It need not equal it: the B formulas were simplified
    /// assuming a feasible C, so B alone can leave extra C feasible.
    #[test]
    fn test_c_tightened_with_b_matches_exact() {
        for max_t in [5, 4, 0] {
            let nm = NetworkModelNC::new().with_qe(Arc::new(PlainWithoutC { max_t, with_c_from_b: true }));
            for window in simulated_windows() {
                let (exact, fallback) = exact_and_fallback_c(&nm, &window);
                let (e_lo, e_hi, f_lo, f_hi) = (my_min(&exact), my_max(&exact), my_min(&fallback), my_max(&fallback));
                let msg = format!("max_t={} exact=[{}, {}] fallback=[{}, {}]", max_t, e_lo, e_hi, f_lo, f_hi);
                assert!(f_lo <= SIM_C.into() && RealNumRep::from(SIM_C) <= f_hi, "{}", msg);
                assert!(f_lo <= e_lo && e_hi <= f_hi, "{}", msg);
                if (f_lo, f_hi) != (e_lo, e_hi) {
                    println!("looser: {}", msg);
                }
            }
        }
    }

    const BURSTY_PERT: RealNumRep = RealNumRep::new_raw(1, 2);

    fn bursty(k: RealNumInt) -> Arc<dyn QeTables> {
        Arc::new(BurstyQe { k: k.into(), pert: BURSTY_PERT })
    }

    /// Seven-observation windows from a bursty simulation with burstiness
    /// `k`, buffer `b` and C=`SIM_C`, driven by `rates`.  `Random` uses a
    /// fixed sequence of fractions.
    fn bursty_windows(k: RealNumInt, b: RealNumInt, policy: SimPolicy, rates: &[RealNumInt]) -> Vec<Vec<ObservationNC>> {
        let mut nm = NetworkModelNC::new().with_qe(bursty(k));
        nm.setup_network_move_sim(false, SIM_C.into(), b.into());
        let mut history = nm.get_initial_history();
        let mut ret = vec![];
        for (i, &rate) in rates.iter().enumerate() {
            let move_cca = Some(CCAAction { rate: rate.into() });
            let rh = &history[history.len() - FEASIBLE_SIZE..];
            let u = RealNumRep::new((i as RealNumInt * 7) % 16, 16);
            let na = nm.get_network_move_sim_policy(rh, &move_cca, policy, u).unwrap();
            let obs = nm.compute_observation(history.last().unwrap(), &na, &move_cca);
            history.push(obs);
            ret.push(history[history.len() - HISTORY_SIZE..].to_vec());
        }
        ret
    }

    const PROBE_RATES: [RealNumInt; 16] = [5, 15, 10, 25, 30, 20, 35, 15, 25, 40, 10, 30, 45, 5, 20, 25];

    /// The burstiness that generated a trace must never be ruled out.
    #[test]
    fn test_true_k_is_feasible() {
        for (k, b) in [(1, 5), (4, 9), (16, 33)] {
            for policy in [SimPolicy::Max, SimPolicy::Min, SimPolicy::Random] {
                let nm = NetworkModelNC::new().with_qe(bursty(k));
                for (i, window) in bursty_windows(k, b, policy, &PROBE_RATES).iter().enumerate() {
                    assert!(nm.window_feasible(window), "k={} b={} {:?} window {}", k, b, policy, i);
                }
            }
        }
    }

    /// Exploratory: which K hypotheses each (true K, B, policy) trace rules
    /// out.  Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_k_feasibility() {
        let hypotheses = [1, 4, 16, 64];
        for (k, b) in [(1, 5), (1, 200), (4, 9), (16, 33), (64, 129)] {
            for policy in [SimPolicy::Max, SimPolicy::Min, SimPolicy::Random] {
                let windows = bursty_windows(k, b, policy, &PROBE_RATES);
                let row: Vec<String> = hypotheses
                    .iter()
                    .map(|&h| {
                        let nm = NetworkModelNC::new().with_qe(bursty(h));
                        let first_dead = windows.iter().position(|w| !nm.window_feasible(w));
                        format!("K={}:{}", h, first_dead.map_or("alive".to_string(), |i| format!("dead@{}", i)))
                    })
                    .collect();
                println!("true k={:>2} b={:>3} {:?}: {}", k, b, policy, row.join(" "));
            }
        }
    }

    /// Burst patterns for the forward simulator: (name, choice at step i).
    fn link_patterns() -> Vec<(&'static str, fn(usize) -> LinkChoice)> {
        fn none(_: usize) -> LinkChoice {
            LinkChoice { burst: 0.into(), service: 1.into() }
        }
        fn max(_: usize) -> LinkChoice {
            LinkChoice { burst: 1.into(), service: 0.into() }
        }
        fn random(i: usize) -> LinkChoice {
            LinkChoice {
                burst: RealNumRep::new((i as RealNumInt * 7) % 16, 16),
                service: RealNumRep::new((i as RealNumInt * 11 + 3) % 16, 16),
            }
        }
        fn alternate(i: usize) -> LinkChoice {
            LinkChoice { burst: ((i % 2) as RealNumInt).into(), service: RealNumRep::new(1, 2) }
        }
        vec![("none", none), ("max", max), ("random", random), ("alternate", alternate)]
    }

    /// Seven-observation windows from the forward simulator.
    fn forward_windows(k: RealNumInt, b: RealNumInt, choose: fn(usize) -> LinkChoice, rates: &[RealNumInt]) -> Vec<Vec<ObservationNC>> {
        let nm = NetworkModelNC::new().with_qe(bursty(k));
        let mut history = nm.get_initial_history();
        let mut sim = BurstyLinkSim::new(SIM_C.into(), b.into(), k.into(), history.len());
        let mut ret = vec![];
        for (i, &rate) in rates.iter().enumerate() {
            let move_cca = Some(CCAAction { rate: rate.into() });
            let na = sim.step(history.last().unwrap().a, history.len(), choose(i));
            let obs = nm.compute_observation(history.last().unwrap(), &na, &move_cca);
            history.push(obs);
            ret.push(history[history.len() - HISTORY_SIZE..].to_vec());
        }
        ret
    }

    /// The whole forward-simulated history (initial observations included).
    fn forward_history(k: RealNumInt, b: RealNumInt, c: RealNumInt, choose: fn(usize) -> LinkChoice, rates: &[RealNumInt]) -> Vec<ObservationNC> {
        let nm = NetworkModelNC::new().with_qe(bursty(k));
        let mut history = nm.get_initial_history();
        let mut sim = BurstyLinkSim::new(c.into(), b.into(), k.into(), history.len());
        for (i, &rate) in rates.iter().enumerate() {
            let move_cca = Some(CCAAction { rate: rate.into() });
            let na = sim.step(history.last().unwrap().a, history.len(), choose(i));
            let obs = nm.compute_observation(history.last().unwrap(), &na, &move_cca);
            history.push(obs);
        }
        history
    }

    /// Chained C bounds must contain the true C on every forward-simulated
    /// history and every suffix of it (a suffix starts with a non-empty
    /// queue), with no tolerance.  Prints how tight they get.
    #[test]
    fn test_chained_c_bounds_sound() {
        let mut rates: Vec<RealNumInt> = LONG_RATES.to_vec();
        // A long backlogged stretch at about 1.2 C, then a drain.
        rates.extend([24; 24]);
        rates.extend([0, 0, 10, 10]);
        for c in [10, 20, 37] {
            let scaled: Vec<RealNumInt> = rates.iter().map(|&r| r * c / 20).collect();
            for (k, b) in [(1, 5), (4, 9), (16, 33), (1, 100), (64, 200)] {
                for (name, choose) in link_patterns() {
                    let history = forward_history(k, b, c, choose, &scaled);
                    let nm = NetworkModelNC::new().with_qe(bursty(k));
                    let mut tightest = (RealNumRep::from(0), RealNumRep::from(LARGEST_BW));
                    for start in (0..history.len() - 3).step_by(3) {
                        for end in (start + 3..=history.len()).rev().step_by(2) {
                            let (lo, hi) = nm.chained_c_bounds(&history[start..end]).unwrap();
                            assert!(
                                lo <= c.into() && RealNumRep::from(c) <= hi,
                                "c={} k={} b={} {} [{}, {}): C in [{}, {}]",
                                c, k, b, name, start, end, lo, hi
                            );
                            if start == 0 {
                                tightest = (std::cmp::max(tightest.0, lo), std::cmp::min(tightest.1, hi));
                            }
                        }
                    }
                    println!("c={} k={} b={} {}: whole history C in [{}, {}]", c, k, b, name, tightest.0, tightest.1);
                }
            }
        }
    }

    const LONG_RATES: [RealNumInt; 32] = [
        5, 15, 10, 25, 30, 20, 35, 15, 25, 40, 10, 30, 45, 5, 20, 25,
        1, 1, 1, 1, 1, 1, 1, 1, 30, 30, 30, 30, 0, 0, 22, 22,
    ];

    /// Forward-simulated traces must never rule out the true K.
    #[test]
    fn test_forward_sim_true_k_is_feasible() {
        for (k, b) in [(1, 5), (4, 9), (16, 33), (1, 100)] {
            for (name, choose) in link_patterns() {
                let nm = NetworkModelNC::new().with_qe(bursty(k));
                for (i, window) in forward_windows(k, b, choose, &LONG_RATES).iter().enumerate() {
                    assert!(nm.window_feasible(window), "k={} b={} {} window {}", k, b, name, i);
                }
            }
        }
    }

    /// The joint (C, B) bound over a whole forward-simulated history must
    /// contain the true C and B, and be no looser than intersecting the
    /// per-window bounds.  Prints where it is strictly tighter.
    #[test]
    fn test_forward_sim_joint_bounds_sound() {
        let mut tighter = 0;
        for (k, b) in [(1, 5), (4, 9), (16, 33), (1, 100)] {
            for (name, choose) in link_patterns() {
                let nm = NetworkModelNC::new().with_qe(bursty(k));
                let windows = forward_windows(k, b, choose, &LONG_RATES);
                let (mut lo_c, mut hi_c, mut lo_b, mut hi_b) =
                    (RealNumRep::from(0), RealNumRep::from(LARGEST_BW), RealNumRep::from(0), RealNumRep::from(LARGEST_BW));
                for i in 0..windows.len() {
                    let bb = nm.try_window_belief(&windows[i]).unwrap();
                    lo_c = std::cmp::max(lo_c, bb.min_c);
                    hi_c = std::cmp::min(hi_c, bb.max_c);
                    lo_b = std::cmp::max(lo_b, bb.min_b);
                    hi_b = std::cmp::min(hi_b, bb.max_b);
                    let refs: Vec<(&[ObservationNC], &Option<CCAAction>)> =
                        windows[..=i].iter().map(|w| (w.as_slice(), &None)).collect();
                    let j = nm.joint_c_b_bounds(&refs, None).unwrap().unwrap_or_else(|| panic!("k={} b={} {} window {}: jointly infeasible", k, b, name, i));
                    let msg = format!(
                        "k={} b={} {} window {}: joint c=[{}, {}] b=[{}, {}], boxes c=[{}, {}] b=[{}, {}]",
                        k, b, name, i, j.min_c, j.max_c, j.min_b, j.max_b, lo_c, hi_c, lo_b, hi_b
                    );
                    assert!(j.min_c <= SIM_C.into() && RealNumRep::from(SIM_C) <= j.max_c, "C excluded: {}", msg);
                    assert!(j.min_b <= b.into() && RealNumRep::from(b) <= j.max_b, "B excluded: {}", msg);
                    assert!(j.min_cb <= RealNumRep::from(SIM_C + b), "C + B excluded: min_cb={} {}", j.min_cb, msg);
                    let w = nm.try_window_belief(&windows[i]).unwrap();
                    assert!(w.min_cb <= RealNumRep::from(SIM_C + b), "window C + B excluded: min_cb={} {}", w.min_cb, msg);
                    assert!(lo_c <= j.min_c && j.max_c <= hi_c && lo_b <= j.min_b && j.max_b <= hi_b, "looser: {}", msg);
                    if (j.min_c, j.max_c, j.min_b, j.max_b) != (lo_c, hi_c, lo_b, hi_b) {
                        tighter += 1;
                        println!("tighter: {}", msg);
                    }
                }
            }
        }
        println!("joint strictly tighter in {} prefixes", tighter);
    }

    /// Intersecting every window's C and B bounds under the true K must stay
    /// non-empty and contain the true C and B, exactly: the projection is
    /// exact, so no tolerance is needed.
    #[test]
    fn test_forward_sim_true_k_bounds_intersect() {
        let tol: RealNumRep = 0.into();
        for (k, b) in [(1, 5), (4, 9), (16, 33), (1, 100)] {
            for (name, choose) in link_patterns() {
                let nm = NetworkModelNC::new().with_qe(bursty(k));
                let (mut lo_c, mut hi_c, mut lo_b, mut hi_b) =
                    (RealNumRep::from(0), RealNumRep::from(LARGEST_BW), RealNumRep::from(0), RealNumRep::from(LARGEST_BW));
                for (i, window) in forward_windows(k, b, choose, &LONG_RATES).iter().enumerate() {
                    let bb = nm.try_window_belief(window).unwrap_or_else(|| panic!("k={} b={} {} window {} infeasible", k, b, name, i));
                    lo_c = std::cmp::max(lo_c, bb.min_c);
                    hi_c = std::cmp::min(hi_c, bb.max_c);
                    lo_b = std::cmp::max(lo_b, bb.min_b);
                    hi_b = std::cmp::min(hi_b, bb.max_b);
                    let msg = format!("k={} b={} {} window {}: c=[{}, {}] b=[{}, {}]", k, b, name, i, lo_c, hi_c, lo_b, hi_b);
                    assert!(lo_c <= hi_c + tol && lo_b <= hi_b + tol, "{}", msg);
                    assert!(lo_c - tol <= SIM_C.into() && RealNumRep::from(SIM_C) <= hi_c + tol, "C excluded: {}", msg);
                    assert!(lo_b - tol <= b.into() && RealNumRep::from(b) <= hi_b + tol, "B excluded: {}", msg);
                }
            }
        }
    }

    /// Debug: per-window bounds of the failing K=1, B=5 bursting trace.
    #[test]
    #[ignore]
    fn debug_window_bounds_exclude_truth() {
        let (_, choose) = link_patterns()[1];
        let nm = NetworkModelNC::new().with_qe(bursty(16));
        for (i, w) in forward_windows(16, 33, choose, &LONG_RATES).iter().enumerate().take(32).skip(24) {
            let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(w, &None);
            let (t, n) = (a.len() as i32, l.len() as i32);
            let raw = nm.qe.c(t, n, &a, &l, &s, l0);
            let (partial, _) = nm.c_from_partial_results(w, t, n, &a, &l, &s, l0);
            let full = nm.compute_c(w, t, n, &a, &l, &s, l0);
            let b_true = nm.qe.b(t, n, &a, &l, &s, l0, SIM_C.into());
            let bb = nm.try_window_belief(w);
            println!("  sub={:?}\n  from_b={:?}", nm.c_from_sub_windows(w, t), nm.qe.c_from_b(t, n, &a, &l, &s, l0));
            println!(
                "w{} n={} qe.c={:?}\n  partial={:?}\n  tightened={:?}\n  B(C=20)={:?}\n  belief c=[{:?}] b=[{:?}]",
                i, n, raw, partial, full, b_true,
                bb.map(|b| (b.min_c.to_f64().unwrap(), b.max_c.to_f64().unwrap())),
                bb.map(|b| (b.min_b.to_f64().unwrap(), b.max_b.to_f64().unwrap()))
            );
        }
    }

    /// Debug: every sub-window C result of the K=16, B=33 bursting window 31,
    /// as Python literals for a z3 check.
    #[test]
    #[ignore]
    fn dump_sub_windows() {
        let (_, choose) = link_patterns()[1];
        let nm = NetworkModelNC::new().with_qe(bursty(16));
        let w = &forward_windows(16, 33, choose, &LONG_RATES)[31];
        let py = |v: &[RealNumRep]| format!("[{}]", v.iter().map(|x| format!("F({}, {})", x.numer(), x.denom())).collect::<Vec<_>>().join(", "));
        for sub_t in [5usize, 4, 3, 2] {
            for first in 0..=(w.len() - sub_t - 1) {
                let sw = &w[first..=first + sub_t];
                let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(sw, &None);
                if let Some(c) = nm.qe.c(sub_t as i32, l.len() as i32, &a, &l, &s, l0) {
                    println!(
                        "W dict(a={}, l={}, s={}, l0=F({}, {}), c=({}, {}), first={})",
                        py(&a), py(&l), py(&s), l0.numer(), l0.denom(),
                        my_min(&c).to_f64().unwrap(), my_max(&c).to_f64().unwrap(), first
                    );
                }
            }
        }
    }

    /// Exploratory: like `print_k_feasibility`, for forward-simulated traces.
    #[test]
    #[ignore]
    fn print_forward_k_feasibility() {
        let hypotheses = [1, 4, 16, 64];
        for (k, b) in [(1, 5), (1, 200), (4, 9), (16, 33), (64, 129)] {
            for (name, choose) in link_patterns() {
                let windows = forward_windows(k, b, choose, &LONG_RATES);
                let row: Vec<String> = hypotheses
                    .iter()
                    .map(|&h| {
                        let nm = NetworkModelNC::new().with_qe(bursty(h));
                        let first_dead = windows.iter().position(|w| !nm.window_feasible(w));
                        format!("K={}:{}", h, first_dead.map_or("alive".to_string(), |i| format!("dead@{}", i)))
                    })
                    .collect();
                println!("fwd true k={:>2} b={:>3} {:>9}: {}", k, b, name, row.join(" "));
            }
        }
    }

    /// Debug: dump forward-simulated windows (K=4, B=9) for the z3 check.
    #[test]
    #[ignore]
    fn dump_forward_windows() {
        let py = |v: &[RealNumRep]| format!("[{}]", v.iter().map(|x| format!("F({}, {})", x.numer(), x.denom())).collect::<Vec<_>>().join(", "));
        for (name, choose) in link_patterns() {
            for w in forward_windows(4, 9, choose, &LONG_RATES) {
                let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(&w, &None);
                println!("W dict(a={}, l={}, s={}, l0=F({}, {}), name='{}')", py(&a), py(&l), py(&s), l0.numer(), l0.denom(), name);
            }
        }
    }

    /// Debug: dump the QE inputs of every window of a K=4, B=9 `Min` trace
    /// as Python literals, to check them against the z3 model directly.
    #[test]
    #[ignore]
    fn dump_min_policy_windows() {
        let mut rates = vec![];
        for _ in 0..4 {
            rates.extend([RealNumRep::from(11), 0.into(), 0.into(), 1.into(), RealNumRep::new(161, 16)]);
        }
        rates.extend(std::iter::repeat(RealNumRep::from(1)).take(20));
        let mut nm = NetworkModelNC::new().with_qe(bursty(4));
        nm.setup_network_move_sim(false, SIM_C.into(), 9.into());
        let mut history = nm.get_initial_history();
        let py = |v: &[RealNumRep]| format!("[{}]", v.iter().map(|x| format!("F({}, {})", x.numer(), x.denom())).collect::<Vec<_>>().join(", "));
        for rate in rates {
            let move_cca = Some(CCAAction { rate });
            let rh = &history[history.len() - FEASIBLE_SIZE..];
            let na = nm.get_network_move_sim_policy(rh, &move_cca, SimPolicy::Min, 0.into()).unwrap();
            let obs = nm.compute_observation(history.last().unwrap(), &na, &move_cca);
            history.push(obs);
            let w = &history[history.len() - HISTORY_SIZE..];
            let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(w, &None);
            println!("W dict(a={}, l={}, s={}, l0=F({}, {}))", py(&a), py(&l), py(&s), l0.numer(), l0.denom());
        }
    }

    /// Debug: why the true K=1 is ruled out on a B=5 random trace.
    #[test]
    #[ignore]
    fn debug_true_k_infeasible() {
        let windows = bursty_windows(1, 5, SimPolicy::Random, &PROBE_RATES);
        let nm = NetworkModelNC::new().with_qe(bursty(1));
        for (i, w) in windows.iter().enumerate() {
            let (a, l, s, l0) = NetworkModelNC::get_observation_vectors(w, &None);
            let (t, n) = (a.len() as i32, l.len() as i32);
            let (c, _) = nm.c_from_partial_results(w, t, n, &a, &l, &s, l0);
            let from_b = nm.qe.c_from_b(t, n, &a, &l, &s, l0);
            let b_true = nm.qe.b(t, n, &a, &l, &s, l0, SIM_C.into());
            println!(
                "w{} feasible={} a={:?} s={:?} l={:?} l0={}\n   c={:?}\n   c_from_b={:?}\n   B(C=20)={:?}",
                i,
                nm.window_feasible(w),
                a.iter().map(|x| x.to_f64().unwrap()).collect::<Vec<_>>(),
                s.iter().map(|x| x.to_f64().unwrap()).collect::<Vec<_>>(),
                l.iter().map(|x| x.to_f64().unwrap()).collect::<Vec<_>>(),
                l0,
                c,
                from_b,
                b_true
            );
        }
    }

    fn get_dummy_history() -> Vec<ObservationNC> {
        let mut ret = Vec::new();
        ret.push(ObservationNC {
            a: 10.into(),
            s: 0.into(),
            lo: vec![LossObservation { t: 0, l: 0.into() }],
        });
        for t in 1..(FEASIBLE_SIZE - 1) {
            ret.push(ObservationNC {
                a: (((t + 1) * 10) as i32).into(),
                s: ((t * 10) as i32).into(),
                lo: vec![LossObservation { t: 1, l: 0.into() }],
            });
        }
        ret[0].lo.push(LossObservation { t: 0, l: 0.into() });
        ret
    }

    #[test]
    fn test_compute_beliefs() {
        setup_test!(nm, rh, mc);

        let bb = nm.compute_belief_bounds(&rh, &mc);
        info!("{:?}", bb);
    }

    // #[test]
    // fn test_compute_max_allowed_rate() {
    //     setup_test!(nm, rh, mc);
    //     rh.pop();

    //     let rate = nm.compute_max_allowed_rate(&rh, &mc, (2 * ALPHA).into(), 3.into(), 1);
    //     info!("rate={:?}", rate);
    // }

    #[test]
    fn test_compute_feasible_network_actions() {
        setup_test!(nm, rh, mc);
        rh.pop();
        mc = Some(CCAAction { rate: 10.into() });

        let nas = nm.compute_feasible_network_moves(&rh, &mc);
        info!("Got nas:");
        for na in nas {
            info!("{:?}", na);
        }
    }

    #[test]
    fn test_compute_loss_observations() {
        INIT.call_once(|| {
            env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();
        });
        let a: Vec<RealNumRep> = vec![0.into(), 11.into(), 22.into(), 33.into(), 44.into()];
        let s: Vec<RealNumRep> = vec![0.into(), 10.into(), 20.into(), 30.into(), 40.into()];
        let l: Vec<RealNumRep> = vec![0.into(), 1.into(), 2.into()];

        let l_intervals = COMPUTE_L_OBS[&(5, 2, 3)](&a, &l, &s, 0.into(), 20.into(), 10.into());
        trace!("l_intervals={:?}", l_intervals);
    }

    #[test]
    fn dummy_test() {
        let mut c_interval_all: IntervalList<RealNumRep> = IntervalList::interval_bounded(
            Bound::Included(RealNumRep::new_raw(34, 3)),
            Bound::Included(RealNumRep::new_raw(35, 2)),
        );
        let c_vals = NetworkModelNC::discretize_interval(&c_interval_all, N_DISCRETE_POINTS);
        print!("{:?}", c_vals);
    }

    #[test]
    fn debug_test() {
        let mut obs = vec![];
        obs.push(ObservationNC {
            a: 60.into(),
            s: 42.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 60.into(),
            s: 60.into(),
            lo: vec![
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 80.into(),
            s: 60.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 80.into(),
            s: 80.into(),
            lo: vec![
                LossObservation {
                    t: 1,
                    l: 0.into(), // it is in discretization unit of packet
                },
                LossObservation {
                    t: 0,
                    l: 0.into(), // it is in discretization unit of packet
                },
            ],
        });
        obs.push(ObservationNC {
            a: 102.into(),
            s: 80.into(),
            lo: vec![LossObservation {
                t: 1,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 102.into(),
            s: 100.into(),
            lo: vec![LossObservation {
                t: 2,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        let move_cca = CCAAction { rate: 10.into() };

        let network_model = NetworkModelNC::new();
        let len = obs.len();
        let res = network_model.compute_min_c_b_sum(&obs[0..6], &None);

        print!("{:?}", res);
        print!("{:?}", res.unwrap().to_f64());
    }

    #[test]
    fn debug_max_rate() {
        let mut obs = vec![];
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 0.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        obs.push(ObservationNC {
            a: 12.into(),
            s: 0.into(),
            lo: vec![LossObservation {
                t: 0,
                l: 0.into(), // it is in discretization unit of packet
            }],
        });
        //let move_cca = None;

        let network_model = NetworkModelNC::new();
        let len = obs.len();
        let res = network_model.compute_max_allowed_rate(&obs, &None, 0.into(), 0.into(), 0);

        print!("{:?}", res);
        print!("{:?}", res.unwrap().to_f64());
    }
}

