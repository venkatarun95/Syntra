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
    /// `min_c`, less whatever the queue may exceed `queue_target` by: hold
    /// the queue near the smallest provable size (about K under bursts)
    /// rather than paying ~K packets/RTT of throughput to aim at zero.
    HoldQueue,
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
/// * `sent`: maximize the data the CCA sends over the horizon.
///
/// The `tt*` metrics use the end of the current speculative horizon as the
/// target; if it is not reached, they receive the horizon length as a penalty.
/// To create a genuinely new objective dimension, add the corresponding
/// metric to `cc_common::Metrics` and its comparison in `CCValue`.
///
/// `sent` comes last: with `SEARCH_DEPTH` 4 every searched decision in a
/// 16-link sweep was an exact tie on all the others, or preferred the higher
/// rate (see "Ties" in `CCA_DEVELOPMENT.md`).  Making that preference an
/// objective gives every decision one optimum instead of a tie broken by the
/// search's move order.
pub const OBJECTIVE_ORDER: &[&str] = &["lt", "schadd", "ttscbadd", "scb", "c", "l", "sent"];

/// The number of observations accepted by one generated QE predicate.  Six
/// transitions need seven observations.  Do not make a QE window shorter than
/// this: the generated evaluator is intentionally only used at this size.
pub const QE_WINDOW_OBSERVATIONS: usize = 7;

// ------------------------------------------------------------------------------------------
// Planner constants.  These are starting points for experiments, not tuned
// values; `--probe-interval` and `--drain-interval` override two of them.

/// The capacity belief counts as tight once
/// `max_c <= C_TIGHT_RATIO * min_c + C_TIGHT_PER_K * K`.
pub const C_TIGHT_RATIO: RealNumRep = RealNumRep::new_raw(5, 4);

/// ... plus this many packets/RTT per unit of the largest assumed K.
pub const C_TIGHT_PER_K: RealNumRep = RealNumRep::new_raw(0, 1);

/// The buffer counts as small once `max_b < SMALL_BUFFER_BDP * min_c`.
pub const SMALL_BUFFER_BDP: RealNumRep = RealNumRep::new_raw(1, 1);

/// While exploring, every rate keeps the worst-case queue within this many
/// BDPs (`EXPLORE_MAX_QUEUE_BDP * min_c`), and above it the CCA holds the
/// queue there.  At `C = min_c` the delay box alone can hold one BDP, so
/// two leave one BDP to grow into: the rate can roughly double per RTT
/// while C is loose.  Filling a large buffer adds delay (and makes QE
/// enumerate many unobserved losses, which is slow).
pub const EXPLORE_MAX_QUEUE_BDP: RealNumRep = RealNumRep::new_raw(2, 1);

/// An Explore probe's loss budget, in BDPs (`min_c`): the probe is this
/// much above `MaxAllowedRate`.  Larger converges faster and loses more in
/// the worst case.
pub const EXPLORE_BUDGET_BDP: RealNumRep = RealNumRep::new_raw(1, 1);

/// RTTs between probes once C is tight.
pub const PROBE_INTERVAL_RTTS: usize = 10;

/// RTTs between probes while C is loose: a probe's losses are observable
/// about two RTTs after it is sent.
pub const EXPLORE_PROBE_GAP_RTTS: usize = 2;

/// RTTs between probes while C is still loose but the CCA is cautious.
pub const CAUTIOUS_EXPLORE_PROBE_GAP_RTTS: usize = 4;

/// RTTs between guaranteed queue drains (for an accurate min-RTT).  In a
/// real deployment one step is one min-RTT, so this is ~10 s / min-RTT.
pub const DRAIN_INTERVAL_RTTS: usize = 40;

/// A drain is complete once the belief proves `max_q <= DRAIN_MAX_Q + K`
/// for the largest assumed K: with bursts of up to K the queue cannot be
/// proven any smaller.
pub const DRAIN_MAX_Q: RealNumRep = RealNumRep::new_raw(1, 1);

/// RTTs in which the CCA sent at most `min_c` that `PlannerState::queue_floor`
/// is the smallest `max_q` of.
pub const QUEUE_FLOOR_RTTS: usize = 3;

/// An RTT counts towards the queue floor if the CCA sent at least this
/// fraction of `min_c` (and one packet) below `min_c`.
pub const QUIET_MARGIN: RealNumRep = RealNumRep::new_raw(1, 8);

/// Give up a drain that has not been proven complete after this many RTTs.
pub const MAX_DRAIN_RTTS: usize = 3;

/// The CCA counts as app-limited when it sent this much less than its rate.
pub const APP_LIMITED_SLACK: RealNumRep = RealNumRep::new_raw(1, 16);

/// An RTT is evidence for confirming a small K only if the link delivered
/// at least this fraction of `min_c` in it: bursts only show on a link that
/// carries traffic.  (Delivery, not sending: a drain RTT sends nothing but
/// keeps the link busy, while an idle app leaves it empty.)
pub const INFORMATIVE_LOAD: RealNumRep = RealNumRep::new_raw(1, 2);

/// Loss budget (packets per probe) when cautious.
pub const CAUTIOUS_LOSS_BUDGET: RealNumRep = RealNumRep::new_raw(1, 1);

/// How the belief is maintained across RTTs.  These are experiment switches
/// (`--k-confirm`, `--k-optimistic`, `--change-restart`, `--explore-free`);
/// the defaults reproduce the original behaviour.
#[derive(Clone, Copy, Debug)]
pub struct BeliefStrategy {
    /// `K_CONFIRM_RTTS`: informative RTTs before the smallest alive K is
    /// assumed alone.
    pub k_confirm_rtts: usize,
    /// Assume the smallest alive K from the start.  A wrong K is ruled out
    /// by the windows it cannot explain, so this is still sound in the
    /// limit, but it trusts the small K before any evidence for it.
    pub k_optimistic: bool,
    /// Treat evidence against a static link as a change of the link: when a
    /// confirmed K is ruled out by disagreeing windows, or the belief's
    /// windows disagree by more than QE slack, restart the belief at the
    /// newest window (instead of escalating K or keeping the older bound).
    pub change_restart: bool,
    /// In Explore, do not force `DrainQueue` when the queue may exceed
    /// `EXPLORE_MAX_QUEUE_BDP`: let minimax choose, with `qdel` among its
    /// objectives (`EXPLORE_FREE_OBJECTIVE_ORDER`).
    pub explore_free: bool,
    /// The upper bound on C expires: only the newest this many observations'
    /// windows bound `max_c` for the planner.  A rise in C is invisible to a
    /// sender at the old rate, so no observation can ever contradict the old
    /// `max_c`; forgetting it is the only way to let the CCA look again.
    /// While the link stays busy the recent windows keep `max_c` tight, so
    /// this only loosens it once the queue has emptied.
    pub c_upper_rtts: Option<usize>,
    /// `EXPLORE_BUDGET_BDP` and `EXPLORE_MAX_QUEUE_BDP` (`--explore-budget`,
    /// `--explore-queue`): the exploration trade-off between convergence
    /// speed and worst-case loss and queue.
    pub explore_budget_bdp: RealNumRep,
    pub explore_queue_bdp: RealNumRep,
}

impl Default for BeliefStrategy {
    fn default() -> Self {
        Self {
            k_confirm_rtts: K_CONFIRM_RTTS,
            k_optimistic: false,
            change_restart: false,
            explore_free: false,
            c_upper_rtts: None,
            explore_budget_bdp: EXPLORE_BUDGET_BDP,
            explore_queue_bdp: EXPLORE_MAX_QUEUE_BDP,
        }
    }
}

/// `explore_free`: the objective once the queue may exceed
/// `EXPLORE_MAX_QUEUE_BDP`, where the planner would otherwise force a drain.
pub const EXPLORE_FREE_OBJECTIVE_ORDER: &[&str] = &["lt", "qdel", "schadd", "ttscbadd", "scb", "c", "l", "sent"];

/// `change_restart`: windows a shadow belief (`KShadow`) must fit before the
/// CCA concludes that the link changed.
pub const SHADOW_WINDOWS: usize = 3;

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
    /// No static link explains the window at all (it straddles a change);
    /// `next_window` is then meaningless.
    pub infeasible: bool,
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
    /// `--belief-history`: observations whose overlapping QE windows are
    /// intersected for the belief (`None`: the whole history).
    pub belief_history: Option<usize>,
    pub strategy: BeliefStrategy,
    /// Some static link (under the assumed K) explains the newest QE window.
    /// For a few RTTs after the link changes, none does, and the model gives
    /// minimax nothing to search with.
    pub newest_window_feasible: bool,
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
    /// Per hypothesis, the intersection of every window's C and B bounds
    /// under it.  C and B are static, so an empty intersection rules the
    /// hypothesis out even if each window alone is feasible.
    pub bounds: Vec<Option<BeliefBounds>>,
    /// Informative RTTs (see `INFORMATIVE_LOAD`) since the smallest alive K
    /// last changed.  Idle RTTs cannot rule a K out, so they do not count
    /// towards confirming it.
    pub stable_rtts: usize,
    /// The K set the belief currently uses.
    pub assumed: Vec<RealNumInt>,
    pub shadow: Option<KShadow>,
}

/// `change_restart`: when the confirmed K stops fitting, either the link is
/// burstier than K or it changed.  K escalates as usual, and in parallel a
/// belief under the confirmed K is built from windows that start after the
/// conflict only.  If `SHADOW_WINDOWS` of them fit, the old windows were the
/// problem: the link changed, and the belief restarts there.  If one does not
/// fit, the link really is burstier, and the shadow is dropped.
#[derive(Clone, Debug)]
pub struct KShadow {
    pub k: RealNumInt,
    /// First observation the shadow's windows may contain.
    pub from: usize,
    pub windows: usize,
    pub bounds: Option<BeliefBounds>,
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
    /// The application sent less than the CCA's last rate.
    pub app_limited: bool,
    /// `min_c` of the last belief, for `INFORMATIVE_LOAD`.
    pub last_min_c: RealNumRep,
    pub k: KTracker,
    /// Index of the first observation the belief uses (`change_restart`).
    pub belief_start: usize,
    /// The part of `max_q` the sender has shown it cannot drain (bursts
    /// held back, unreported losses, delay-box ambiguity): the smallest
    /// `max_q` over the last `QUEUE_FLOOR_RTTS` RTTs in which the CCA sent
    /// at most `min_c` less `QUIET_MARGIN`, which drain any real queue.  Re-tested at
    /// every periodic drain.
    pub queue_floor: RealNumRep,
    /// `max_q` after each of those recent RTTs.
    pub quiet_max_q: std::collections::VecDeque<RealNumRep>,
    /// What was sent in the last RTT.
    pub last_sent: RealNumRep,
    /// Times the belief was restarted because the link changed.
    pub restarts: usize,
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
    /// Queue `HoldQueue` aims for.
    pub queue_target: RealNumRep,
    /// Every candidate rate is capped so that the worst-case queue (at
    /// `C = min_c`) stays within this.
    pub queue_cap: Option<RealNumRep>,
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
            queue_target: DRAIN_MAX_Q,
            queue_cap: None,
        }
    }
}

fn names(objectives: &[&str]) -> Vec<String> {
    objectives.iter().map(|name| (*name).to_string()).collect()
}

/// `update_k`: how far two windows' C or B bounds may miss each other
/// before a K is ruled out.  Zero: each window's bounds are an exact
/// projection rounded outward (`network_model_nc::exact`), so under the true
/// K they always overlap.
pub const K_WINDOW_DISAGREEMENT_TOL: RealNumRep = RealNumRep::new_raw(0, 1);

/// Intersect a window's C/B bounds into `acc`, within
/// `K_WINDOW_DISAGREEMENT_TOL` (QE slack); `None` if they disagree by more.
fn merge_window(acc: Option<BeliefBounds>, next: BeliefBounds) -> Option<BeliefBounds> {
    let Some(prev) = acc else { return Some(next) };
    let mut merged = BeliefBounds {
        min_c: std::cmp::max(prev.min_c, next.min_c),
        max_c: std::cmp::min(prev.max_c, next.max_c),
        min_b: std::cmp::max(prev.min_b, next.min_b),
        max_b: std::cmp::min(prev.max_b, next.max_b),
        ..next
    };
    let tol = K_WINDOW_DISAGREEMENT_TOL;
    if merged.min_c > merged.max_c + tol || merged.min_b > merged.max_b + tol {
        return None;
    }
    if merged.min_c > merged.max_c {
        (merged.min_c, merged.max_c) = (next.min_c, next.max_c);
    }
    if merged.min_b > merged.max_b {
        (merged.min_b, merged.max_b) = (next.min_b, next.max_b);
    }
    Some(merged)
}

/// Bursty model, `KPolicy::Hypotheses` only: update which K are still
/// consistent and return the K set the belief should use.  Called once per
/// committed observation, before `state_policy`.  `window_belief(k)` is the
/// newest QE window's belief under burstiness `k`, or `None` if the window
/// is infeasible for it.  `delivered` is the newest RTT's service; only RTTs
/// that delivered `INFORMATIVE_LOAD * min_c` count towards confirming a K.
///
/// K feasibility is not monotone, so every hypothesis is tested.  While the
/// smallest alive K is unconfirmed, the belief is the union over every
/// alive K; once it has survived `K_CONFIRM_RTTS` RTTs, only it is assumed.
pub fn update_k(
    planner: &mut PlannerState,
    timestep: usize,
    delivered: RealNumRep,
    strategy: &BeliefStrategy,
    window_belief: &dyn Fn(RealNumInt) -> Option<BeliefBounds>,
) -> Vec<RealNumInt> {
    let hypotheses = match K_POLICY {
        KPolicy::Fixed(k) => return vec![k],
        KPolicy::Hypotheses(ks) => ks,
    };
    let tracker = &mut planner.k;
    if tracker.alive.is_empty() {
        tracker.alive = hypotheses.iter().map(|&k| (k, true)).collect();
        tracker.bounds = vec![None; hypotheses.len()];
        tracker.stable_rtts = 0;
    }
    let smallest = |alive: &[(RealNumInt, bool)]| alive.iter().find(|(_, a)| *a).map(|(k, _)| *k);
    let smallest_before = smallest(&tracker.alive);
    // `change_restart`: a K that explained every window for
    // `k_confirm_rtts` informative RTTs and then stops doing so is evidence
    // that the link changed, not that it was burstier all along.
    let confirmed = (strategy.change_restart && tracker.stable_rtts >= strategy.k_confirm_rtts)
        .then_some(smallest_before)
        .flatten();
    let mut conflicted = None;
    // The shadow takes the newest window once it starts after the conflict.
    let mut adopt = None;
    if let Some(sh) = tracker.shadow.as_mut() {
        if timestep >= sh.from {
            match window_belief(sh.k).and_then(|next| merge_window(sh.bounds, next)) {
                Some(b) => {
                    sh.bounds = Some(b);
                    sh.windows += 1;
                    if sh.windows >= SHADOW_WINDOWS {
                        adopt = tracker.shadow.take();
                    }
                }
                None => {
                    info!("t={}: fresh windows do not fit K={} either: burstier, not changed", timestep, sh.k);
                    tracker.shadow = None;
                }
            }
        }
    }
    for ((k, alive), acc) in tracker.alive.iter_mut().zip(tracker.bounds.iter_mut()) {
        if !*alive {
            continue;
        }
        let Some(next) = window_belief(*k) else {
            if confirmed == Some(*k) {
                conflicted = Some(*k);
            }
            info!("t={}: K={} ruled out (window infeasible)", timestep, k);
            *alive = false;
            continue;
        };
        let merged = match acc {
            None => next,
            Some(prev) => BeliefBounds {
                min_c: std::cmp::max(prev.min_c, next.min_c),
                max_c: std::cmp::min(prev.max_c, next.max_c),
                min_b: std::cmp::max(prev.min_b, next.min_b),
                max_b: std::cmp::min(prev.max_b, next.max_b),
                ..next
            },
        };
        let tol = K_WINDOW_DISAGREEMENT_TOL;
        if merged.min_c > merged.max_c + tol || merged.min_b > merged.max_b + tol {
            if confirmed == Some(*k) {
                conflicted = Some(*k);
            }
            info!(
                "t={}: K={} ruled out (windows disagree: c=[{}, {}], b=[{}, {}])",
                timestep, k, merged.min_c, merged.max_c, merged.min_b, merged.max_b
            );
            *alive = false;
            continue;
        }
        // Within the tolerance, keep the newest window's bounds for the
        // dimension that crossed, so the intersection stays non-empty.
        let mut merged = merged;
        if merged.min_c > merged.max_c {
            (merged.min_c, merged.max_c) = (next.min_c, next.max_c);
        }
        if merged.min_b > merged.max_b {
            (merged.min_b, merged.max_b) = (next.min_b, next.max_b);
        }
        *acc = Some(merged);
    }
    if let Some(k) = conflicted {
        if !tracker.alive.iter().any(|(_, a)| *a) {
            // No K explains the old windows with this one, while the
            // confirmed K did until now: the link changed.  The belief starts
            // over at the newest window (one straddling the change fits no K;
            // the next ones will), and K stays confirmed.
            info!("t={}: no K fits since confirmed K={} stopped fitting: link changed", timestep, k);
            adopt = Some(KShadow { k, from: timestep, windows: 0, bounds: None });
        } else if tracker.shadow.is_none() && adopt.is_none() {
            info!("t={}: confirmed K={} stopped fitting; checking fresh windows for a change", timestep, k);
            tracker.shadow = Some(KShadow {
                k,
                from: timestep + QE_WINDOW_OBSERVATIONS - 1,
                windows: 0,
                bounds: None,
            });
        }
    }
    let mut adopted = false;
    if let Some(sh) = adopt {
        info!("t={}: link changed; belief restarts at observation {} with K={}", timestep, sh.from, sh.k);
        for ((k, alive), acc) in tracker.alive.iter_mut().zip(tracker.bounds.iter_mut()) {
            *alive = *k >= sh.k;
            *acc = if *k == sh.k { sh.bounds } else { None };
        }
        tracker.shadow = None;
        tracker.stable_rtts = tracker.stable_rtts.max(strategy.k_confirm_rtts);
        planner.belief_start = sh.from;
        planner.restarts += 1;
        adopted = true;
    }
    let tracker = &mut planner.k;
    // Ruling out a large K (e.g. by a small buffer) is no evidence against
    // the smallest K, so only restart its confirmation when it changes.
    if smallest(&tracker.alive) != smallest_before && !adopted {
        tracker.stable_rtts = 0;
    }
    let mut alive: Vec<RealNumInt> =
        tracker.alive.iter().filter(|(_, a)| *a).map(|(k, _)| *k).collect();
    if alive.is_empty() {
        // Every hypothesis is inconsistent: QE discretization, or a K outside
        // the list.  Start over rather than use an empty belief.
        warn!("t={}: every K hypothesis was ruled out; reviving all", timestep);
        tracker.alive = hypotheses.iter().map(|&k| (k, true)).collect();
        tracker.bounds = vec![None; hypotheses.len()];
        tracker.stable_rtts = 0;
        alive = hypotheses.to_vec();
    }
    if delivered >= planner.last_min_c * INFORMATIVE_LOAD {
        tracker.stable_rtts += 1;
    }
    tracker.assumed = if strategy.k_optimistic || tracker.stable_rtts >= strategy.k_confirm_rtts {
        vec![alive[0]]
    } else {
        alive
    };
    tracker.assumed.clone()
}

/// The search's choice among `root_moves` without searching, and whether it is known to be
/// the search's choice.  `root_moves` are the distinct candidate rates in
/// `StatePolicy::candidate_actions` order.
///
/// The search tries the root moves last to first and keeps the first one unless a later one
/// is strictly better, so on a tie it returns the last: `ProbeAboveLimit` when offered, else
/// `DrainQueue`.  In the `--decisions` logs of 30 varied links (see `CCA_DEVELOPMENT.md`)
/// every root move tied exactly while `max_c` was still `LARGEST_BW`: the delay box lets the
/// worst-case link reveal nothing new in two RTTs, whatever the CCA sends.  Once `max_c` is
/// bounded the search still chose the tie-break in 214 of 216 RTTs; the other two were
/// `DrainQueue`, which won on window-local objectives (`scb`, the width of C) that depend on
/// the worst-case replies through QE.  So the result is only known while `max_c` is
/// unbounded (or there is one move).
pub fn explore_rule(root_moves: &[RealNumRep], belief: &BeliefBounds) -> (RealNumRep, bool) {
    let known = root_moves.len() == 1 || belief.max_c >= cc_common::LARGEST_BW.into();
    (*root_moves.last().unwrap(), known)
}

/// Called after the engine commits the chosen action at `timestep`: the
/// CCA asked for `intended`, and the application let `sent` be sent.
///
/// An app-limited RTT (`sent < intended`) says little about the link: a
/// probe that was not sent is no probe.
pub fn record_action(
    planner: &mut PlannerState,
    timestep: usize,
    action: Option<CandidateAction>,
    intended: RealNumRep,
    sent: RealNumRep,
) {
    planner.app_limited = sent + APP_LIMITED_SLACK < intended;
    planner.last_sent = sent;
    let probed = planner.mode == Mode::Probe || action == Some(CandidateAction::ProbeAboveLimit);
    if probed && !planner.app_limited {
        planner.last_probe = Some(timestep);
    }
}

/// Edit this function to make the CCA state-dependent.  It is called exactly
/// once before each minimax search, rather than at speculative tree nodes.
/// It is the long-horizon planner: it picks a `Mode`, and the mode
/// restricts what minimax may do over its short horizon.
pub fn state_policy(state: &StateQuantities<'_>, planner: &mut PlannerState) -> StatePolicy {
    // This is deliberately policy code, not an engine fallback: use the whole
    // history (the engine caches committed windows, and C and B are static,
    // so nothing learned is forgotten), falling back to the windows before
    // the first inconsistent one.
    let n_obs = state.history.len();
    let since_start = n_obs.saturating_sub(planner.belief_start);
    let mut default_history_len = state
        .belief_history
        .map_or(since_start, |n| n.min(since_start))
        .clamp(QE_WINDOW_OBSERVATIONS, n_obs);
    let belief = loop {
        match (state.belief_bounds_for_history)(default_history_len) {
            Ok(belief) => break belief,
            // Changes are detected per K, in `update_k`.
            Err(conflict) => {
                break conflict.accumulated;
            }
        }
    };
    let mut belief = belief;
    if let Some(n) = state.strategy.c_upper_rtts {
        let recent_len = (n + QE_WINDOW_OBSERVATIONS - 1).min(default_history_len);
        if recent_len < default_history_len {
            let recent = match (state.belief_bounds_for_history)(recent_len) {
                Ok(b) => b,
                Err(conflict) => conflict.accumulated,
            };
            if recent.max_c > belief.max_c {
                info!("t={}: max_c {} expired; the newest {} RTTs only bound it by {}", state.timestep, belief.max_c, n, recent.max_c);
                belief.max_c = recent.max_c;
            }
        }
    }
    let t = state.timestep;
    let prev_min_c = planner.last_min_c;
    planner.last_min_c = belief.min_c;

    let k_max: RealNumRep = match K_POLICY {
        KPolicy::Fixed(k) => k,
        KPolicy::Hypotheses(_) => planner.k.assumed.iter().copied().max().unwrap_or(0),
    }
    .into();
    // Bursts of up to K blur each window's view of C, so the achievable
    // width grows with K.
    let c_tight = belief.max_c <= belief.min_c * C_TIGHT_RATIO + k_max * C_TIGHT_PER_K;
    let small_buffer = belief.max_b < belief.min_c * SMALL_BUFFER_BDP;
    // Unconfirmed burstiness: more than one K is still in the belief.
    let k_unconfirmed = planner.k.assumed.len() > 1;
    let cautious = small_buffer || k_unconfirmed;
    let since_probe = planner.last_probe.map_or(usize::MAX, |p| t - p);

    // Sending well below `min_c` drains any real queue (by at least the
    // margin), so what `max_q` keeps after such RTTs is not a queue the CCA
    // can drain.  Sending exactly `min_c` would not do: at `C = min_c` a
    // standing queue would stay, and count as a floor.
    let base_target = DRAIN_MAX_Q + k_max;
    let margin = std::cmp::max(prev_min_c * QUIET_MARGIN, 1.into());
    if planner.last_sent + margin <= prev_min_c {
        planner.quiet_max_q.push_back(belief.max_q);
        if planner.quiet_max_q.len() > QUEUE_FLOOR_RTTS {
            planner.quiet_max_q.pop_front();
        }
    }
    let floor = if planner.quiet_max_q.len() >= QUEUE_FLOOR_RTTS {
        planner.quiet_max_q.iter().copied().min().unwrap()
    } else {
        0.into()
    };
    if floor > base_target && floor > planner.queue_floor {
        info!("t={}: max_q does not drain below {}; holding it as the queue floor", t, floor);
    }
    planner.queue_floor = floor;
    let queue_target = std::cmp::max(base_target, planner.queue_floor);

    let drained = belief.max_q <= queue_target;
    if drained {
        planner.last_drained = t;
    }
    if planner.mode != Mode::Drain {
        planner.drain_started = t;
    }
    let drain_due = t >= planner.last_drained + state.drain_interval_rtts;
    if drain_due && planner.mode != Mode::Drain {
        // Re-test the floor: the link may have released what it held.
        planner.quiet_max_q.clear();
    }
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
    if !state.newest_window_feasible {
        // No static link explains the newest window: there are no feasible
        // network moves to search over.  Drain on the (restarted) belief
        // until windows after the change exist.
        return StatePolicy {
            belief_history_observations: default_history_len,
            loss_tolerance_abs: CAUTIOUS_LOSS_BUDGET,
            delay_tolerance_frac: 3.into(),
            objective_order: names(OBJECTIVE_ORDER),
            candidate_actions: vec![CandidateAction::HoldQueue],
            mode,
            queue_target,
            queue_cap: None,
        };
    }

    let aggressive_budget = std::cmp::max(belief.min_c * state.strategy.explore_budget_bdp, 2.into());
    // While exploring, the worst-case queue is held within one BDP (or the
    // floor): every rate is capped by it, and above it the CCA holds the
    // queue there instead of idling.  The link stays backlogged, which is
    // what bounds C from above.
    // Bursts of up to K are part of the worst-case queue that no rate
    // drains (as in `queue_target`), so they do not count against the cap.
    let explore_cap = belief.min_c * state.strategy.explore_queue_bdp + std::cmp::max(planner.queue_floor, k_max);
    let queue_high = belief.max_q > explore_cap;
    let mut objective_order = names(OBJECTIVE_ORDER);
    let mut queue_target = queue_target;
    let mut queue_cap = None;
    let (candidate_actions, loss_tolerance_abs) = match mode {
        Mode::Explore if queue_high && state.strategy.explore_free => {
            objective_order = names(EXPLORE_FREE_OBJECTIVE_ORDER);
            let actions = vec![
                CandidateAction::MaxAllowedRate,
                CandidateAction::DrainQueue,
                CandidateAction::ProbeAboveLimit,
            ];
            (actions, CAUTIOUS_LOSS_BUDGET)
        }
        Mode::Explore if queue_high => {
            queue_target = explore_cap;
            (vec![CandidateAction::HoldQueue], CAUTIOUS_LOSS_BUDGET)
        }
        Mode::Explore => {
            queue_cap = Some(explore_cap);
            // Capturing bandwidth fast matters most while C is loose; only
            // a buffer proven small makes the CCA ration its probes.
            // A probe's losses are reported about two RTTs later; probing
            // again before then would spend its loss budget a second time.
            let probe_gap = if small_buffer { CAUTIOUS_EXPLORE_PROBE_GAP_RTTS } else { EXPLORE_PROBE_GAP_RTTS };
            let probe_ok = since_probe >= probe_gap;
            // `HoldQueue` (at the cap) rather than `DrainQueue`: idling for
            // a whole RTT teaches nothing about C and cannot drain a floor.
            queue_target = explore_cap;
            let mut actions = vec![CandidateAction::MaxAllowedRate, CandidateAction::HoldQueue];
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
        Mode::Cruise => (vec![CandidateAction::HoldQueue], CAUTIOUS_LOSS_BUDGET),
        // `HoldQueue` at the queue target: `DrainQueue` unless a floor is
        // known, in which case draining below it would only idle the link.
        Mode::Drain => (vec![CandidateAction::HoldQueue], CAUTIOUS_LOSS_BUDGET),
    };

    StatePolicy {
        belief_history_observations: default_history_len,
        loss_tolerance_abs,
        delay_tolerance_frac: 3.into(),
        objective_order,
        candidate_actions,
        mode,
        queue_target,
        queue_cap,
    }
}
