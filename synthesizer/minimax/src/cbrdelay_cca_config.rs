//! The editable design surface for the CBR-delay CCA experiment.
//!
//! `cargo run -p minimax --bin cbrdelay-cca -- ...` compiles this file into
//! the CCA search.  This file intentionally contains policy choices only;
//! the minimax implementation remains in `cong_ctrl.rs`.  Agents changing
//! this experiment should first read `../CCA_DEVELOPMENT.md`.

use cc_common::BeliefBounds;
use ds::{RealNumInt, RealNumRep};
use log::{info, warn};
use network_model_nc::{AppSendFraction, ObservationNC};

/// Start with the standard saturated-source model: every CCA request is
/// available to send.  Set this to `false` to let the adversary select one of
/// `APP_SEND_CHOICES` on every RTT.
pub const APP_IS_BACKLOGGED: bool = true;

/// Finite approximation used when `APP_IS_BACKLOGGED` is false.  `Full`
/// means the app supplies the entire requested CCA rate; `Half` and `Zero`
/// model an under-running app.  Add choices here if you want a finer model.
pub const APP_SEND_CHOICES: &[AppSendFraction] = &[
    AppSendFraction::Full,
    AppSendFraction::Half,
    AppSendFraction::Zero,
];

/// Which generated QE evaluator models the network.  `Plain` is the
/// original CBR-delay link; `Bursty` adds a sender whose arrivals may be
/// perturbed by up to K bytes.  `--qe-model` overrides this.
pub const QE_MODEL: QeModel = QeModel::Bursty;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum QeModel {
    Plain,
    Bursty,
}

/// Bursty model only: the burstiness K, in the same units as C and B.
/// `Fixed` assumes one K throughout.  `Hypotheses` tracks which of the
/// listed K are still consistent with the observations; see `update_k`.
/// The QE results are only valid when `K < BURSTY_PERT * B`, so each K
/// implies a buffer of more than `K / BURSTY_PERT`.
#[derive(Clone, Copy, Debug)]
pub enum KPolicy {
    Fixed(RealNumInt),
    Hypotheses(&'static [RealNumInt]),
}

pub const K_POLICY: KPolicy = KPolicy::Hypotheses(&[1, 4, 16, 64]);

/// Bursty model only: `pert` in the constraint `K < pert * B`, i.e. how large
/// a burst may be relative to the (unknown) buffer.
pub const BURSTY_PERT: RealNumRep = RealNumRep::new_raw(1, 2);

/// `KPolicy::Hypotheses`: once the smallest alive K has not been ruled out
/// for this many RTTs (since the start or since the last K was ruled out),
/// assume it and drop the larger K from the belief.
pub const K_CONFIRM_RTTS: usize = 20;

/// Minimax plies per CCA decision.  This must be even: one CCA action and one
/// adversary action make a round.  Four is a practical interactive default;
/// increase only after reducing the action spaces or when you specifically
/// want a deeper look-ahead.
pub const SEARCH_DEPTH: u16 = 4;

/// Candidate CCA actions.  The planner (`state_policy`) selects which of
/// them minimax may consider at each RTT; they are evaluated against the
/// current belief and deduplicated automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateAction {
    /// Largest rate that keeps the one-RTT queue bound (no loss for any
    /// (C, B) in the belief).
    MaxAllowedRate,
    /// Drain the largest possible queue at the smallest plausible capacity.
    /// Once the queue is empty this is `min_c`: the lowest-delay rate.
    DrainQueue,
    /// `loss_tolerance_abs` above `MaxAllowedRate`.
    ProbeAboveLimit,
}

/// Lexicographic objective, from most important to least important.  Pass
/// `--obj-perm ...` to temporarily prepend CLI choices; edit this list for
/// the experiment's stable objective.
///
/// Built-in objective names (each one is optimized in the listed direction):
///
/// * `c`: minimize the width of the capacity belief, `max_c - min_c`.
/// * `b`: minimize the width of the buffer belief, `max_b - min_b`.
/// * `sc`: prefer any shrinkage of the capacity interval.
/// * `sb`: prefer an upper buffer bound below the global maximum.
/// * `s`: maximize delivered data over the evaluated horizon.
/// * `ld`: minimize loss exceeding `loss_tolerance_abs`.
/// * `l`: minimize total observed loss, including loss within the tolerance.
/// * `qdel`: minimize the largest observed queue-delay estimate (beyond the
///   unavoidable one-RTT baseline).
/// * `schmul` / `schadd`: prefer `max_c <= start_max_c / 2` /
///   `max_c < global_max_c`; evidence that the high-capacity end is ruled out.
/// * `sclmul` / `scladd`: prefer `min_c >= 2 * start_min_c` /
///   `min_c >= start_min_c + 2`; evidence that the low-capacity end is ruled
///   out.
/// * `ttsc`: minimize RTTs until the capacity interval first shrinks.
/// * `ttscl`: minimize RTTs until `min_c` rises by at least two packets/RTT.
/// * `lt`: prefer a total loss increase within `loss_tolerance_abs`.
/// * `ttc`: minimize RTTs until the capacity bounds equal their final bounds
///   in the speculative horizon.
/// * `cb`: maximize `min_c + min_b`, a conservative combined lower bound.
/// * `ttcb`: minimize RTTs until that combined lower bound reaches its final
///   value.
/// * `ttscbadd`: minimize RTTs until `min_c + min_b` increases by roughly one
///   loss-tolerance unit (allowing QE numerical slack).
/// * `scb`: prefer `min_c + min_b` not decreasing from its initial value.
///
/// The `tt*` metrics use the end of the current speculative horizon as the
/// target; if it is not reached, they receive the horizon length as a penalty.
/// To create a genuinely new objective dimension, add the corresponding
/// metric to `cc_common::Metrics` and its comparison in `CCValue`.
pub const OBJECTIVE_ORDER: &[&str] = &["lt", "schadd", "ttscbadd", "scb", "c", "l"];

/// The number of observations accepted by one generated QE predicate.  Six
/// transitions need seven observations.  Do not make a QE window shorter than
/// this: the generated evaluator is intentionally only used at this size.
pub const QE_WINDOW_OBSERVATIONS: usize = 7;

// ------------------------------------------------------------------------------------------
// Planner constants.  These are starting points for experiments, not tuned
// values; `--probe-interval` and `--drain-interval` override two of them.

/// The capacity belief counts as tight once `max_c <= C_TIGHT_RATIO * min_c`.
pub const C_TIGHT_RATIO: RealNumRep = RealNumRep::new_raw(5, 4);

/// The buffer counts as small once `max_b < SMALL_BUFFER_BDP * min_c`.
pub const SMALL_BUFFER_BDP: RealNumRep = RealNumRep::new_raw(1, 1);

/// RTTs between probes once C is tight.
pub const PROBE_INTERVAL_RTTS: usize = 10;

/// RTTs between probes while C is still loose but the CCA is cautious.
pub const CAUTIOUS_EXPLORE_PROBE_GAP_RTTS: usize = 4;

/// RTTs between guaranteed queue drains (for an accurate min-RTT).  In a
/// real deployment one step is one min-RTT, so this is ~10 s / min-RTT.
pub const DRAIN_INTERVAL_RTTS: usize = 40;

/// A drain is complete once the belief proves `max_q <= DRAIN_MAX_Q + K`
/// for the largest assumed K: with bursts of up to K the queue cannot be
/// proven any smaller.
pub const DRAIN_MAX_Q: RealNumRep = RealNumRep::new_raw(1, 1);

/// Give up a drain that has not been proven complete after this many RTTs.
pub const MAX_DRAIN_RTTS: usize = 3;

/// Loss budget (packets per probe) when cautious.
pub const CAUTIOUS_LOSS_BUDGET: RealNumRep = RealNumRep::new_raw(1, 1);

// ------------------------------------------------------------------------------------------

/// A non-overlap encountered while intersecting fixed-size QE windows.
///
/// A long history is approximated by running QE on overlapping seven-
/// observation windows.  Each window returns an interval for the persistent
/// parameters `C` (link rate) and `B` (buffer).  To combine them, the engine
/// takes the maximum lower bound and minimum upper bound for each parameter.
/// This value means that doing so for `next_window` would make either the C or
/// B interval empty:
///
/// `max(accumulated.min_c, next.min_c) > min(accumulated.max_c, next.max_c)`
/// (or the corresponding B expression).
///
/// It is not evidence that the concrete simulation has impossible C/B values.
/// Generated QE uses finite discretization and per-window approximations, so
/// independently computed windows can disagree.  `state_policy` receives the
/// two bounds instead of a panic so it can choose a shorter history, test
/// several lengths, or apply an experiment-specific rule.
#[derive(Clone, Debug)]
pub struct InconsistentQeWindows {
    /// Number of newest observations used for the failed request.
    pub history_observations: usize,
    /// Zero-based index of the conflicting window, oldest window being zero.
    pub window_index: usize,
    /// Intersection of every earlier window that was still consistent.
    pub accumulated: BeliefBounds,
    /// Bounds returned by the window that conflicts with `accumulated`.
    pub next_window: BeliefBounds,
}

/// Runs QE on the newest requested history and intersects its C/B bounds.
pub type BeliefBoundsQuery<'a> =
    dyn Fn(usize) -> Result<BeliefBounds, InconsistentQeWindows> + 'a;

/// Everything the policy may inspect when it chooses the settings for one
/// minimax root.  `history` is oldest-to-newest and is deliberately not
/// truncated, so policy code can use arbitrarily long real histories.
pub struct StateQuantities<'a> {
    pub timestep: usize,
    pub history: &'a [ObservationNC],
    pub previous_belief: BeliefBounds,
    /// Use this to probe one or more history lengths before selecting policy.
    /// It uses the K set `update_k` returned for this state.
    pub belief_bounds_for_history: &'a BeliefBoundsQuery<'a>,
    /// CLI overrides of `PROBE_INTERVAL_RTTS` / `DRAIN_INTERVAL_RTTS`.
    pub probe_interval_rtts: usize,
    pub drain_interval_rtts: usize,
}

/// What the planner is doing this RTT.  Logged in the CSV.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// C is loose: let minimax choose among probing and holding.
    #[default]
    Explore,
    /// C is tight: hold the lowest-delay rate.
    Cruise,
    /// C is tight and a probe is due.
    Probe,
    /// Hold the drain rate until the belief proves the queue is empty.
    Drain,
}

/// Burstiness hypotheses for `KPolicy::Hypotheses`.
#[derive(Clone, Debug, Default)]
pub struct KTracker {
    /// Every hypothesis and whether it is still consistent.
    pub alive: Vec<(RealNumInt, bool)>,
    /// Timestep since which no K was ruled out.
    pub stable_since: usize,
    /// The K set the belief currently uses.
    pub assumed: Vec<RealNumInt>,
}

/// Planner memory that persists across RTTs.  Unlike `StatePolicy`, it is
/// owned by the engine and updated once per committed observation.
#[derive(Clone, Debug, Default)]
pub struct PlannerState {
    pub mode: Mode,
    /// Timestep of the most recent probe (an action above `MaxAllowedRate`
    /// or a `Probe`-mode action).
    pub last_probe: Option<usize>,
    /// Timestep at which the belief last proved the queue empty.
    pub last_drained: usize,
    /// Timestep at which the current (or last) drain began.
    pub drain_started: usize,
    pub k: KTracker,
}

/// Settings selected once at the beginning of a state.  They remain immutable
/// throughout that state's search tree, even though speculative observations
/// are appended while minimax explores it.
#[derive(Clone, Debug)]
pub struct StatePolicy {
    /// Number of observations whose information should constrain the belief.
    /// Any value at least `QE_WINDOW_OBSERVATIONS` is valid; every overlapping
    /// QE-sized window is evaluated and their bounds are intersected.
    pub belief_history_observations: usize,
    pub loss_tolerance_abs: RealNumRep,
    pub delay_tolerance_frac: RealNumRep,
    pub objective_order: Vec<String>,
    /// Actions minimax may choose from.  With a single action the engine
    /// skips the search.
    pub candidate_actions: Vec<CandidateAction>,
    pub mode: Mode,
}

impl Default for StatePolicy {
    fn default() -> Self {
        Self {
            belief_history_observations: QE_WINDOW_OBSERVATIONS,
            loss_tolerance_abs: 2.into(),
            delay_tolerance_frac: 3.into(),
            objective_order: names(OBJECTIVE_ORDER),
            candidate_actions: vec![
                CandidateAction::MaxAllowedRate,
                CandidateAction::DrainQueue,
                CandidateAction::ProbeAboveLimit,
            ],
            mode: Mode::Explore,
        }
    }
}

fn names(objectives: &[&str]) -> Vec<String> {
    objectives.iter().map(|name| (*name).to_string()).collect()
}

/// Bursty model, `KPolicy::Hypotheses` only: update which K are still
/// consistent and return the K set the belief should use.  Called once per
/// committed observation, before `state_policy`.  `feasible(k)` says
/// whether the newest QE window is consistent with burstiness `k`.
///
/// K feasibility is not monotone, so every hypothesis is tested.  While the
/// smallest alive K is unconfirmed, the belief is the union over every
/// alive K; once it has survived `K_CONFIRM_RTTS` RTTs without any K being
/// ruled out, only it is assumed.
pub fn update_k(
    planner: &mut PlannerState,
    timestep: usize,
    feasible: &dyn Fn(RealNumInt) -> bool,
) -> Vec<RealNumInt> {
    let hypotheses = match K_POLICY {
        KPolicy::Fixed(k) => return vec![k],
        KPolicy::Hypotheses(ks) => ks,
    };
    let tracker = &mut planner.k;
    if tracker.alive.is_empty() {
        tracker.alive = hypotheses.iter().map(|&k| (k, true)).collect();
        tracker.stable_since = timestep;
    }
    let smallest = |alive: &[(RealNumInt, bool)]| alive.iter().find(|(_, a)| *a).map(|(k, _)| *k);
    let smallest_before = smallest(&tracker.alive);
    for (k, alive) in tracker.alive.iter_mut() {
        if *alive && !feasible(*k) {
            info!("t={}: K={} ruled out", timestep, k);
            *alive = false;
        }
    }
    // Ruling out a large K (e.g. by a small buffer) is no evidence against
    // the smallest K, so only restart its confirmation when it changes.
    if smallest(&tracker.alive) != smallest_before {
        tracker.stable_since = timestep;
    }
    let mut alive: Vec<RealNumInt> =
        tracker.alive.iter().filter(|(_, a)| *a).map(|(k, _)| *k).collect();
    if alive.is_empty() {
        // Every hypothesis is inconsistent: QE discretization, or a K outside
        // the list.  Start over rather than use an empty belief.
        warn!("t={}: every K hypothesis was ruled out; reviving all", timestep);
        tracker.alive = hypotheses.iter().map(|&k| (k, true)).collect();
        tracker.stable_since = timestep;
        alive = hypotheses.to_vec();
    }
    tracker.assumed = if timestep >= tracker.stable_since + K_CONFIRM_RTTS {
        vec![alive[0]]
    } else {
        alive
    };
    tracker.assumed.clone()
}

/// Called after the engine commits the chosen action at `timestep`.
pub fn record_action(planner: &mut PlannerState, timestep: usize, action: Option<CandidateAction>) {
    if planner.mode == Mode::Probe || action == Some(CandidateAction::ProbeAboveLimit) {
        planner.last_probe = Some(timestep);
    }
}

/// Edit this function to make the CCA state-dependent.  It is called exactly
/// once before each minimax search, rather than at speculative tree nodes.
/// It is the long-horizon planner: it picks a `Mode`, and the mode
/// restricts what minimax may do over its short horizon.
pub fn state_policy(state: &StateQuantities<'_>, planner: &mut PlannerState) -> StatePolicy {
    // This is deliberately policy code, not an engine fallback: try the
    // desired long history and choose the QE-sized history if its window
    // intersection is inconsistent.
    let default_history_len = 10;
    let (_history_len, belief) = match (state.belief_bounds_for_history)(default_history_len) {
        Ok(belief) => (default_history_len, belief),
        Err(conflict) => (
            conflict.history_observations - conflict.window_index - 1,
            conflict.accumulated,
        ),
    };
    let t = state.timestep;

    let c_tight = belief.max_c <= belief.min_c * C_TIGHT_RATIO;
    let small_buffer = belief.max_b < belief.min_c * SMALL_BUFFER_BDP;
    // Unconfirmed burstiness: more than one K is still in the belief.
    let k_unconfirmed = planner.k.assumed.len() > 1;
    let cautious = small_buffer || k_unconfirmed;
    let since_probe = planner.last_probe.map_or(usize::MAX, |p| t - p);

    let k_max: RealNumRep = match K_POLICY {
        KPolicy::Fixed(k) => k,
        KPolicy::Hypotheses(_) => planner.k.assumed.iter().copied().max().unwrap_or(0),
    }
    .into();
    let drained = belief.max_q <= DRAIN_MAX_Q + k_max;
    if drained {
        planner.last_drained = t;
    }
    if planner.mode != Mode::Drain {
        planner.drain_started = t;
    }
    let drain_due = t >= planner.last_drained + state.drain_interval_rtts;
    let drain_gave_up = planner.mode == Mode::Drain && t >= planner.drain_started + MAX_DRAIN_RTTS;
    if drain_gave_up {
        warn!("t={}: drain not proven after {} RTTs (max_q={})", t, MAX_DRAIN_RTTS, belief.max_q);
        planner.last_drained = t;
    }

    let mode = if planner.mode == Mode::Drain && !drained && !drain_gave_up {
        Mode::Drain
    } else if drain_due && !drain_gave_up {
        Mode::Drain
    } else if !c_tight {
        Mode::Explore
    } else if since_probe >= state.probe_interval_rtts {
        Mode::Probe
    } else {
        Mode::Cruise
    };
    planner.mode = mode;

    let aggressive_budget = std::cmp::max(belief.min_c, 2.into());
    let (candidate_actions, loss_tolerance_abs) = match mode {
        Mode::Explore => {
            // Capturing bandwidth fast matters most while C is loose; only
            // a buffer proven small makes the CCA ration its probes.
            let probe_ok = !small_buffer || since_probe >= CAUTIOUS_EXPLORE_PROBE_GAP_RTTS;
            let mut actions = vec![CandidateAction::MaxAllowedRate, CandidateAction::DrainQueue];
            if probe_ok {
                actions.push(CandidateAction::ProbeAboveLimit);
            }
            let budget = if small_buffer { CAUTIOUS_LOSS_BUDGET } else { aggressive_budget };
            (actions, budget)
        }
        Mode::Probe => {
            let budget = if cautious { 0.into() } else { CAUTIOUS_LOSS_BUDGET };
            (vec![CandidateAction::ProbeAboveLimit], budget)
        }
        Mode::Cruise | Mode::Drain => (vec![CandidateAction::DrainQueue], CAUTIOUS_LOSS_BUDGET),
    };

    StatePolicy {
        belief_history_observations: state.history.len(),
        loss_tolerance_abs,
        delay_tolerance_frac: 3.into(),
        objective_order: names(OBJECTIVE_ORDER),
        candidate_actions,
        mode,
    }
}
