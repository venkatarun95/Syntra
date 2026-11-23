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
/// belief at every node of the search tree and deduplicated automatically.
/// The search picks among them; no rule does (see "Trust the planner" in
/// `CCA_DEVELOPMENT.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateAction {
    /// Largest rate that keeps the one-RTT queue bound (no loss for any
    /// (C, B) in the belief), and whose worst-case queue (at `C = min_c`)
    /// stays within `StatePolicy::queue_cap` (a BDP): a queue beyond one
    /// BDP adds delay and no throughput.  Above the cap this drains to it.
    MaxAllowedRate,
    /// `min_c`, less whatever the queue may exceed `queue_target` by: drain
    /// the worst-case queue to its provable floor (about K under bursts) in
    /// one RTT.
    DrainQueue,
    /// Drain half of that excess: a drain spread over two RTTs.
    DrainHalf,
    /// `MaxAllowedRate` plus what is left of the loss budget in the loss
    /// window (`loss_tolerance_abs` less the loss reported in the newest
    /// `LOSS_WINDOW_RTTS`).
    ProbeAboveLimit,
    /// `MaxAllowedRate` plus half of what is left: two such probes can be
    /// in flight at once (a probe's loss shows about two RTTs later).
    ProbeHalf,
    /// What the link delivered per RTT over the (up to two) newest RTTs
    /// since the belief started, or over the newest RTT if none.  Right
    /// after a change, before any QE window exists: a backlogged link keeps
    /// its queue, an idle one keeps the old rate.  Not a bound on anything.
    /// Offered only then (`Mode::Recover`).
    MatchDelivery,
    /// `MatchDelivery` plus as much again as keeps the in-flight queue
    /// bound within two of it (`RECOVER_QUEUE_DELIVERIES`): Recover's probe.
    /// After a rise in C the queue bound is about one RTT of sending and
    /// this roughly doubles the rate; after a fall the link is backlogged,
    /// the bound is several times the delivery and this is `MatchDelivery`.
    ProbeDelivery,
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
/// * `ld`: minimize loss exceeding `loss_tolerance_abs`.  Like `l` and `lt`,
///   it counts the loss reported since the start of the loss window (the
///   newest `LOSS_WINDOW_RTTS` committed RTTs, across restarts) plus the
///   horizon.
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
/// * `dq`: minimize the smallest worst-case queue (`max_q`) reached in the
///   horizon, counting anything at or below `queue_target` as zero: a drain
///   that completes within the horizon, however it is spread.
///
/// The `tt*` metrics use the end of the current speculative horizon as the
/// target; if it is not reached, they receive the horizon length as a penalty.
/// To create a genuinely new objective dimension, add the corresponding
/// metric to `cc_common::Metrics` and its comparison in `CCValue`.
///
/// Every mode uses every objective; only the order differs (`state_policy`).
/// A mode lists its priorities, and `full_order` appends every other
/// objective in this order, before `sent`.  An extra objective never hurts:
/// it only breaks ties the earlier ones leave.  `sent` comes last in every
/// order: it is the one objective the adversary does not choose, so every
/// decision has a unique optimum rather than a tie broken by the search's
/// move order (anything after it would never matter).  The exception is
/// `dq`: a drain is needed only when no min-RTT sample has been seen for too
/// long (Drain), so a mode that does not list it gets it after `sent`.
/// Otherwise it breaks the ties of a worst-case `s` (which a link that may
/// serve a round late leaves between rates near C) toward draining, and
/// Cruise drains below C round after round.
pub const ALL_OBJECTIVES: &[&str] = &[
    "ld", "lt", "dq", "schadd", "ttscbadd", "scb", "c", "b", "sc", "sb", "schmul", "sclmul", "scladd",
    "ttsc", "ttscl", "ttc", "cb", "ttcb", "l", "qdel", "s", "sent",
];

/// Probe: learn C and B (`schadd`, `ttscbadd`, `scb`, `c`) without loss
/// beyond the budget (`ld` first), then deliver.
pub const PROBE_OBJECTIVE_ORDER: &[&str] = &["ld", "schadd", "ttscbadd", "scb", "c", "l", "s"];

/// Cruise: utilization (`s`) without loss beyond the budget.
pub const CRUISE_OBJECTIVE_ORDER: &[&str] = &["ld", "s", "l"];

/// Drain: prove the queue empty within the horizon (`dq`), and otherwise
/// deliver as much as possible.
pub const DRAIN_OBJECTIVE_ORDER: &[&str] = &["dq", "ld", "s", "l"];

/// The default (and Recover's, where there is no search).
pub const OBJECTIVE_ORDER: &[&str] = PROBE_OBJECTIVE_ORDER;

/// `priorities`, then every other objective of `ALL_OBJECTIVES` in its
/// order, then `sent`, then `dq` unless `priorities` lists it.
pub fn full_order(priorities: &[&str]) -> Vec<String> {
    let mut order: Vec<&str> = priorities.iter().copied().filter(|o| *o != "sent").collect();
    let late = |o: &str| o == "sent" || o == "dq";
    let rest: Vec<&str> = ALL_OBJECTIVES.iter().copied().filter(|o| !order.contains(o) && !late(o)).collect();
    order.extend(rest);
    order.push("sent");
    if !order.contains(&"dq") {
        order.push("dq");
    }
    order.into_iter().map(str::to_string).collect()
}

/// The number of observations accepted by one generated QE predicate.  Six
/// transitions need seven observations.  Do not make a QE window shorter than
/// this: the generated evaluator is intentionally only used at this size.
pub const QE_WINDOW_OBSERVATIONS: usize = 7;

// ------------------------------------------------------------------------------------------
// Planner constants.  The planner only picks a mode (and with it the
// objective and the candidate actions); within a mode the search decides.
// `--param NAME=VALUE` overrides any of them.

/// The capacity belief counts as tight once
/// `max_c <= C_TIGHT_RATIO * min_c + C_TIGHT_PER_K * K`.  While it is not,
/// the CCA is in Probe.
pub const C_TIGHT_RATIO: RealNumRep = RealNumRep::new_raw(5, 4);

/// ... plus this many packets/RTT per unit of the largest assumed K.
pub const C_TIGHT_PER_K: RealNumRep = RealNumRep::new_raw(0, 1);

/// C is assumed constant only for this many RTTs: `max_c` comes from the
/// newest `C_EXPIRY_RTTS` RTTs' windows alone, while `min_c` and B are
/// retained (`CongCtrlState::retained_prior`).  A rise in C is invisible to
/// a sender at the old rate, so no observation contradicts an old `max_c`;
/// letting it expire is what makes the CCA look again.  While the link is
/// backlogged the newest windows keep `max_c` tight, so it only loosens
/// when the CCA has not filled the link for a while.  0: never expires.
pub const C_EXPIRY_RTTS: usize = 20;

/// Every mode keeps the worst-case queue (at `C = min_c`) within this many
/// BDPs (`min_c`), plus the queue floor or K: `MaxAllowedRate` is capped by
/// it.  A queue beyond a BDP adds delay but no throughput.
pub const MAX_QUEUE_BDP: RealNumRep = RealNumRep::new_raw(1, 1);

/// The loss budget is counted over the newest this many committed RTTs
/// (and the search horizon), across belief restarts: repeated probes are
/// charged together, whether or not they overlap.
pub const LOSS_WINDOW_RTTS: usize = 20;

/// The loss budget while C has not been tight since the belief (re)started
/// (startup, or after a detected change), in BDPs (`min_c`), and at least
/// the constant budget.
pub const UNCERTAIN_LOSS_BDP: RealNumRep = RealNumRep::new_raw(1, 1);

/// The loss budget otherwise, in packets (`pkts_per_unit` converts it).
pub const CONSTANT_LOSS_PKTS: RealNumRep = RealNumRep::new_raw(1, 1);

/// Packets per unit of C, B and loss, for budgets set in packets.  A
/// wrapper that rescales units (cc_sim's `MinimaxQe`) passes it.
pub const PKTS_PER_UNIT: RealNumRep = RealNumRep::new_raw(1, 1);

/// The smallest loss budget, in units: QE works on a 1/16 grid.
pub const MIN_LOSS_BUDGET: RealNumRep = RealNumRep::new_raw(1, 16);

/// Drop every K hypothesis that is at least a BDP: K is ruled out once its
/// windows prove `C <= K`, and `min_c` is at least the smallest assumed K.
pub const K_BELOW_BDP: bool = true;

/// Across a belief restart, keep the old belief's `min_b` (the buffer
/// rarely changes with the rate).  A window that contradicts it overrides
/// it, as for any conflicting bound.
pub const CARRY_BUFFER: bool = true;

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

/// Recover: RTTs between probes above the delivery (`ProbeDelivery`).  A
/// probe's losses are observable about two RTTs after it is sent.
pub const RECOVER_PROBE_GAP_RTTS: usize = 2;

/// Recover's probe (`ProbeDelivery`) keeps the belief's `max_q` (in Recover
/// only the in-flight bound, which includes an RTT of service lag) within
/// this many RTTs of delivery.
pub const RECOVER_QUEUE_DELIVERIES: RealNumRep = RealNumRep::new_raw(2, 1);

/// The planner constants above, as the running CCA uses them: the constants
/// are the defaults, and `--param NAME=VALUE` (`Tunables::set`) overrides
/// them for the process (one CCA per process).
#[derive(Clone, Debug)]
pub struct Tunables {
    pub c_tight_ratio: RealNumRep,
    pub c_tight_per_k: RealNumRep,
    pub c_expiry_rtts: usize,
    pub max_queue_bdp: RealNumRep,
    pub loss_window_rtts: usize,
    pub uncertain_loss_bdp: RealNumRep,
    pub constant_loss_pkts: RealNumRep,
    pub pkts_per_unit: RealNumRep,
    pub k_below_bdp: bool,
    pub carry_buffer: bool,
    pub drain_max_q: RealNumRep,
    pub queue_floor_rtts: usize,
    pub quiet_margin: RealNumRep,
    pub max_drain_rtts: usize,
    pub app_limited_slack: RealNumRep,
    pub informative_load: RealNumRep,
    pub recover_probe_gap_rtts: usize,
    pub recover_queue_deliveries: RealNumRep,
    pub shadow_windows: usize,
    /// `K_POLICY`'s hypotheses (`k_hypotheses=1/4/16/64`; one value is
    /// `KPolicy::Fixed`).
    pub k_hypotheses: Vec<RealNumInt>,
}

impl Default for Tunables {
    fn default() -> Self {
        Self {
            c_tight_ratio: C_TIGHT_RATIO,
            c_tight_per_k: C_TIGHT_PER_K,
            c_expiry_rtts: C_EXPIRY_RTTS,
            max_queue_bdp: MAX_QUEUE_BDP,
            loss_window_rtts: LOSS_WINDOW_RTTS,
            uncertain_loss_bdp: UNCERTAIN_LOSS_BDP,
            constant_loss_pkts: CONSTANT_LOSS_PKTS,
            pkts_per_unit: PKTS_PER_UNIT,
            k_below_bdp: K_BELOW_BDP,
            carry_buffer: CARRY_BUFFER,
            drain_max_q: DRAIN_MAX_Q,
            queue_floor_rtts: QUEUE_FLOOR_RTTS,
            quiet_margin: QUIET_MARGIN,
            max_drain_rtts: MAX_DRAIN_RTTS,
            app_limited_slack: APP_LIMITED_SLACK,
            informative_load: INFORMATIVE_LOAD,
            recover_probe_gap_rtts: RECOVER_PROBE_GAP_RTTS,
            recover_queue_deliveries: RECOVER_QUEUE_DELIVERIES,
            shadow_windows: SHADOW_WINDOWS,
            k_hypotheses: match K_POLICY {
                KPolicy::Fixed(k) => vec![k],
                KPolicy::Hypotheses(ks) => ks.to_vec(),
            },
        }
    }
}

/// A fraction on a 1/64 grid (`--param`, CLI fractions).
pub fn frac(x: f64) -> RealNumRep {
    RealNumRep::new((x * 64.0).round() as RealNumInt, 64)
}

impl Tunables {
    /// Set the field `name` (lower case, as in the struct) from `value`.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        if name == "k_hypotheses" {
            self.k_hypotheses = value
                .split('/')
                .map(|k| k.parse::<RealNumInt>().map_err(|e| format!("{name}={value}: {e}")))
                .collect::<Result<_, _>>()?;
            return Ok(());
        }
        if name == "pkts_per_unit" {
            // Exact: a power of two, possibly large.
            self.pkts_per_unit = value.parse::<RealNumInt>().map_err(|e| format!("{name}={value}: {e}"))?.into();
            return Ok(());
        }
        let v: f64 = value.parse().map_err(|e| format!("{name}={value}: {e}"))?;
        match name {
            "c_tight_ratio" => self.c_tight_ratio = frac(v),
            "c_tight_per_k" => self.c_tight_per_k = frac(v),
            "c_expiry_rtts" => self.c_expiry_rtts = v.round() as usize,
            "max_queue_bdp" => self.max_queue_bdp = frac(v),
            "loss_window_rtts" => self.loss_window_rtts = v.round() as usize,
            "uncertain_loss_bdp" => self.uncertain_loss_bdp = frac(v),
            "constant_loss_pkts" => self.constant_loss_pkts = frac(v),
            "k_below_bdp" => self.k_below_bdp = v != 0.,
            "carry_buffer" => self.carry_buffer = v != 0.,
            "drain_max_q" => self.drain_max_q = frac(v),
            "queue_floor_rtts" => self.queue_floor_rtts = v.round() as usize,
            "quiet_margin" => self.quiet_margin = frac(v),
            "max_drain_rtts" => self.max_drain_rtts = v.round() as usize,
            "app_limited_slack" => self.app_limited_slack = frac(v),
            "informative_load" => self.informative_load = frac(v),
            "recover_probe_gap_rtts" => self.recover_probe_gap_rtts = v.round() as usize,
            "recover_queue_deliveries" => self.recover_queue_deliveries = frac(v),
            "shadow_windows" => self.shadow_windows = v.round() as usize,
            _ => return Err(format!("unknown parameter {name}")),
        }
        Ok(())
    }

    /// The constant loss budget in units: `constant_loss_pkts` packets, on
    /// QE's grid.
    pub fn constant_loss_budget(&self) -> RealNumRep {
        std::cmp::max(self.constant_loss_pkts / self.pkts_per_unit, MIN_LOSS_BUDGET)
    }

    /// `k_hypotheses` as a `KPolicy`.  Only for the process's tunables
    /// (`tun()`): built once, as `KPolicy` holds a static slice.
    pub fn k_policy(&self) -> KPolicy {
        static POLICY: std::sync::OnceLock<KPolicy> = std::sync::OnceLock::new();
        *POLICY.get_or_init(|| {
            let ks: &'static [RealNumInt] = Box::leak(tun().k_hypotheses.clone().into_boxed_slice());
            match ks {
                [k] => KPolicy::Fixed(*k),
                _ => KPolicy::Hypotheses(ks),
            }
        })
    }
}

static TUNABLES: std::sync::OnceLock<Tunables> = std::sync::OnceLock::new();

/// Install the process's tunables; call once, before the first RTT.
pub fn set_tunables(t: Tunables) {
    TUNABLES.set(t).expect("tunables set twice");
}

/// The process's tunables (the defaults unless `set_tunables` ran).
pub fn tun() -> &'static Tunables {
    TUNABLES.get_or_init(Tunables::default)
}

/// How the belief is maintained across RTTs.  These are experiment switches
/// (`--k-confirm`, `--k-optimistic`, `--change-restart`); the defaults
/// reproduce the original behaviour.
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
    /// confirmed K is ruled out by disagreeing windows, or no link explains
    /// the belief's history, restart the belief at the longest consistent
    /// suffix of the history (`CongCtrlState::locate_change`).  On by
    /// default (`--no-change-restart`).
    pub change_restart: bool,
    /// Probe in Recover (`ProbeDelivery`, `--no-recover-probe`).
    pub recover_probe: bool,
}

impl Default for BeliefStrategy {
    fn default() -> Self {
        Self { k_confirm_rtts: K_CONFIRM_RTTS, k_optimistic: false, change_restart: true, recover_probe: true }
    }
}

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
    /// CLI override of `DRAIN_INTERVAL_RTTS`.
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

/// What the planner is doing this RTT.  Logged in the CSV.  The mode
/// selects the objective and the candidate actions; the search decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// C is loose (or its upper bound expired): learn C and B, losing at
    /// most the loss budget.
    #[default]
    Probe,
    /// C is tight: maximize utilization.
    Cruise,
    /// Prove the queue empty (for a min-RTT sample), otherwise maximize
    /// utilization.
    Drain,
    /// The link changed less than a QE window ago: match its delivery
    /// until the new link's windows exist.
    Recover,
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
    /// or above the delivery).
    pub last_probe: Option<usize>,
    /// Timestep at which the C interval last grew (`min_c` fell or `max_c`
    /// rose: a restart, a larger K, an expired `max_c`).  Recover's probes
    /// are timed from the later of this and `last_probe`.
    pub c_widened: Option<usize>,
    /// The C interval at the previous RTT.
    pub last_c_range: Option<(RealNumRep, RealNumRep)>,
    /// `belief_start` of the belief in which C was last tight: C has been
    /// tight since the belief (re)started if it equals `belief_start`.
    pub c_tight_in: Option<usize>,
    /// Timestep at which the belief last proved the queue empty.
    pub last_drained: usize,
    /// The belief proves the queue is at most `DRAIN_MAX_Q + K` now: RTT
    /// samples from this RTT measure the propagation delay (for a wrapper
    /// that tracks the min RTT).  Guaranteed at least every
    /// `DRAIN_INTERVAL_RTTS` unless a drain fails (`MAX_DRAIN_RTTS`).
    pub min_rtt_sample: bool,
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
    /// The loss budget over the loss window (`LOSS_WINDOW_RTTS`).
    pub loss_tolerance_abs: RealNumRep,
    pub delay_tolerance_frac: RealNumRep,
    pub objective_order: Vec<String>,
    /// Actions minimax may choose from.  With a single distinct rate the
    /// engine skips the search.
    pub candidate_actions: Vec<CandidateAction>,
    pub mode: Mode,
    /// The queue `DrainQueue` drains to, and below which `dq` counts a
    /// queue as drained.
    pub queue_target: RealNumRep,
    /// `MaxAllowedRate` is capped so that the worst-case queue (at
    /// `C = min_c`) stays within this.
    pub queue_cap: Option<RealNumRep>,
    /// What `state_policy` derived from the belief and its memory on the
    /// way to `mode` (logged by `--decisions` and `--external-json`).
    pub features: PlannerFeatures,
}

/// The planner's intermediate quantities for one committed RTT: everything
/// its mode and candidate set depend on.  Filled in by `state_policy`
/// before it returns; quantities it never reached keep their defaults.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlannerFeatures {
    /// The mode at the previous RTT (before this call).
    pub prev_mode: Mode,
    /// Observations since the belief (re)started.
    pub since_start: usize,
    pub newest_window_feasible: bool,
    pub k_max: RealNumRep,
    pub c_tight: bool,
    /// C has been tight since the belief (re)started: the loss budget is
    /// the constant one.
    pub c_known: bool,
    /// RTTs since the last probe or the last growth of the C interval,
    /// whichever is later (`None`: neither happened).
    pub since_probe: Option<usize>,
    pub queue_target: RealNumRep,
    pub drained: bool,
    pub drain_due: bool,
    pub drain_gave_up: bool,
    /// The worst-case queue cap (`MAX_QUEUE_BDP`).
    pub queue_cap: RealNumRep,
    pub loss_budget: RealNumRep,
    /// Recover only: a probe is allowed (the probe gap has passed).
    pub probe_ok: bool,
    pub probe_gap: usize,
}

impl Default for StatePolicy {
    fn default() -> Self {
        Self {
            belief_history_observations: QE_WINDOW_OBSERVATIONS,
            loss_tolerance_abs: 2.into(),
            delay_tolerance_frac: 3.into(),
            objective_order: full_order(OBJECTIVE_ORDER),
            candidate_actions: vec![CandidateAction::MaxAllowedRate],
            mode: Mode::Probe,
            queue_target: tun().drain_max_q,
            queue_cap: None,
            features: PlannerFeatures::default(),
        }
    }
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
    let hypotheses = match tun().k_policy() {
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
                    if sh.windows >= tun().shadow_windows {
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
        // `K_BELOW_BDP`: bursts are assumed smaller than a BDP, so a K that
        // the windows prove at least C (per RTT) is out.
        if tun().k_below_bdp && merged.max_c <= (*k).into() {
            info!("t={}: K={} ruled out (K < BDP, and C <= {})", timestep, k, merged.max_c);
            *alive = false;
            continue;
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
    if delivered >= planner.last_min_c * tun().informative_load {
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
    planner.app_limited = sent + tun().app_limited_slack < intended;
    planner.last_sent = sent;
    let probed = matches!(
        action,
        Some(CandidateAction::ProbeAboveLimit | CandidateAction::ProbeHalf | CandidateAction::ProbeDelivery)
    );
    if probed && !planner.app_limited {
        planner.last_probe = Some(timestep);
    }
}

/// Edit this function to make the CCA state-dependent.  It is called exactly
/// once before each minimax search, rather than at speculative tree nodes.
/// It is the long-horizon planner: it picks a `Mode`, and the mode picks
/// the objective and the actions minimax chooses among.
pub fn state_policy(state: &StateQuantities<'_>, planner: &mut PlannerState) -> StatePolicy {
    // The newest `c_expiry_rtts` RTTs' windows (the engine caches committed
    // windows); what older windows proved about `min_c` and B is retained by
    // the engine, while `max_c` expires.
    let n_obs = state.history.len();
    let since_start = n_obs.saturating_sub(planner.belief_start);
    let mut history_len = state.belief_history.map_or(since_start, |n| n.min(since_start));
    if tun().c_expiry_rtts > 0 {
        history_len = history_len.min(tun().c_expiry_rtts + QE_WINDOW_OBSERVATIONS - 1);
    }
    let history_len = history_len.clamp(1, n_obs);
    let belief = match (state.belief_bounds_for_history)(history_len) {
        Ok(belief) => belief,
        // Changes are detected per K, in `update_k`, and in `locate_change`.
        Err(conflict) => conflict.accumulated,
    };
    let t = state.timestep;
    let prev_min_c = planner.last_min_c;
    planner.last_min_c = belief.min_c;

    let k_max: RealNumRep = match tun().k_policy() {
        KPolicy::Fixed(k) => k,
        KPolicy::Hypotheses(_) => planner.k.assumed.iter().copied().max().unwrap_or(0),
    }
    .into();
    // Bursts of up to K blur each window's view of C, so the achievable
    // width grows with K.
    let c_tight = belief.max_c <= belief.min_c * tun().c_tight_ratio + k_max * tun().c_tight_per_k;
    if c_tight {
        planner.c_tight_in = Some(planner.belief_start);
    }
    let c_known = planner.c_tight_in == Some(planner.belief_start);
    // Recover's probes are timed from the last probe or from the last time
    // the C interval grew, whichever is later.
    if let Some((lo, hi)) = planner.last_c_range {
        if belief.min_c < lo || belief.max_c > hi {
            planner.c_widened = Some(t);
        }
    }
    planner.last_c_range = Some((belief.min_c, belief.max_c));
    let probe_ref = std::cmp::max(planner.last_probe, planner.c_widened);
    let since_probe = probe_ref.map_or(usize::MAX, |p| t - p);
    let mut features = PlannerFeatures {
        prev_mode: planner.mode,
        since_start,
        newest_window_feasible: state.newest_window_feasible,
        k_max,
        c_tight,
        c_known,
        since_probe: probe_ref.map(|p| t - p),
        ..Default::default()
    };

    // Sending well below `min_c` drains any real queue (by at least the
    // margin), so what `max_q` keeps after such RTTs is not a queue the CCA
    // can drain.  Sending exactly `min_c` would not do: at `C = min_c` a
    // standing queue would stay, and count as a floor.
    let base_target = tun().drain_max_q + k_max;
    let margin = std::cmp::max(prev_min_c * tun().quiet_margin, 1.into());
    if planner.last_sent + margin <= prev_min_c {
        planner.quiet_max_q.push_back(belief.max_q);
        if planner.quiet_max_q.len() > tun().queue_floor_rtts {
            planner.quiet_max_q.pop_front();
        }
    }
    let floor = if planner.quiet_max_q.len() >= tun().queue_floor_rtts {
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
    // A drain is due once no RTT has been a min-RTT sample (a queue at most
    // `base_target`, not the floor) for `drain_interval_rtts`: the min RTT
    // is probed explicitly even on a link with a floor.  A drain that ends
    // at the floor also resets the timer.
    planner.min_rtt_sample = belief.max_q <= base_target;
    if planner.min_rtt_sample || (planner.mode == Mode::Drain && drained) {
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
    let drain_gave_up = planner.mode == Mode::Drain && t >= planner.drain_started + tun().max_drain_rtts;
    if drain_gave_up {
        warn!("t={}: drain not proven after {} RTTs (max_q={})", t, tun().max_drain_rtts, belief.max_q);
        planner.last_drained = t;
    }
    features.queue_target = queue_target;
    features.drained = drained;
    features.drain_due = drain_due;
    features.drain_gave_up = drain_gave_up;

    let mode = if planner.mode == Mode::Drain && !drained && !drain_gave_up {
        Mode::Drain
    } else if drain_due && !drain_gave_up {
        Mode::Drain
    } else if !c_tight {
        Mode::Probe
    } else {
        Mode::Cruise
    };
    planner.mode = mode;
    // Fewer observations than a QE window since the link changed: QE
    // tables exist only for whole 7-observation windows, and every window
    // that reaches back past the change mixes two links, so the belief is
    // the chained C bounds alone and the queue is only bounded by what is
    // in flight (which includes up to an RTT of service lag), and the
    // model has no network moves to search over.  Match the delivery until
    // the new link's windows exist, and probe above it every
    // `RECOVER_PROBE_GAP_RTTS` (timed from the restart, which widened C): a
    // rise in C would otherwise stay hidden.
    if since_start < QE_WINDOW_OBSERVATIONS {
        planner.mode = Mode::Recover;
        let probe_ok = state.strategy.recover_probe && since_probe >= tun().recover_probe_gap_rtts;
        features.probe_ok = probe_ok;
        features.probe_gap = tun().recover_probe_gap_rtts;
        return StatePolicy {
            belief_history_observations: history_len,
            loss_tolerance_abs: tun().constant_loss_budget(),
            delay_tolerance_frac: 3.into(),
            objective_order: full_order(OBJECTIVE_ORDER),
            candidate_actions: vec![if probe_ok { CandidateAction::ProbeDelivery } else { CandidateAction::MatchDelivery }],
            mode: Mode::Recover,
            queue_target,
            queue_cap: None,
            features,
        };
    }
    // Bursts of up to K (or the floor) are part of the worst-case queue
    // that no rate drains, so they do not count against the cap.
    let queue_cap = belief.min_c * tun().max_queue_bdp + std::cmp::max(planner.queue_floor, k_max);
    features.queue_cap = queue_cap;
    if !state.newest_window_feasible {
        // No static link explains the newest window: there are no feasible
        // network moves to search over.  Drain on the (restarted) belief
        // until windows after the change exist.
        return StatePolicy {
            belief_history_observations: history_len,
            loss_tolerance_abs: tun().constant_loss_budget(),
            delay_tolerance_frac: 3.into(),
            objective_order: full_order(OBJECTIVE_ORDER),
            candidate_actions: vec![CandidateAction::DrainQueue],
            mode,
            queue_target,
            queue_cap: Some(queue_cap),
            features,
        };
    }

    // A BDP of loss while C is uncertain (it has not been tight since the
    // belief started: startup, or a detected change), otherwise a constant.
    let constant = tun().constant_loss_budget();
    let loss_budget = match mode {
        Mode::Probe if !c_known => std::cmp::max(belief.min_c * tun().uncertain_loss_bdp, constant),
        _ => constant,
    };
    features.loss_budget = loss_budget;
    use CandidateAction::*;
    // Listed lowest rate first (the search breaks an exact tie towards the
    // last; `sent` leaves no ties between distinct rates anyway).
    let (candidate_actions, objectives) = match mode {
        Mode::Probe => (vec![MaxAllowedRate, ProbeHalf, ProbeAboveLimit], PROBE_OBJECTIVE_ORDER),
        Mode::Cruise => (vec![DrainQueue, MaxAllowedRate, ProbeAboveLimit], CRUISE_OBJECTIVE_ORDER),
        Mode::Drain => (vec![DrainQueue, DrainHalf, MaxAllowedRate], DRAIN_OBJECTIVE_ORDER),
        Mode::Recover => unreachable!("Recover returns early"),
    };

    StatePolicy {
        belief_history_observations: history_len,
        loss_tolerance_abs: loss_budget,
        delay_tolerance_frac: 3.into(),
        objective_order: full_order(objectives),
        candidate_actions,
        mode,
        queue_target,
        queue_cap: Some(queue_cap),
        features,
    }
}
