use clap::Parser;
use env_logger::Env;
use itertools::Itertools;
use log::{debug, info, warn};
use num_traits::ToPrimitive;
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::path::Path;
use std::rc::{Rc, Weak};

use cc_common::*;
use minimax::State;
// use network_model_z3::NetworkModelZ3;
use ds::tree::Tree;
use ds::*;
use network_model_nc::{
    BurstyLinkSim, BurstyQe, LinkChoice, LossObservation, NetworkActionNC, NetworkModelNC, UnionBurstyQe,
};

mod cbrdelay_cca_config;
use cbrdelay_cca_config::{
    explore_rule, record_action, state_policy, update_k, BeliefStrategy, CandidateAction, InconsistentQeWindows, KPolicy,
    PlannerState, K_CONFIRM_RTTS,
    StatePolicy, StateQuantities, APP_IS_BACKLOGGED, APP_SEND_CHOICES, BURSTY_PERT, DRAIN_INTERVAL_RTTS,
    EXPLORE_BUDGET_BDP, EXPLORE_MAX_QUEUE_BDP, K_POLICY, PROBE_INTERVAL_RTTS, QE_MODEL, QE_WINDOW_OBSERVATIONS, SEARCH_DEPTH, QeModel,
};

// ------------------------------------------------------------------------------------------
// Minimax move
#[derive(Clone, PartialEq, Debug, Eq, Hash, Serialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum CCMove<NA: NetworkAction> {
    CCA(CCAAction),
    Network(NA),
}

impl<NA: NetworkAction> Display for CCMove<NA> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CCMove::CCA(a) => write!(f, "rate={:.2}", a.rate.to_f64().unwrap()),
            CCMove::Network(n) => write!(f, "{}", n),
        }
    }
}

impl<NA: NetworkAction> minimax::Move for CCMove<NA> {}

// ------------------------------------------------------------------------------------------
// Minimax transposition table state, and search tree
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CCTTState<O: Observation> {
    history: Vec<O>,
    move_cca: Option<CCAAction>,
}

impl<O: Observation> minimax::TTState for CCTTState<O> {}

#[derive(Debug, Serialize)]
struct TreeData<O: Observation, NA: NetworkAction> {
    observation: O,
    #[serde(with = "serde_optional_real")]
    cca_rate: Option<RealNumRep>,
    history: Vec<O>,
    belief_bounds: BeliefBounds,
    metrics: Metrics,
    chosen_move: Option<CCMove<NA>>,
    chosen_value: Option<CCValue>,
}

// ------------------------------------------------------------------------------------------
// Minimax value
#[derive(Clone, PartialEq, Debug, Copy, Serialize)]
enum CCValue {
    Inf,
    NegInf,
    Value {
        belief_bounds: BeliefBounds,
        metrics: Metrics,
        objective_order: u128,
    },
}

impl CCValue {
    fn cmp_c_beliefs(b1: &BeliefBounds, b2: &BeliefBounds) -> Option<Ordering> {
        (b1.max_c - b1.min_c).partial_cmp(&(b2.max_c - b2.min_c))
    }

    fn cmp_b_beliefs(b1: &BeliefBounds, b2: &BeliefBounds) -> Option<Ordering> {
        (b1.max_b - b1.min_b).partial_cmp(&(b2.max_b - b2.min_b))
    }

    // fn cmp_metrics(m1: &Metrics, m2: &Metrics) -> Option<Ordering> {
    //     m1.partial_cmp(m2)
    // }

    fn cmp_entry_point(&self, other: &Self) -> Option<Ordering> {
        let (b1, m1, objective_order) = match self {
            CCValue::Value {
                belief_bounds,
                metrics,
                objective_order,
            } => (belief_bounds, metrics, objective_order),
            _ => panic!("Should not be here"),
        };
        let (b2, m2) = match other {
            CCValue::Value {
                belief_bounds,
                metrics,
                ..
            } => (belief_bounds, metrics),
            _ => panic!("Should not be here"),
        };

        let c = Self::cmp_c_beliefs(b1, b2);
        let b = Self::cmp_b_beliefs(b1, b2);

        // let m = Self::cmp_metrics(m1, m2);
        // If shrink = true, then we want less than. bool would give greater
        // than, so we reverse the order of 1 and 2.
        let sc = m2.shrink_c.partial_cmp(&m1.shrink_c);
        let sb = m2.shrink_b.partial_cmp(&m1.shrink_b);
        let schmul = m2.shrink_maxc_mul.partial_cmp(&m1.shrink_maxc_mul);
        let schadd = m2.shrink_maxc_add.partial_cmp(&m1.shrink_maxc_add);
        let sclmul = m2.shrink_minc_mul.partial_cmp(&m1.shrink_minc_mul);
        let scladd = m2.shrink_minc_add.partial_cmp(&m1.shrink_minc_add);

        let s = m1.neg_delivered.partial_cmp(&m2.neg_delivered);
        let ld = m1.loss.partial_cmp(&m2.loss);
        let l = m1.total_loss.partial_cmp(&m2.total_loss);
        let qdel = m1.qdel.partial_cmp(&m2.qdel);
        let ttsc = m1.time_to_shrink_c.partial_cmp(&m2.time_to_shrink_c);
        let ttscl = m1.time_to_shrink_c_l.partial_cmp(&m2.time_to_shrink_c_l);
        let lt = m2.loss_tol.partial_cmp(&m1.loss_tol);
        let ttc = m1.time_to_c.partial_cmp(&m2.time_to_c);
        let cb = m2.minc_minb_sum.partial_cmp(&m1.minc_minb_sum);
        let ttcb = m1.time_to_c_b.partial_cmp(&m2.time_to_c_b);
        let ttscbadd = m1
            .time_to_shrink_minc_minb_add
            .partial_cmp(&m2.time_to_shrink_minc_minb_add);
        let scb = m2.shrink_minc_minb.partial_cmp(&m1.shrink_minc_minb);
        let sent = m1.neg_sent.partial_cmp(&m2.neg_sent);

        let comparisons = [
            c, b, sc, sb, s, ld, l, qdel, schmul, schadd, sclmul, scladd, ttsc, lt, ttc, ttscl,
            cb, ttcb, ttscbadd, scb, sent,
        ];
        for idx in decode_objective_order(*objective_order) {
            let r = comparisons[idx];
            if r.is_some() && r.unwrap() != Ordering::Equal {
                return r;
            }
        }
        None
    }
}

const OBJECTIVE_NAMES: [&str; 21] = [
    "c", "b", "sc", "sb", "s", "ld", "l", "qdel", "schmul", "schadd", "sclmul", "scladd", "ttsc", "lt",
    "ttc", "ttscl", "cb", "ttcb", "ttscbadd", "scb", "sent",
];

/// Compact, `Copy` representation required by the generic minimax value API.
/// Each five-bit entry is an index into `OBJECTIVE_NAMES`; 31 terminates it.
fn encode_objective_order(names: &[String]) -> u128 {
    assert!(names.len() <= OBJECTIVE_NAMES.len());
    names
        .iter()
        .enumerate()
        .fold(0u128, |code, (position, name)| {
            let index = OBJECTIVE_NAMES
                .iter()
                .position(|known| *known == name)
                .unwrap_or_else(|| panic!("unknown CCA objective: {name}"));
            code | ((index as u128) << (position * 5))
        })
        | (31u128 << (names.len() * 5))
}

fn decode_objective_order(code: u128) -> impl Iterator<Item = usize> {
    (0..OBJECTIVE_NAMES.len())
        .map(move |position| ((code >> (position * 5)) & 31) as usize)
        .take_while(|index| *index != 31)
}

impl minimax::Value for CCValue {
    fn inf() -> Self {
        CCValue::Inf
    }

    fn neg_inf() -> Self {
        CCValue::NegInf
    }

    fn fully_known(&self) -> bool {
        false
    }
}

impl PartialOrd for CCValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // smaller beliefs implies smaller value.
        // better metrics implies smaller value.
        match self {
            CCValue::Inf => match other {
                CCValue::Inf => Some(Ordering::Equal),
                _ => Some(Ordering::Greater),
            },
            CCValue::NegInf => match other {
                CCValue::NegInf => Some(Ordering::Equal),
                _ => Some(Ordering::Less),
            },
            CCValue::Value { .. } => match other {
                CCValue::Inf => Some(Ordering::Less),
                CCValue::NegInf => Some(Ordering::Greater),
                CCValue::Value { .. } => self.cmp_entry_point(other),
            },
        }
    }
}

// ------------------------------------------------------------------------------------------
// Minimax state
#[derive(Debug)]
struct CongCtrlState<NM: NetworkModel> {
    // TODO: Only compute something if strictly needed.
    // fill other values with None.
    network_model: NM, // NetworkModelType<'a>,

    // Simulation
    /// All committed observations.  QE calls below take bounded windows from
    /// this sequence, so retaining it does not increase QE formula size.
    history: Vec<NM::O>,
    belief_bounds: BeliefBounds, // at the end of ^^ history.

    // Speculation

    // The belief bounds don't take into account the move_cca variable. I.e.,
    // the bounds are always computed after the network's move and are used for
    // enumerating CCA's moves and defining objective function.

    // All the stacks are in lock step
    history_stack: Vec<NM::O>, // board state after network move
    belief_bounds_stack: Vec<BeliefBounds>, // beliefs at the end of the sequence of observations
    move_choices_cca_stack: Vec<Vec<CCMove<NM::NA>>>, // move choices at each step of the speculation for the CCA.
    move_choices_network_stack: Vec<Vec<CCMove<NM::NA>>>, // move choices at each step of the speculation for the network.

    // Joint
    move_cca: Option<CCAAction>, // board state after CCA move (as we lack a observation)

    // To understand decisions based on speculation.
    root: Option<Rc<Tree<TreeData<NM::O, NM::NA>>>>, // Only keeping root so that we don't drop the tree.
    current: Weak<Tree<TreeData<NM::O, NM::NA>>>,
    recording_tree: bool,

    disable_pre_enumeration: bool,

    // Config
    policy: StatePolicy,
    /// Long-horizon planner memory; see `cbrdelay_cca_config::state_policy`.
    planner: PlannerState,
    /// K set of the bursty belief (`None` for the plain model).
    qe_ks: Option<Vec<RealNumInt>>,
    /// Belief bounds of committed QE windows, which never change for a given
    /// K set.  Speculative windows are not cached.
    window_cache: RefCell<HashMap<Vec<NM::O>, Option<BeliefBounds>>>,
    /// The last committed belief and the K set it assumed.  C and B are
    /// static, so a later belief is intersected with it rather than allowed
    /// to get broader; see `retained_prior`.
    /// The K set, the bounds and `PlannerState::belief_start` they came from.
    retained: Option<(Option<Vec<RealNumInt>>, BeliefBounds, usize)>,
    /// `--belief-history N` (`None`: whole-history beliefs, which retain the
    /// prior belief on every RTT).
    belief_history: Option<usize>,
    /// Per committed RTT, whether it was app-limited.
    app_limited_rtts: Vec<bool>,
    /// The candidate that produced the last committed CCA move.
    last_action: Option<CandidateAction>,

    n_nodes_explored: u32,

    parallel_binary_search: bool,
}

impl<NM: NetworkModel> minimax::State for CongCtrlState<NM> {
    // Used for speculation into the future game states

    type M = CCMove<NM::NA>;
    type V = CCValue;
    type T = CCTTState<NM::O>;

    fn maximizer(&self) -> bool {
        // if CCA has moved then it is network's turn (i.e., maximizer)
        // CCA is minimizer
        self.move_cca.is_some()
    }

    fn game_over(&self) -> Option<Self::V> {
        None
    }

    fn best_move_guess(&mut self, _tt: &minimax::TranspositionTable<Self>) -> Option<Self::M> {
        if self.maximizer() {
            // network's turn
            self.move_choices_network_stack.last_mut().unwrap().pop()
        } else {
            self.move_choices_cca_stack.last_mut().unwrap().pop()
        }
    }

    fn best_move_sort(&mut self, tt: &minimax::TranspositionTable<Self>, tt_state: Self::T) {
        // do nothing
    }

    fn value(&self) -> Self::V {
        let latest = self.get_latest_belief_bounds();
        // let start = &self.belief_bounds;

        CCValue::Value {
            metrics: self.compute_metrics(),
            // self.belief_bounds_stack.last().unwrap().clone(),
            belief_bounds: *latest,
            objective_order: encode_objective_order(&self.policy.objective_order),
            // Deepen tree may query for a value after CCA move.
            // TODO: update it to only query on even moves.
            // Basically just explore even depths.
        }
    }

    fn make_move_depth(&mut self, m: &Self::M, depth: u16) {
        self.make_move(m);
    }

    fn make_move(&mut self, m: &Self::M) {
        self.n_nodes_explored += 1;
        debug!("Make move {:?}", m);
        match m {
            CCMove::CCA(a) => {
                self.move_cca = Some(*a);
                if self.disable_pre_enumeration {
                    self.move_choices_network_stack.push(Vec::new());
                } else {
                    self.move_choices_network_stack
                        .push(self.compute_feasible_network_moves());
                }
            }
            CCMove::Network(n) => {
                let last = self.get_last_history_item();
                let observation = self
                    .network_model
                    .compute_observation(last, n, &self.move_cca);

                self.move_cca = None;
                self.history_stack.push(observation);
                self.belief_bounds_stack.push(self.compute_belief_bounds());
                self.move_choices_cca_stack.push(self.compute_cca_moves());
            }
        }
        self.tree_make_move();
    }

    fn pop_move(&mut self) {
        if self.maximizer() {
            // network's turn now. Implies popping CCA's move.
            debug!("Pop move: {:?}", self.move_cca);
            self.move_cca = None;
            self.move_choices_network_stack.pop();
            // After a CCA's make_move, we compute next move choices for the
            // network, so we pop those.
        } else {
            let ret = self.history_stack.pop();
            let last_history_item = self.get_last_history_item();
            let rate = ret.as_ref().unwrap().get_a() - last_history_item.get_a();
            self.move_cca = Some(CCAAction { rate });
            debug!("Pop move: {:?}", ret);
            self.belief_bounds_stack.pop();
            self.move_choices_cca_stack.pop();
        }
        self.tree_pop_move();
    }

    fn reset_moves(&mut self) {
        // Only reset when all the speculative moves have been popped.
        // Assuming we never start speculation from the network's POV.
        assert!(self.history_stack.is_empty());
        assert!(self.belief_bounds_stack.is_empty());
        assert!(self.move_cca.is_none());
        assert!(self.move_choices_network_stack.is_empty());

        // Moves may have been dequeued from the choices. We need to reset this
        // operation. As the speculation is starting afresh.
        assert!(self.move_choices_cca_stack.len() == 1);
        self.move_choices_cca_stack.pop();
        self.move_choices_cca_stack.push(self.compute_cca_moves());
    }

    fn tt_state(&self) -> CCTTState<NM::O> {
        let mut history = Vec::new();
        for o in self.history.iter() {
            history.push(o.clone());
        }
        history.append(&mut self.history_stack.clone());
        CCTTState {
            history,
            move_cca: self.move_cca,
        }
    }

    fn chosen_move(&self, m: &Option<CCMove<NM::NA>>, v: &CCValue) {
        if self.recording_tree {
            let this = self.current.upgrade().unwrap();
            this.borrow_data_mut().chosen_move = m.clone();
            this.borrow_data_mut().chosen_value = Some(v.clone());
        }
    }
}

impl<NM: NetworkModel> CongCtrlState<NM> {
    fn init(&mut self) {
        for obs in self.network_model.get_initial_history() {
            self.history.push(obs);
        }
    }

    fn get_tree_data(&self) -> TreeData<NM::O, NM::NA> {
        TreeData {
            observation: self.get_last_history_item().clone(),
            cca_rate: if self.move_cca.is_none() {
                None
            } else {
                Some(self.move_cca.unwrap().rate)
            },
            history: self.get_relevant_history(Some(HISTORY_SIZE)),
            belief_bounds: *self.get_latest_belief_bounds(),
            metrics: self.compute_metrics(),
            chosen_move: None,
            chosen_value: None,
        }
    }

    fn start_recording_tree(&mut self) {
        self.recording_tree = true;
        self.root = Some(Rc::new(Tree::new(self.get_tree_data())));
        self.current = Rc::downgrade(self.root.as_ref().unwrap());
    }

    fn clear_tree(&mut self) {
        self.recording_tree = false;
        self.root = None;
        self.current = Weak::new();
    }

    fn tree_make_move(&mut self) {
        if self.recording_tree {
            let child = Rc::new(Tree::new(self.get_tree_data()));
            let w_child = Rc::downgrade(&child);
            Tree::add_child(&self.current, child);
            self.current = w_child;
        }
    }

    fn tree_pop_move(&mut self) {
        if self.recording_tree {
            let this = self.current.upgrade().unwrap();
            let parent = this.parent();
            self.current = parent;
        }
    }

    fn any_move_guess(&mut self) -> Option<CCMove<NM::NA>> {
        if self.maximizer() {
            // network's turn
            self.move_choices_network_stack.last_mut().unwrap().pop()
        } else {
            self.move_choices_cca_stack.last_mut().unwrap().pop()
        }
    }

    fn prepare_speculation(&mut self) {
        let mut bounds = self.compute_belief_bounds();
        if let Some(prior) = self.retained_prior() {
            if intersect_c_b(&mut bounds, &prior) {
                warn!("committed belief {} disagrees with the retained {}; keeping the newer bound there", bounds, prior);
            }
        }
        self.retained = Some((self.qe_ks.clone(), bounds, self.planner.belief_start));
        self.belief_bounds = bounds;
        self.move_choices_cca_stack.push(self.compute_cca_moves());
    }

    fn commit_and_prepare(&mut self) {
        self.commit_move();
        self.prepare_speculation();
    }

    fn commit_move(&mut self) {
        // info!("Committing moves");
        // Used for simulation
        assert!(!self.maximizer()); // CCA's turn
        assert!(self.history_stack.len() == 1);
        assert!(self.belief_bounds_stack.len() == 1);
        assert!(self.move_choices_network_stack.len() == 1);
        assert!(self.move_choices_cca_stack.len() == 2);

        self.history.push(self.history_stack.pop().unwrap());
        self.belief_bounds_stack.pop();
        self.move_choices_network_stack.pop();
        self.move_choices_cca_stack.pop();
        self.move_choices_cca_stack.pop();
    }

    fn compute_metrics(&self) -> Metrics {
        let last = self.get_last_history_item();
        let first = self
            .get_relevant_history(Some(self.policy.belief_history_observations))
            .into_iter()
            .next()
            .expect("QE history must not be empty");

        let qmax1: usize = self.history.iter().map(|o| o.get_qdel()).max().unwrap();
        let qmax2: usize = self
            .history_stack
            .iter()
            .map(|o| o.get_qdel())
            .max()
            .unwrap_or(0);
        let qmax = std::cmp::max(qmax1, qmax2);

        let latest = self.get_latest_belief_bounds();
        let start = &self.belief_bounds;
        let one: RealNumRep = 1.into();

        let mut time_to_shrink_c = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        for (idx, bb) in self.belief_bounds_stack.iter().enumerate() {
            if bb.min_c > start.min_c || bb.max_c < start.max_c {
                time_to_shrink_c = idx as RealNumInt;
                break;
            }
        }

        let mut time_to_shrink_c_l = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        for (idx, bb) in self.belief_bounds_stack.iter().enumerate() {
            //if bb.c_add_gap() < start.c_add_gap() {
            // Need to check min_c is gradually increasing
            if bb.min_c >= start.min_c + one * 2 {
                //if bb.min_c > start.min_c {
                time_to_shrink_c_l = idx as RealNumInt;
                break;
            }
        }

        let mut time_to_shrink_c_h = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        for (idx, bb) in self.belief_bounds_stack.iter().enumerate() {
            //if bb.c_add_gap() < start.c_add_gap() {
            // Need to check min_c is gradually increasing
            if bb.max_c < start.max_c {
                //if bb.min_c > start.min_c {
                time_to_shrink_c_h = idx as RealNumInt;
                break;
            }
        }

        let mut time_to_c = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        for (idx, bb) in self.belief_bounds_stack.iter().enumerate() {
            if bb.min_c == latest.min_c && bb.max_c == latest.max_c {
                time_to_c = idx as RealNumInt;
                break;
            }
        }

        // This should be true when speculation size is of even size
        let minc_minb_sum = self
            .network_model
            .compute_min_c_b_sum(&self.get_relevant_history(Some(HISTORY_SIZE)), &None)
            .unwrap();
        let mut time_to_c_b = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        let relevant_history =
            &self.get_relevant_history(Some(HISTORY_SIZE + self.history_stack.len()));
        for ptr in 0..=relevant_history.len().saturating_sub(HISTORY_SIZE) {
            let rh = &relevant_history[ptr..(ptr + HISTORY_SIZE)];
            if self.network_model.compute_min_c_b_sum(rh, &None).unwrap() >= minc_minb_sum {
                // `ptr == 0` is the current history, i.e. zero RTTs away.
                time_to_c_b = ptr as RealNumInt;
                break;
            }
        }
        let start_minc_minb_sum = self
            .network_model
            .compute_min_c_b_sum(&relevant_history[0..HISTORY_SIZE], &None)
            .unwrap();
        let mut time_to_shrink_minc_minb_add = (HISTORY_SIZE + SPECULATION_SIZE + 1) as RealNumInt;
        for ptr in 0..=relevant_history.len().saturating_sub(HISTORY_SIZE) {
            let rh = &relevant_history[ptr..(ptr + HISTORY_SIZE)];
            // Account for unsoundness during QE
            if self.network_model.compute_min_c_b_sum(rh, &None).unwrap()
                >= start_minc_minb_sum + self.policy.loss_tolerance_abs
                    - RealNumRep::new_raw(1, 1 << 4)
            {
                time_to_shrink_minc_minb_add = ptr as RealNumInt;
                break;
            }
        }

        let steps = self.history.len() as RealNumInt + self.history_stack.len() as RealNumInt - 1;

        //let steady_state = self.history.get(HISTORY_SIZE - 1).unwrap().get_ld() > 0.into();
        let steady_state = self
            .history
            .get(HISTORY_SIZE - 1)
            .unwrap()
            .get_lo()
            .last()
            .unwrap()
            .0
            >= 2;
        //let steady_state = self.belief_bounds.max_c < LARGEST_BW.into();
        let loss_tol = if steady_state {
            (last.get_ld() - first.get_ld()) <= self.policy.loss_tolerance_abs * 2
        } else {
            (last.get_ld() - first.get_ld()) <= self.policy.loss_tolerance_abs
        };

        Metrics {
            neg_delivered: (first.get_s()) - (last.get_s()),
            neg_sent: first.get_a() - last.get_a(),
            qdel: std::cmp::max(0.into(), (qmax as i32 - 1).into()),
            loss: std::cmp::max(
                0.into(),
                last.get_ld() - first.get_ld() - self.policy.loss_tolerance_abs,
            ),
            total_loss: std::cmp::max(0.into(), last.get_ld() - first.get_ld()),
            // Basically is latest smaller than start:
            //shrink_c: latest.c_add_gap() < start.c_add_gap(),
            shrink_c: latest.min_c > start.min_c || latest.max_c < start.max_c,
            shrink_b: latest.max_b < LARGEST_BW.into(),
            shrink_maxc_mul: latest.max_c <= start.max_c / 2,
            shrink_maxc_add: latest.max_c < LARGEST_BW.into(),
            shrink_minc_mul: latest.min_c >= start.min_c * 2,
            shrink_minc_add: latest.min_c >= start.min_c + one * 2,
            time_to_shrink_c,
            //loss_tol: last.get_ld() <= latest.min_c,
            //loss_tol,
            loss_tol: (last.get_ld() - first.get_ld()) <= self.policy.loss_tolerance_abs * 1,
            time_to_shrink_c_l,
            time_to_shrink_c_h,
            time_to_c,
            minc_minb_sum,
            time_to_c_b,
            time_to_shrink_minc_minb_add,
            shrink_minc_minb: minc_minb_sum >= start_minc_minb_sum,
        }
    }

    // fn get_feasible_moves(&self) -> Vec<CCMove> {
    //     if self.maximizer() {
    //         // network's turn
    //         return self.get_feasible_network_moves();
    //     } else {
    //         return self.get_cca_moves();
    //     }
    // }

    fn get_last_history_item(&self) -> &NM::O {
        if !self.history_stack.is_empty() {
            return self.history_stack.last().unwrap();
        } else {
            return self.history.last().unwrap();
        }
    }

    fn get_relevant_history(&self, size: Option<usize>) -> Vec<NM::O> {
        // We want to build vector with at most FEASIBLE_SIZE-1 observations
        // worth of history. "Minus 1" because that's what we will be binary
        // searching for. We also want as many elements as possible from the
        // history_stack. Basically we copy the last elements from
        // history_stack, if space is left we also copy from history.

        let mut ret: Vec<NM::O> = Vec::new();

        let total = size.unwrap_or(FEASIBLE_SIZE - 1);
        let n_stack = self.history_stack.len();
        let n_committed = self.history.len();

        let mut copy = std::cmp::min(total as i64 - n_stack as i64, n_committed as i64);
        copy = std::cmp::max(copy, 0);
        let start = n_committed - copy as usize;
        for i in start..n_committed {
            ret.push(self.history.get(i).unwrap().clone());
        }

        let copy = std::cmp::min(n_stack, total);
        let start = n_stack - copy;
        // ret.extend(self.history_stack[start..].iter().copied());
        ret.extend(self.history_stack[start..].to_vec().clone());
        ret
    }

    fn compute_feasible_network_moves(&self) -> Vec<CCMove<NM::NA>> {
        assert!(self.maximizer()); // only call when its network's turn.
                                   // let ret = self
                                   //     .network_model
                                   //     .compute_feasible_network_moves(&self.get_relevant_history(None), &self.move_cca);

        // Sending full history to be able to compute belief paths, the compute
        // feasible moves then pops the oldest observation.
        let ret = self
            .network_model
            //.compute_feasible_network_moves(&self.get_relevant_history(Some(FEASIBLE_SIZE + self.history_stack.len())), &self.move_cca);
            .compute_feasible_network_moves(
                &self.get_relevant_history(Some(HISTORY_SIZE + self.history_stack.len())),
                &self.move_cca,
            );
        //ret.into_iter().map(CCMove::Network).unique().collect()
        ret.into_iter().map(CCMove::Network).collect()
    }

    /// The rate of every candidate action the policy allows, in policy order.
    fn candidate_rates(&self) -> Vec<(CandidateAction, RealNumRep)> {
        let belief_bounds = self.get_latest_belief_bounds();
        let needs_max_rate = self.policy.candidate_actions.iter().any(|candidate| {
            matches!(
                candidate,
                CandidateAction::MaxAllowedRate | CandidateAction::ProbeAboveLimit
            )
        });
        // `compute_max_allowed_rate` enumerates network moves, so skip it
        // when only `DrainQueue` is allowed.
        let max_allowed_rate = if needs_max_rate {
            let rh = self.get_relevant_history(Some(QE_WINDOW_OBSERVATIONS));
            self.network_model
                .compute_max_allowed_rate(&rh, &self.move_cca, 0.into(), self.policy.delay_tolerance_frac, 1)
                // The model minimizes `c + b - max_q` over sampled (C, B) of
                // the newest window, which can miss the worst case (lowest C
                // and B); the belief cannot.  For any feasible link,
                // `c + b - q >= min_cb - max_q`, and `>= c >= min_c` since
                // the queue never exceeds the buffer.
                .map(|r| {
                    let min_cb = std::cmp::max(belief_bounds.min_cb, belief_bounds.min_c + belief_bounds.min_b);
                    let safe = std::cmp::max(min_cb - belief_bounds.max_q, belief_bounds.min_c);
                    std::cmp::min(r, safe)
                })
        } else {
            None
        };
        let min_c = belief_bounds.min_c;
        let max_q = belief_bounds.max_q;
        self.policy
            .candidate_actions
            .iter()
            .map(|candidate| {
                let rate = match candidate {
                    CandidateAction::MaxAllowedRate => max_allowed_rate.unwrap(),
                    CandidateAction::DrainQueue => std::cmp::max(0.into(), min_c - max_q),
                    CandidateAction::HoldQueue => std::cmp::max(
                        0.into(),
                        min_c - std::cmp::max(0.into(), max_q - self.policy.queue_target),
                    ),
                    CandidateAction::ProbeAboveLimit => {
                        max_allowed_rate.unwrap() + self.policy.loss_tolerance_abs
                    }
                };
                // Keep the worst-case queue within the cap: at C = min_c the
                // queue grows by `rate - min_c`.
                let rate = match self.policy.queue_cap {
                    Some(cap) => std::cmp::min(rate, std::cmp::max(0.into(), min_c + cap - max_q)),
                    None => rate,
                };
                (*candidate, rate)
            })
            .filter(|(_, rate)| *rate >= 0.into())
            .collect()
    }

    fn compute_cca_moves(&self) -> Vec<CCMove<NM::NA>> {
        assert!(!self.maximizer()); // only call when its CCA's turn.
        let start = std::time::Instant::now();
        let ret = self
            .candidate_rates()
            .into_iter()
            .map(|(_, rate)| CCMove::CCA(CCAAction { rate }))
            .unique()
            .collect();
        // https://stackoverflow.com/questions/47636618/vecdedup-does-not-work-how-do-i-deduplicate-a-vector-of-strings
        debug!("CCA choices:\n{:?}\n in {:.2?} time.", ret, start.elapsed());
        ret
    }

    /// Intersect the C/B bounds of every QE window in the newest
    /// `history_len` observations.  On the first window whose bounds do not
    /// overlap the accumulated ones, a strict call returns the conflict;
    /// a `lenient` one keeps the accumulated (older, multi-window) bound for
    /// that dimension and carries on, counting the conflicts.
    fn fold_belief_windows(
        &self,
        history_len: usize,
        lenient: bool,
    ) -> Result<(BeliefBounds, usize), InconsistentQeWindows> {
        let history = self.get_relevant_history(Some(history_len));
        assert!(history.len() >= QE_WINDOW_OBSERVATIONS);

        let window_count = history.len() - QE_WINDOW_OBSERVATIONS + 1;
        // Observations at index >= n_committed are speculative.
        let n_committed = history.len() - std::cmp::min(self.history_stack.len(), history.len());
        // `None`: no static link explains the window (it straddles a change).
        let window_bounds = |idx: usize, window: &[NM::O]| -> Option<BeliefBounds> {
            // An outstanding CCA action belongs only to the newest window.
            let action = if idx + 1 == window_count {
                &self.move_cca
            } else {
                &None
            };
            if action.is_some() || idx + QE_WINDOW_OBSERVATIONS > n_committed {
                return self.network_model.try_compute_belief_bounds(window, action);
            }
            if let Some(bounds) = self.window_cache.borrow().get(window) {
                return *bounds;
            }
            let bounds = self.network_model.try_compute_belief_bounds(window, action);
            self.window_cache.borrow_mut().insert(window.to_vec(), bounds);
            bounds
        };
        // Until the first explainable window, the committed belief stands in.
        let mut bounds: Option<BeliefBounds> = None;
        let mut conflicts = 0;
        // The windows that contributed, for the joint (C, B) bound below.
        let mut used: Vec<(&[NM::O], &Option<CCAAction>)> = vec![];
        for (idx, window) in history.windows(QE_WINDOW_OBSERVATIONS).enumerate() {
            let action = if idx + 1 == window_count { &self.move_cca } else { &None };
            let Some(next) = window_bounds(idx, window) else {
                if !lenient {
                    return Err(InconsistentQeWindows {
                        history_observations: history.len(),
                        window_index: idx,
                        accumulated: bounds.unwrap_or(self.belief_bounds),
                        next_window: bounds.unwrap_or(self.belief_bounds),
                        infeasible: true,
                    });
                }
                conflicts += 1;
                continue;
            };
            used.push((window, action));
            if bounds.is_none() {
                bounds = Some(next);
                continue;
            }
            let bounds = bounds.as_mut().unwrap();

            // C and B are persistent parameters, so intersect their bounds.
            // Generated QE uses finite discretization, however, and individual
            // windows can occasionally disagree by more than its numerical
            // slack.  Do not turn that approximation artifact into a runtime
            // crash or an empty belief: retain the prior consistent result.
            let mut merged = *bounds;
            if intersect_c_b(&mut merged, &next) {
                if !lenient {
                    return Err(InconsistentQeWindows {
                        history_observations: history.len(),
                        window_index: idx,
                        accumulated: *bounds,
                        next_window: next,
                        infeasible: false,
                    });
                }
                conflicts += 1;
            }
            bounds.min_c = merged.min_c;
            bounds.max_c = merged.max_c;
            bounds.min_b = merged.min_b;
            bounds.max_b = merged.max_b;

            // Q and inflight are state variables at the end of a window, not
            // persistent model parameters.  The newest window is the one
            // aligned with the current state, so keep its values.
            if idx + 1 == window_count {
                bounds.min_q = next.min_q;
                bounds.max_q = next.max_q;
                bounds.min_if = next.min_if;
                bounds.max_if = next.max_if;
            }
        }
        // One (C, B) must explain every window, which the per-window
        // intersection above does not require.  This is still exact (no
        // window is ever dropped for being slow), and the network model
        // caches the committed prefix.
        if let Some(acc) = bounds.as_mut() {
            match self.network_model.joint_c_b_bounds(&used) {
                Some(Some(joint)) => {
                    let crossed = intersect_c_b(acc, &joint);
                    debug_assert!(!crossed || conflicts > 0, "joint bound outside the per-window bounds");
                }
                Some(None) if !lenient => {
                    return Err(InconsistentQeWindows {
                        history_observations: history.len(),
                        window_index: window_count - 1,
                        accumulated: *acc,
                        next_window: *acc,
                        infeasible: false,
                    });
                }
                Some(None) => conflicts += 1,
                None => {}
            }
        }
        Ok((bounds.unwrap_or(self.belief_bounds), conflicts))
    }

    fn try_compute_belief_bounds(&self, history_len: usize) -> Result<BeliefBounds, InconsistentQeWindows> {
        self.fold_belief_windows(history_len, false).map(|(bounds, _)| bounds)
    }

    fn compute_belief_bounds(&self) -> BeliefBounds {
        let history_len = self.policy.belief_history_observations + self.history_stack.len();
        // Conflicting windows keep the older accumulated bound rather than
        // replacing the whole history by the newest window, which would
        // discard everything learned about C and B.
        let (mut bounds, conflicts) = self.fold_belief_windows(history_len, true).unwrap();
        if conflicts > 0 {
            debug!("{} QE window(s) conflicted; kept the accumulated bounds there", conflicts);
        }
        if !self.history_stack.is_empty() {
            // A speculative state is never broader than the committed belief
            // it extends: the K set is fixed for the whole tree.
            intersect_c_b(&mut bounds, &self.belief_bounds);
        }
        bounds
    }

    /// The previous committed belief, if it may still constrain the new one:
    /// it assumed a superset of the current K set (C and B are static, so its
    /// bounds stay sound), and either beliefs use the whole history or an
    /// RTT in the `--belief-history` window was app-limited.  App-limited
    /// RTTs say too little about the link to push out what older RTTs
    /// proved, so the prior is only dropped once the window holds nothing
    /// but busy RTTs.
    fn retained_prior(&self) -> Option<BeliefBounds> {
        let (ks, prior, start) = self.retained.as_ref()?;
        if *start != self.planner.belief_start {
            // The belief restarted because the link changed: the older
            // bounds describe a different link.
            return None;
        }
        let narrowed = match (ks, &self.qe_ks) {
            (Some(old), Some(new)) => new.iter().all(|k| old.contains(k)),
            (old, new) => old.is_none() && new.is_none(),
        };
        let retain = match self.belief_history {
            None => true,
            Some(n) => self.app_limited_rtts.iter().rev().take(n).any(|&a| a),
        };
        (narrowed && retain).then_some(*prior)
    }

    fn get_network_move_sim(&self, args: &Args) -> Option<CCMove<NM::NA>> {
        self.network_model
            .get_network_move_sim(
                &self.get_relevant_history(Some(FEASIBLE_SIZE)),
                &self.move_cca,
                args.sim_ideal,
            )
            .map(CCMove::Network)
    }

    fn get_latest_belief_bounds(&self) -> &BeliefBounds {
        if let Some(ret) = self.belief_bounds_stack.last() {
            ret
        } else {
            &self.belief_bounds
        }
    }

    pub fn new(network_model: NM) -> CongCtrlState<NM> {
        let history = Vec::new();
        let belief_bounds = BeliefBounds::default();
        let move_cca = None;
        let history_stack = Vec::new();
        let belief_bounds_stack = Vec::new();
        let move_choices_cca_stack = Vec::new();
        let move_choices_network_stack = Vec::new();
        CongCtrlState {
            network_model,
            history,
            belief_bounds,
            move_cca,
            history_stack,
            belief_bounds_stack,
            move_choices_cca_stack,
            move_choices_network_stack,
            root: None,
            current: Weak::new(),
            recording_tree: false,
            disable_pre_enumeration: false,
            policy: StatePolicy::default(),
            planner: PlannerState::default(),
            qe_ks: None,
            window_cache: RefCell::new(HashMap::new()),
            retained: None,
            belief_history: None,
            app_limited_rtts: Vec::new(),
            last_action: None,
            n_nodes_explored: 0,
            parallel_binary_search: false,
            // pool: rayon::ThreadPoolBuilder::new()
            //     .num_threads(N_THREADS)
            //     .build()
            //     .unwrap(),
        }
    }
}

impl CongCtrlState<NetworkModelNC> {
    /// Freeze the user-selected policy for the next minimax tree.  This is
    /// intentionally called only between committed control intervals.
    fn select_state_policy(&mut self, obj_perm: &[String], intervals: &PlannerIntervals) {
        let timestep = self.history.len().saturating_sub(QE_WINDOW_OBSERVATIONS);
        if self.qe_ks.is_some() {
            // Test each K on the newest committed window with its own
            // evaluator, so an inconsistent K cannot trip the belief asserts.
            let window = &self.history[self.history.len() - QE_WINDOW_OBSERVATIONS..];
            let feasible = |k: RealNumInt| {
                let mut nm = self.network_model.clone();
                nm.set_qe(std::sync::Arc::new(BurstyQe { k: k.into(), pert: BURSTY_PERT }));
                nm.try_window_belief(window)
            };
            let mut planner = std::mem::take(&mut self.planner);
            let n = self.history.len();
            let delivered = self.history[n - 1].s - self.history[n - 2].s;
            let start_before = planner.belief_start;
            let ks = update_k(&mut planner, timestep, delivered, &intervals.strategy, &feasible);
            if planner.belief_start != start_before {
                // The link changed: what the committed belief says about C and
                // B described the old link.
                self.belief_bounds.min_c = SMALLEST_BW.into();
                self.belief_bounds.max_c = LARGEST_BW.into();
                self.belief_bounds.min_b = SMALLEST_BW.into();
                self.belief_bounds.max_b = LARGEST_BW.into();
            }
            self.planner = planner;
            if self.qe_ks.as_ref() != Some(&ks) {
                info!("t={}: belief uses K in {:?}", timestep, ks);
                self.network_model.set_qe(union_bursty_qe(&ks));
                self.window_cache.borrow_mut().clear();
                self.qe_ks = Some(ks);
            }
        }
        // The planner sees the same retained belief `prepare_speculation`
        // commits, not a fresh one that may be broader.
        self.belief_history = intervals.belief_history;
        let prior = self.retained_prior();
        let retain = |mut bounds: BeliefBounds| {
            if let Some(prior) = &prior {
                intersect_c_b(&mut bounds, prior);
            }
            bounds
        };
        let belief_bounds_for_history = |history_observations| {
            self.try_compute_belief_bounds(history_observations)
                .map(retain)
                .map_err(|conflict| InconsistentQeWindows {
                    accumulated: retain(conflict.accumulated),
                    ..conflict
                })
        };
        let state = StateQuantities {
            timestep,
            history: &self.history,
            previous_belief: self.belief_bounds,
            belief_bounds_for_history: &belief_bounds_for_history,
            probe_interval_rtts: intervals.probe,
            drain_interval_rtts: intervals.drain,
            belief_history: intervals.belief_history,
            strategy: intervals.strategy,
            newest_window_feasible: self
                .network_model
                .try_compute_belief_bounds(&self.history[self.history.len() - QE_WINDOW_OBSERVATIONS..], &None)
                .is_some(),
        };
        let mut planner = self.planner.clone();
        let mut policy = state_policy(&state, &mut planner);
        assert!(
            policy.belief_history_observations >= QE_WINDOW_OBSERVATIONS,
            "belief_history_observations must be at least QE_WINDOW_OBSERVATIONS"
        );
        let requested: HashSet<&str> = obj_perm.iter().map(String::as_str).collect();
        let mut objective_order = obj_perm.to_vec();
        objective_order.extend(
            policy
                .objective_order
                .iter()
                .filter(|name| !requested.contains(name.as_str()))
                .cloned(),
        );
        policy.objective_order = objective_order;
        self.policy = policy;
        self.planner = planner;
    }

    /// What the root of a search knows, for `--decisions`.
    fn decision_features(&self) -> serde_json::Value {
        let f = |x: RealNumRep| x.to_f64().unwrap();
        let bb = self.belief_bounds;
        let first = self
            .get_relevant_history(Some(self.policy.belief_history_observations))
            .into_iter()
            .next()
            .unwrap();
        let last = self.history.last().unwrap();
        serde_json::json!({
            "mode": format!("{:?}", self.policy.mode),
            "loss_tol": f(self.policy.loss_tolerance_abs),
            "history_obs": self.policy.belief_history_observations,
            "n_history": self.history.len(),
            "belief": {
                "min_c": f(bb.min_c), "max_c": f(bb.max_c), "min_b": f(bb.min_b), "max_b": f(bb.max_b),
                "min_q": f(bb.min_q), "max_q": f(bb.max_q), "min_if": f(bb.min_if), "max_if": f(bb.max_if),
            },
            "candidates": self
                .candidate_rates()
                .into_iter()
                .map(|(c, r)| serde_json::json!([format!("{:?}", c), f(r)]))
                .collect::<Vec<_>>(),
            "loss_since_start": f(last.get_ld() - first.get_ld()),
            "ld": f(last.get_ld()),
            "qdel": last.get_qdel(),
            "a": f(last.a),
            "s": f(last.s),
            "last_rate": f(last.a - self.history[self.history.len() - 2].a),
            "last_delivered": f(last.s - self.history[self.history.len() - 2].s),
            "qe_ks": self.qe_ks.clone().unwrap_or_default(),
            "metrics": self.compute_metrics(),
        })
    }

    /// Commit `m` (the CCA move just chosen at the root) to the planner;
    /// the application let `sent` of its rate be sent.
    fn record_cca_move(&mut self, m: &CCMove<NetworkActionNC>, sent: RealNumRep) {
        let CCMove::CCA(a) = m else { return };
        let timestep = self.history.len().saturating_sub(QE_WINDOW_OBSERVATIONS);
        let action = self
            .candidate_rates()
            .into_iter()
            .find(|(_, rate)| *rate == a.rate)
            .map(|(candidate, _)| candidate);
        record_action(&mut self.planner, timestep, action, a.rate, sent);
        self.app_limited_rtts.push(self.planner.app_limited);
        self.last_action = action;
    }
}

/// `--probe-interval` / `--drain-interval`, passed to the planner.
struct PlannerIntervals {
    probe: usize,
    drain: usize,
    belief_history: Option<usize>,
    strategy: BeliefStrategy,
}

/// Intersect the persistent C and B bounds of `next` into `acc`, one
/// dimension at a time.  A dimension whose intervals do not overlap (QE
/// discretization) keeps `acc`'s interval.  Returns whether one crossed.
/// Q and inflight are state, not parameters, and are left alone.
fn intersect_c_b(acc: &mut BeliefBounds, next: &BeliefBounds) -> bool {
    let mut crossed = false;
    let min_c = std::cmp::max(acc.min_c, next.min_c);
    let max_c = std::cmp::min(acc.max_c, next.max_c);
    if min_c <= max_c {
        (acc.min_c, acc.max_c) = (min_c, max_c);
    } else {
        crossed = true;
    }
    // Each is a lower bound on the same link's C + B.
    acc.min_cb = std::cmp::max(acc.min_cb, next.min_cb);
    let min_b = std::cmp::max(acc.min_b, next.min_b);
    let max_b = std::cmp::min(acc.max_b, next.max_b);
    if min_b <= max_b {
        (acc.min_b, acc.max_b) = (min_b, max_b);
    } else {
        crossed = true;
    }
    crossed
}

fn union_bursty_qe(ks: &[RealNumInt]) -> std::sync::Arc<UnionBurstyQe> {
    std::sync::Arc::new(UnionBurstyQe {
        ks: ks.iter().map(|&k| k.into()).collect(),
        pert: BURSTY_PERT,
    })
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // https://docs.rs/clap/latest/clap/_derive/_cookbook/typed_derive/index.html
    #[arg(short, long)]
    csv: bool,

    #[arg(long, default_value_t = true)]
    sim: bool,
    #[arg(long, default_value_t=20.into())]
    sim_c: RealNumRep,
    #[arg(long, default_value_t=10.into())]
    sim_b: RealNumRep,
    #[arg(long)]
    sim_ideal: bool,

    #[arg(long)]
    tree: bool,

    #[arg(long, num_args=0..)]
    obj_perm: Vec<String>,

    #[arg(short, long)]
    out: Option<String>,

    #[arg(long)]
    sim_max_rate: bool,
    #[arg(long, default_value_t = 1)]
    probe_duration: u32,
    #[arg(long, default_value_t=(2 * ALPHA).into())]
    loss_tolerance_abs: RealNumRep,
    #[arg(long, default_value_t=3.into())]
    delay_tolerance_frac: RealNumRep,

    #[arg(long)]
    parallel: bool,

    /// Number of control intervals to execute. Keep this small while
    /// iterating on objectives/actions; the minimax search grows quickly.
    #[arg(long, default_value_t = 4)]
    steps: u32,

    /// Generated QE evaluator; defaults to `QE_MODEL` in the config.
    #[arg(long, value_enum)]
    qe_model: Option<QeModel>,

    /// Bursty model: the simulated link's burstiness, independent of the
    /// K the CCA assumes.  It only bounds what the simulated adversary may
    /// do; `--sim-policy` decides whether it bursts.
    #[arg(long, default_value_t = 1)]
    sim_k: RealNumInt,

    /// How the simulated link picks each response.
    #[arg(long, value_enum, default_value_t = SimChoice::Max)]
    sim_policy: SimChoice,

    /// Seed for `random` and `mixed` simulation policies.
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// `mixed`: RTTs between switches of the simulation policy.
    #[arg(long, default_value_t = 10)]
    mixed_period: u32,

    /// Override `PROBE_INTERVAL_RTTS` in the config.
    #[arg(long)]
    probe_interval: Option<usize>,

    /// Override `DRAIN_INTERVAL_RTTS` in the config.
    #[arg(long)]
    drain_interval: Option<usize>,

    /// Observations whose overlapping QE windows are intersected for the
    /// belief (default: the whole history).
    #[arg(long)]
    belief_history: Option<usize>,

    /// Simulated application: `backlogged`, `rate:X` (X pkts/RTT into an
    /// app buffer), `onoff:ON:OFF` (backlogged for ON RTTs, idle for OFF) or
    /// `random` (supply uniform in [0, 2 sim_c) each RTT, buffered).  The CCA
    /// sends `min(its rate, buffered app data)`.
    #[arg(long, default_value = "backlogged")]
    app: AppModel,

    /// Time-varying link: `T:C,T:C,...` sets the simulated C from RTT T on.
    #[arg(long, value_parser = parse_schedule)]
    sim_c_schedule: Option<Schedule>,

    /// Belief strategy (`BeliefStrategy`): RTTs to confirm the smallest K.
    #[arg(long)]
    k_confirm: Option<usize>,
    /// Assume the smallest alive K from the start.
    #[arg(long)]
    k_optimistic: bool,
    /// Restart the belief when the evidence says the link changed.
    #[arg(long)]
    change_restart: bool,
    /// Let minimax choose (with `qdel`) instead of forcing drains in Explore.
    #[arg(long)]
    explore_free: bool,
    /// Override `EXPLORE_BUDGET_BDP`: an Explore probe's loss budget, in
    /// BDPs (`min_c`), e.g. 0.5.
    #[arg(long)]
    explore_budget: Option<f64>,
    /// Override `EXPLORE_MAX_QUEUE_BDP`, e.g. 3.
    #[arg(long)]
    explore_queue: Option<f64>,
    /// Only the newest N RTTs bound C from above for the planner.
    #[arg(long)]
    c_upper_rtts: Option<usize>,
    /// Write one JSON line per searched RTT here: the root's belief,
    /// candidates and metrics, and every root move's value as the search
    /// compared it.  Used to distil the search into rules.
    #[arg(long)]
    decisions: Option<String>,
    /// Search every root move with a full alpha-beta window, so the
    /// `--decisions` root values are exact and ties are real ties.
    #[arg(long)]
    exact_root: bool,
    /// Override `SEARCH_DEPTH` (plies; must be even).
    #[arg(long)]
    depth: Option<u16>,
    /// The link is outside this process (e.g. a packet simulator): each RTT
    /// print `rate R` on stdout, then read the RTT's outcome from stdin as
    /// `SENT S APP_LIMITED [ROUND:L ...]`: what was sent, the cumulative
    /// service S, whether the application held the sender back (0/1), and
    /// the cumulative loss L up to each sending round (0 is the first RTT,
    /// -1 the empty initial history) that became observable.  Runs until
    /// stdin closes; `--steps` is ignored.
    #[arg(long)]
    external: bool,
    /// `--external`: append every line read from stdin to this file (replay it with
    /// `network_model_nc/examples/window_check.rs`).
    #[arg(long)]
    record: Option<String>,
    /// Never search: take `explore_rule`'s choice (the search's tie-break).
    #[arg(long)]
    explore_rule: bool,
    /// Take `explore_rule`'s choice only where it is known to be the search's, and search
    /// elsewhere.
    #[arg(long)]
    explore_rule_exact: bool,
    /// Search every RTT and compare `explore_rule` with it (CSV column `rule`: `ok` or
    /// `MISMATCH`, suffixed `-known` where the rule claims to know the search's choice).
    #[arg(long)]
    check_rule: bool,
}

/// `--external`: one RTT's outcome, as read from stdin.
struct ExternalObs {
    sent: RealNumRep,
    s: RealNumRep,
    app_limited: bool,
    losses: Vec<(i64, RealNumRep)>,
}

/// Values arrive as decimals; the QE evaluators need small denominators, so
/// round to the 1/16 grid the link simulator also uses.
fn parse_real(x: &str) -> Result<RealNumRep, String> {
    let v: f64 = x.parse().map_err(|e| format!("{x}: {e}"))?;
    Ok(RealNumRep::new((v * 16.).round() as RealNumInt, 16))
}

fn parse_external(line: &str) -> Result<ExternalObs, String> {
    let mut it = line.split_whitespace();
    let mut next = || it.next().ok_or_else(|| format!("short line: {line}"));
    let sent = parse_real(next()?)?;
    let s = parse_real(next()?)?;
    let app_limited = next()? == "1";
    let losses = it
        .map(|part| {
            let (r, l) = part.split_once(':').ok_or(format!("{part}: expected ROUND:L"))?;
            Ok((r.parse::<i64>().map_err(|e| format!("{part}: {e}"))?, parse_real(l)?))
        })
        .collect::<Result<_, String>>()?;
    Ok(ExternalObs { sent, s, app_limited, losses })
}

/// `--sim-c-schedule`: (from RTT, link rate) pairs.
#[derive(Clone, Debug)]
struct Schedule(Vec<(u32, RealNumRep)>);

fn parse_schedule(s: &str) -> Result<Schedule, String> {
    s.split(',')
        .map(|part| {
            let (t, c) = part.split_once(':').ok_or(format!("{part}: expected T:C"))?;
            let t = t.parse::<u32>().map_err(|e| format!("{part}: {e}"))?;
            let c = c.parse::<RealNumInt>().map_err(|e| format!("{part}: {e}"))?;
            Ok((t, c.into()))
        })
        .collect::<Result<_, _>>()
        .map(Schedule)
}

/// `--app`: what the application gives the CCA to send.
#[derive(Clone, Copy, Debug, PartialEq)]
enum AppModel {
    Backlogged,
    Rate(RealNumRep),
    OnOff(u32, u32),
    Random,
}

impl std::str::FromStr for AppModel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split(':').collect();
        let num = |x: &str| x.parse::<u32>().map_err(|e| format!("{s}: {e}"));
        match parts.as_slice() {
            ["backlogged"] => Ok(Self::Backlogged),
            ["random"] => Ok(Self::Random),
            ["rate", x] => Ok(Self::Rate((num(x)? as RealNumInt).into())),
            ["onoff", on, off] => Ok(Self::OnOff(num(on)?, num(off)?)),
            _ => Err(format!("unknown --app {s}")),
        }
    }
}

impl Display for AppModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backlogged => write!(f, "backlogged"),
            Self::Rate(x) => write!(f, "rate{}", x),
            Self::OnOff(on, off) => write!(f, "onoff{}-{}", on, off),
            Self::Random => write!(f, "random"),
        }
    }
}

/// The simulated application's buffer of data waiting to be sent.
struct AppSource {
    model: AppModel,
    buffered: RealNumRep,
    /// Cap on `buffered`, so a long idle CCA does not build an unbounded
    /// backlog (an app producing data at a fixed rate keeps only so much).
    cap: RealNumRep,
}

impl AppSource {
    fn new(model: AppModel, sim_c: RealNumRep) -> Self {
        let cap = match model {
            AppModel::Rate(x) => x * 2,
            _ => sim_c * 4,
        };
        Self { model, buffered: 0.into(), cap }
    }

    /// How much of `intended` the application lets the CCA send at RTT `t`.
    fn send(&mut self, t: u32, intended: RealNumRep, sim_c: RealNumRep, rng: &mut SimRng) -> RealNumRep {
        let supply = match self.model {
            AppModel::Backlogged => return intended,
            AppModel::OnOff(on, off) => {
                if t % (on + off) < on {
                    return intended;
                }
                0.into()
            }
            AppModel::Rate(x) => x,
            AppModel::Random => rng.fraction() * sim_c * 2,
        };
        self.buffered = std::cmp::min(self.buffered + supply, self.cap);
        let sent = std::cmp::min(intended, self.buffered);
        self.buffered = self.buffered - sent;
        sent
    }
}

/// `--sim-policy`.  With the bursty model, `max`/`min`/`random`/`alternate`
/// drive `BurstyLinkSim`, a forward simulation of the true (C, B, K) link:
/// never burst and serve as much as possible / always burst by K and serve
/// as little as possible / random / burst every other RTT.  `adversary`
/// replays the network move minimax expected (feasible for the CCA's belief,
/// so not tied to the true C/B).  `mixed` switches among the four link
/// policies every `--mixed-period` RTTs.  The plain model keeps the original
/// QE-based simulator (`max` only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum SimChoice {
    Max,
    Min,
    Random,
    Alternate,
    Adversary,
    Mixed,
}

/// A CLI fraction as an exact rational on a 1/64 grid.
fn frac(x: f64) -> RealNumRep {
    RealNumRep::new((x * 64.0).round() as RealNumInt, 64)
}

/// xorshift64*: deterministic simulation randomness without a dependency.
struct SimRng(u64);

impl SimRng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// A fraction in [0, 1) on a 1/16 grid.
    fn fraction(&mut self) -> RealNumRep {
        RealNumRep::new((self.next() >> 60) as RealNumInt, 16)
    }
}

fn get_sim_tag(args: &Args) -> String {
    let mut ret = String::new();
    ret.push_str("net");
    if args.sim {
        if args.sim_ideal {
            ret.push_str("_ideal");
        } else {
            ret.push_str("_cbrd");
        }
        ret.push_str(&format!("_c{}_b{}", args.sim_c, args.sim_b));
    }
    if args.qe_model.unwrap_or(QE_MODEL) == QeModel::Bursty {
        // Ratios print as "1/2"; keep the tag a single path component.
        let assumed = match K_POLICY {
            KPolicy::Fixed(k) => format!("{}", k),
            KPolicy::Hypotheses(ks) => ks.iter().map(|k| k.to_string()).collect::<Vec<_>>().join("-"),
        };
        let tag = format!("_bursty_k{}_pert{}_simk{}", assumed, BURSTY_PERT, args.sim_k);
        ret.push_str(&tag.replace('/', "over"));
    }
    ret.push_str(&format!("_{:?}", args.sim_policy).to_lowercase());
    if matches!(args.sim_policy, SimChoice::Random | SimChoice::Mixed) {
        ret.push_str(&format!("_seed{}", args.seed));
    }
    if let Some(n) = args.belief_history {
        ret.push_str(&format!("_bh{}", n));
    }
    if args.app != AppModel::Backlogged {
        ret.push_str(&format!("_app{}", args.app));
    }
    if let Some(Schedule(sched)) = &args.sim_c_schedule {
        let parts: Vec<String> = sched.iter().map(|(t, c)| format!("{t}-{c}")).collect();
        ret.push_str(&format!("_sched{}", parts.join("-")));
    }
    if let Some(n) = args.c_upper_rtts {
        ret.push_str(&format!("_cup{n}"));
    }
    if let Some(k) = args.k_confirm {
        ret.push_str(&format!("_kc{k}"));
    }
    for (on, name) in [(args.k_optimistic, "kopt"), (args.change_restart, "chg"), (args.explore_free, "xfree")] {
        if on {
            ret.push_str(&format!("_{name}"));
        }
    }
    ret
}

fn ks_string(ks: &[RealNumInt]) -> String {
    ks.iter().map(|k| k.to_string()).collect::<Vec<_>>().join("|")
}

fn get_obj_tag(args: &Args) -> String {
    let mut ret = String::new();
    ret.push_str("obj");
    let mut objectives = args.obj_perm.clone();
    objectives.extend(StatePolicy::default().objective_order);
    for obj in objectives {
        ret.push_str(&format!("_{}", obj));
    }
    ret
}

struct SimStrategy<'a> {
    sim_state: u32,
    args: &'a Args,
}

impl SimStrategy<'_> {
    fn new(args: &Args) -> SimStrategy {
        SimStrategy { sim_state: 1, args }
    }

    fn get_rate<NM: NetworkModel>(&mut self, ccs: &CongCtrlState<NM>) -> Option<CCMove<NM::NA>> {
        let bb = ccs.get_latest_belief_bounds();
        let history = ccs.get_relevant_history(Some(HISTORY_SIZE));
        let move_cca = ccs.move_cca;

        if bb.max_q <= 1.into() {
            self.sim_state = self.args.probe_duration;
        }

        if self.sim_state == 0 {
            Some(CCMove::CCA(CCAAction {
                rate: std::cmp::max(bb.min_c - bb.max_q, 0.into()),
            }))
        } else {
            //unimplemented!("Need to re-implement max allowed rate computation");
            let this_loss_abs = self.args.loss_tolerance_abs;
            let ret = ccs.network_model.compute_max_allowed_rate(
                &history,
                &move_cca,
                0.into(),
                self.args.delay_tolerance_frac,
                self.sim_state,
            );
            if self.sim_state == self.args.probe_duration {
                self.sim_state -= 1;
                ret.map(|x| {
                    CCMove::CCA(CCAAction {
                        rate: x + this_loss_abs,
                    })
                })
            } else {
                self.sim_state -= 1;
                ret.map(|x| CCMove::CCA(CCAAction { rate: x }))
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This source is built as both `cc` and `cbrdelay-cca`; filtering only
    // `cc=info` hid every progress line from the latter binary.  Use a
    // program-specific variable so an unrelated shell RUST_LOG=warn does not
    // hide the CCA's progress.  Set SYNTRA_LOG=debug or =trace when needed.
    env_logger::Builder::from_env(Env::new().filter_or("SYNTRA_LOG", "info")).init();
    // env_logger::Builder::from_env(Env::default().default_filter_or("cc=info,network_model_nc=info")).init();

    let args = Args::parse();
    info!("Using {:?}", args);

    let sim_tag = get_sim_tag(&args);
    let obj_tag = get_obj_tag(&args);
    info!("obj_tag: {}", obj_tag);
    info!("sim_tag: {}", sim_tag);

    let exp_dir = if args.out.is_some() {
        let dir = Path::new(args.out.as_ref().unwrap()).join(obj_tag);
        std::fs::create_dir_all(&dir)?;
        Some(dir)
    } else {
        None
    };
    let tree_dir = if exp_dir.is_some() {
        let dir = exp_dir.as_ref().unwrap().join(format!("tree_{}", sim_tag));
        if args.tree {
            std::fs::create_dir_all(&dir)?;
        }
        Some(dir)
    } else {
        None
    };

    let mut some_wtr = None;
    if args.csv {
        let csv_path = exp_dir.as_ref().unwrap().join(format!("{}.csv", sim_tag));
        let mut wtr = csv::Writer::from_path(csv_path).unwrap();
        wtr.write_record([
            "time", "r", "a", "ld", "s", "qdel", "min_c", "max_c", "min_b", "max_b", "min_q",
            "max_q", "min_if", "max_if", "mode", "action", "loss_tol", "sim", "qe_ks", "alive_ks",
            "sent", "app_limited", "candidates", "true_c", "belief_start", "restarts", "searched",
            "rule",
        ])?;
        some_wtr = Some(wtr);
    }

    // let ctx = z3::Context::new(&z3::Config::new());
    // let mut network_model = NetworkModelZ3::new(&ctx, args.parallel);

    let mut network_model = if APP_IS_BACKLOGGED {
        NetworkModelNC::new()
    } else {
        NetworkModelNC::new_with_app_send_choices(APP_SEND_CHOICES)
    };
    let qe_model = args.qe_model.unwrap_or(QE_MODEL);
    info!("QE model: {:?}", qe_model);
    if qe_model == QeModel::Bursty {
        // The CCA's K set is chosen by `select_state_policy`; the simulated
        // link keeps its own K.
        network_model = network_model
            .with_qe(union_bursty_qe(&[args.sim_k]))
            .with_sim_qe(std::sync::Arc::new(BurstyQe {
                k: args.sim_k.into(),
                pert: BURSTY_PERT,
            }));
    }
    let intervals = PlannerIntervals {
        probe: args.probe_interval.unwrap_or(PROBE_INTERVAL_RTTS),
        drain: args.drain_interval.unwrap_or(DRAIN_INTERVAL_RTTS),
        belief_history: args.belief_history,
        strategy: BeliefStrategy {
            k_confirm_rtts: args.k_confirm.unwrap_or(K_CONFIRM_RTTS),
            k_optimistic: args.k_optimistic,
            change_restart: args.change_restart,
            explore_free: args.explore_free,
            c_upper_rtts: args.c_upper_rtts,
            explore_budget_bdp: args.explore_budget.map_or(EXPLORE_BUDGET_BDP, frac),
            explore_queue_bdp: args.explore_queue.map_or(EXPLORE_MAX_QUEUE_BDP, frac),
        },
    };

    network_model.init();
    let mut ccs = CongCtrlState::new(network_model);
    if qe_model == QeModel::Bursty {
        ccs.qe_ks = Some(vec![]);
    }
    ccs.init();
    ccs.select_state_policy(&args.obj_perm, &intervals);
    ccs.prepare_speculation();
    let mut search: minimax::Search<CongCtrlState<NetworkModelNC>> = minimax::Search::new();
    // cong_ctrl_state.best_move_guess(tt);
    ccs.parallel_binary_search = args.parallel;

    if args.sim {
        ccs.network_model
            .setup_network_move_sim(args.sim_ideal, args.sim_c, args.sim_b);
    }

    let mut sim_strategy = None;
    if args.sim_max_rate {
        ccs.disable_pre_enumeration = true;
        sim_strategy = Some(SimStrategy::new(&args));
    }

    let mut rng = SimRng(args.seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let mut link_sim = (qe_model == QeModel::Bursty && args.sim).then(|| {
        BurstyLinkSim::new(args.sim_c, args.sim_b, args.sim_k.into(), ccs.history.len())
    });
    let mut mixed_choice = SimChoice::Max;
    let mut app = AppSource::new(args.app, args.sim_c);
    let mut decisions = match &args.decisions {
        Some(path) => Some(std::io::BufWriter::new(File::create(path)?)),
        None => None,
    };
    let tsteps = if args.external { u32::MAX } else { args.steps };
    let mut true_c = args.sim_c;
    // `--external`: the history index of the first RTT, and the newest loss
    // reported so far (the observation repeats it when nothing is new).
    let n0 = ccs.history.len() as i64;
    let mut last_loss: (i64, RealNumRep) = (n0 - 1, 0.into());
    let mut stdin = std::io::stdin().lock();
    let mut record = match &args.record {
        Some(path) => Some(std::io::BufWriter::new(File::create(path)?)),
        None => None,
    };
    for t in 0..tsteps {
        let change = args.sim_c_schedule.as_ref().and_then(|Schedule(s)| s.iter().find(|(at, _)| *at == t));
        if let Some((_, c)) = change {
            true_c = *c;
            if let Some(sim) = link_sim.as_mut() {
                sim.c = true_c;
            }
            info!("t={}: simulated link rate is now {}", t, true_c);
        }
        let mode = ccs.policy.mode;
        let loss_tol = ccs.policy.loss_tolerance_abs;
        info!("Step {}: searching...", t);
        let start = std::time::Instant::now();
        if args.tree {
            ccs.start_recording_tree();
        }

        let root_moves = ccs.move_choices_cca_stack.last().unwrap().clone();
        let root_rates: Vec<RealNumRep> = root_moves
            .iter()
            .map(|m| match m {
                CCMove::CCA(a) => a.rate,
                _ => panic!("Expecting CCA move"),
            })
            .collect();
        let (rule_rate, rule_known) = explore_rule(&root_rates, &ccs.belief_bounds);
        let use_rule = args.explore_rule || (args.explore_rule_exact && rule_known);
        let rule = use_rule.then_some(rule_rate);
        let searched = !args.sim_max_rate && root_moves.len() > 1 && !use_rule;
        let best_cc_move = if args.sim_max_rate {
            sim_strategy.as_mut().unwrap().get_rate(&ccs)
        } else if !searched {
            // The planner left a single move, or `explore_rule` knows the search's choice.
            match rule {
                Some(rate) => Some(CCMove::CCA(CCAAction { rate })),
                None => root_moves.first().cloned(),
            }
        } else {
            let root = decisions.is_some().then(|| ccs.decision_features());
            // A fresh table per decision: entries hold alpha-beta bounds as
            // if exact, and values depend on this RTT's policy and root.
            search = minimax::Search::new();
            search.exact_root = args.exact_root;
            let (ret, _, _) = search.best_move(&mut ccs, Some(args.depth.unwrap_or(SEARCH_DEPTH)), None);
            if let (Some(w), Some(mut root)) = (decisions.as_mut(), root) {
                let rate = |m: &CCMove<NetworkActionNC>| match m {
                    CCMove::CCA(a) => a.rate.to_f64().unwrap(),
                    _ => f64::NAN,
                };
                root["t"] = t.into();
                root["chosen_rate"] = rate(ret.as_ref().unwrap()).into();
                root["root_values"] = search
                    .root_values
                    .iter()
                    .map(|(m, v)| serde_json::json!({"rate": rate(m), "value": v}))
                    .collect();
                use std::io::Write;
                writeln!(w, "{}", root)?;
                w.flush()?;
            }
            ret
        };
        let rule_check = match (args.check_rule && searched, best_cc_move.as_ref()) {
            (true, Some(CCMove::CCA(a))) => {
                let verdict = if a.rate == rule_rate { "ok" } else { "MISMATCH" };
                if a.rate != rule_rate {
                    warn!("t={}: explore_rule chose {} but the search chose {}", t, rule_rate, a.rate);
                }
                format!("{}{}", verdict, if rule_known { "-known" } else { "" })
            }
            _ => "-".to_string(),
        };
        // The CCA chose a rate; the application decides how much of it is
        // sent.  Only what is sent reaches the link and the QE history.
        let intended = match best_cc_move.as_ref().unwrap() {
            CCMove::CCA(a) => a.rate,
            _ => panic!("Expecting CCA move"),
        };
        let external = if args.external {
            use std::io::{BufRead, Write};
            let mut out = std::io::stdout().lock();
            writeln!(out, "rate {}", intended.to_f64().unwrap())?;
            out.flush()?;
            let mut line = String::new();
            if stdin.read_line(&mut line)? == 0 {
                break;
            }
            if let Some(w) = record.as_mut() {
                write!(w, "{}", line)?;
                w.flush()?;
            }
            Some(parse_external(&line)?)
        } else {
            None
        };
        let sent = match &external {
            Some(obs) => obs.sent,
            None => app.send(t, intended, args.sim_c, &mut rng),
        };
        // A sender that sends whole packets misses a fractional rate by
        // less than a packet; only the application can make it app-limited.
        let planner_sent = match &external {
            Some(obs) if !obs.app_limited => intended,
            _ => sent,
        };
        ccs.record_cca_move(best_cc_move.as_ref().unwrap(), planner_sent);
        let best_cc_move = Some(CCMove::CCA(CCAAction { rate: sent }));

        if args.tree {
            let mut file =
                File::create(tree_dir.as_ref().unwrap().join(format!("{t}.json"))).unwrap();
            serde_json::to_writer(&mut file, &ccs.root).unwrap();
            ccs.clear_tree();
        }
        // info!(
        //     "Got best move: {:} in {:.2?} time.",
        //     best_cc_move.unwrap(),
        //     start.elapsed()
        // );
        // ccs.reset_moves();
        ccs.make_move(best_cc_move.as_ref().unwrap());
        if args.sim_policy == SimChoice::Mixed && t % args.mixed_period == 0 {
            // The adversary is excluded: its replies would desynchronize the
            // link simulator's hidden state from the trace.
            let choices = [SimChoice::Max, SimChoice::Min, SimChoice::Random, SimChoice::Alternate];
            mixed_choice = choices[(rng.next() % choices.len() as u64) as usize];
        }
        let sim_choice = if args.sim_policy == SimChoice::Mixed {
            mixed_choice
        } else {
            args.sim_policy
        };
        let (network_move, sim_used) = if let Some(obs) = &external {
            // History index of the observation being committed.
            let now = ccs.history.len() as i64;
            let mut lo: Vec<LossObservation> = obs
                .losses
                .iter()
                .map(|&(round, l)| {
                    let tau = n0 + round;
                    assert!(tau < now, "loss reported for round {round}, which has not been served");
                    last_loss = (tau, l);
                    LossObservation { t: (now - 1 - tau) as usize, l }
                })
                .collect();
            if lo.is_empty() {
                lo.push(LossObservation { t: (now - 1 - last_loss.0) as usize, l: last_loss.1 });
            }
            let na = NetworkActionNC { s: obs.s, lo, app_send_fraction: network_model_nc::AppSendFraction::Full };
            (Some(CCMove::Network(na)), SimChoice::Max)
        } else if !args.sim {
            (ccs.any_move_guess(), sim_choice)
        } else {
            // The adversary's reply to the chosen move, as minimax evaluated it.
            let adversary_move = if searched {
                match search.tt().get(&ccs.tt_state()) {
                    Some((Some(CCMove::Network(na)), _, _)) => Some(CCMove::Network(na.clone())),
                    _ => None,
                }
            } else {
                None
            };
            let mut link = |choice: SimChoice| -> Option<CCMove<NetworkActionNC>> {
                let Some(sim) = link_sim.as_mut() else {
                    // Plain model: the original QE-based simulator.
                    return ccs.get_network_move_sim(&args);
                };
                let (zero, one, half) = (RealNumRep::from(0), RealNumRep::from(1), RealNumRep::new(1, 2));
                let link_choice = match choice {
                    SimChoice::Max => LinkChoice { burst: zero, service: one },
                    SimChoice::Min => LinkChoice { burst: one, service: zero },
                    SimChoice::Alternate => LinkChoice { burst: ((t % 2) as RealNumInt).into(), service: half },
                    _ => LinkChoice { burst: rng.fraction(), service: rng.fraction() },
                };
                Some(CCMove::Network(sim.step(
                    ccs.history.last().unwrap().a,
                    ccs.history.len(),
                    link_choice,
                )))
            };
            match sim_choice {
                // Without a search there is no expected reply; the worst
                // case the true link allows is the closest stand-in.
                SimChoice::Adversary | SimChoice::Mixed => match adversary_move {
                    Some(m) => (Some(m), SimChoice::Adversary),
                    None => (link(SimChoice::Min), SimChoice::Min),
                },
                choice => (link(choice), choice),
            }
        };
        ccs.make_move(network_move.as_ref().unwrap());
        ccs.commit_move();
        ccs.select_state_policy(&args.obj_perm, &intervals);
        ccs.prepare_speculation();
        let bb = ccs.belief_bounds;
        let last_obs = ccs.history.last().unwrap();
        info!(
            "Step {} ({:.2?}): {}. {}. {}. nodes={}.",
            t,
            start.elapsed(),
            best_cc_move.as_ref().unwrap(),
            last_obs,
            bb,
            ccs.n_nodes_explored + 1, // +1 as we only count make_move calls which does not include the root node.
        );
        ccs.n_nodes_explored = 0;

        if some_wtr.is_some() {
            let wtr = some_wtr.as_mut().unwrap();
            let rate = intended.to_f64().unwrap();
            wtr.write_record(&[
                format!("{:.2?}", start.elapsed()),
                format!("{:.2}", rate),
                format!("{:.2}", last_obs.a.to_f64().unwrap()),
                format!("{:.2}", last_obs.get_ld().to_f64().unwrap()),
                format!("{:.2}", last_obs.s.to_f64().unwrap()),
                format!("{}", last_obs.get_qdel()),
                format!("{:.2}", bb.min_c.to_f64().unwrap()),
                format!("{:.2}", bb.max_c.to_f64().unwrap()),
                format!("{:.2}", bb.min_b.to_f64().unwrap()),
                format!("{:.2}", bb.max_b.to_f64().unwrap()),
                format!("{:.2}", bb.min_q.to_f64().unwrap()),
                format!("{:.2}", bb.max_q.to_f64().unwrap()),
                format!("{:.2}", bb.min_if.to_f64().unwrap()),
                format!("{:.2}", bb.max_if.to_f64().unwrap()),
                format!("{:?}", mode),
                ccs.last_action.map_or("none".to_string(), |a| format!("{:?}", a)),
                format!("{:.2}", loss_tol.to_f64().unwrap()),
                format!("{:?}", sim_used).to_lowercase(),
                ks_string(ccs.qe_ks.as_deref().unwrap_or(&[])),
                ks_string(
                    &ccs.planner
                        .k
                        .alive
                        .iter()
                        .filter(|(_, alive)| *alive)
                        .map(|(k, _)| *k)
                        .collect::<Vec<_>>(),
                ),
                format!("{:.2}", sent.to_f64().unwrap()),
                format!("{}", ccs.planner.app_limited),
                format!("{}", root_moves.len()),
                format!("{:.2}", true_c.to_f64().unwrap()),
                format!("{}", ccs.planner.belief_start),
                format!("{}", ccs.planner.restarts),
                format!("{}", searched),
                rule_check,
            ])?;
            wtr.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // TODO: add assertions at the end of tests so that we can use as automated
    // tests instead of manually verifying outputs.

    use network_model_z3::ObservationZ3;

    use super::*;

    macro_rules! setup_test {
        ($ccs:ident) => {
            let ctx = z3::Context::new(&z3::Config::new());
            let mut network_model = NetworkModelZ3::new(&ctx, false);
            network_model.init();
            let mut $ccs = CongCtrlState::new(network_model);
            // https://stackoverflow.com/questions/38779591/macro-that-declare-variables-in-rust
        };
    }

    fn add_dummy_history(ccs: &mut CongCtrlState<NetworkModelZ3>) {
        for t in 0..5 {
            ccs.history.push_back(ObservationZ3 {
                a: ((t + 1) * 10).into(),
                s: (t * 10).into(),
                ld: 0.into(),
                qdel: 1.into(),
            });
        }
    }

    #[test]
    fn test_get_belief_bounds() {
        env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();

        setup_test!(ccs);
        add_dummy_history(&mut ccs);
        println!("{:?}", ccs.compute_belief_bounds());
    }

    #[test]
    fn test_compute_feasible_network_moves() {
        env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();

        setup_test!(ccs);
        add_dummy_history(&mut ccs);
        ccs.move_cca = Some(CCAAction { rate: 10.into() });
        let start = std::time::Instant::now();
        println!(
            "Returned: \n{:?}\nin {:.2?} time",
            ccs.compute_feasible_network_moves(),
            start.elapsed()
        );
    }

    #[test]
    fn test_get_cca_moves() {
        env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();

        setup_test!(ccs);
        add_dummy_history(&mut ccs);
        let start = std::time::Instant::now();
        ccs.prepare_speculation();
        println!(
            "Returned: \n{:?}\nin {:?} time",
            ccs.move_choices_cca_stack.last().unwrap(),
            start.elapsed()
        );
    }

    #[test]
    fn test_bookkeeping() {
        env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();

        setup_test!(ccs);
        ccs.init();
        ccs.prepare_speculation();
        let cca_move1 = ccs.any_move_guess();
        ccs.make_move(cca_move1.as_ref().unwrap());

        ccs.pop_move();
        let cca_move2 = ccs.any_move_guess();
        assert_ne!(cca_move1, cca_move2);
        ccs.make_move(cca_move2.as_ref().unwrap());

        let net_move1 = ccs.any_move_guess();
        let net_move2 = ccs.any_move_guess();
        ccs.make_move(net_move1.as_ref().unwrap());
        assert_ne!(net_move1, net_move2);

        ccs.commit_and_prepare();
        assert_eq!(ccs.history.len(), 2);
    }

    #[test]
    fn test_ordering() {
        env_logger::Builder::from_env(Env::default().default_filter_or("cc=trace")).init();

        let m1 = Metrics {
            loss: 0.into(),
            qdel: 0.into(),
            neg_delivered: 0.into(),
            neg_sent: 0.into(),
            shrink_c: true,
            shrink_b: true,
            shrink_maxc_mul: true,
            shrink_maxc_add: true,
            shrink_minc_add: true,
            shrink_minc_mul: true,
            time_to_shrink_c: 0,
        };
        let mut m2 = m1;
        println!("{:?}", m1.partial_cmp(&m2));

        let b1 = BeliefBounds::default();
        let b2 = b1;

        let v1 = CCValue::Value {
            metrics: m1,
            belief_bounds: b1,
        };
        m2.neg_delivered = (-1).into();
        let v2 = CCValue::Value {
            metrics: m2,
            belief_bounds: b2,
        };
        println!("{:?}", v1.partial_cmp(&v2));
    }
}
