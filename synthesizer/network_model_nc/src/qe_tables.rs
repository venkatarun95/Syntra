//! Dispatch from (trace length, losses observed) to a generated QE function.
//!
//! The network model is identical for every QE backend; only the generated
//! functions differ.  Each lookup returns `None` when that (T, losses) key was
//! never solved, so the caller can fall back to a different trace length
//! instead of panicking inside a `HashMap` index.

use std::cell::Cell;
use std::fmt::Debug;
use std::ops::Bound;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

use ds::interval::IntervalList;
use ds::*;

type Bounds = Option<IntervalList<RealNumRep>>;

pub trait QeTables: Debug + Send + Sync {
    /// The burstiness values this evaluator assumes (0 for the plain link).
    fn k_values(&self) -> Vec<RealNumRep> {
        vec![0.into()]
    }

    fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
        -> Bounds;

    /// The constraints on C alone within the solved B query.  Unlike `c`,
    /// this need not be the tightest bound (B is not eliminated), but C
    /// values inside it are safe to pass to `b` at the same key.
    fn c_from_b(
        &self,
        _t: i32,
        _n: i32,
        _a: &[RealNumRep],
        _l: &[RealNumRep],
        _s: &[RealNumRep],
        _l0: RealNumRep,
    ) -> Bounds {
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
    ) -> Bounds;

    /// Bounds on the queue at the end of the trace.
    #[allow(clippy::too_many_arguments)]
    fn q(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds;

    /// Bounds on the next (not yet observed) service.
    #[allow(clippy::too_many_arguments)]
    fn s(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds;

    /// Bounds on loss `total`, given `prior` previously observed losses.
    #[allow(clippy::too_many_arguments)]
    fn l_obs(
        &self,
        t: i32,
        prior: i32,
        total: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds;
}

/// The original CBR-delay model (`network_model_nc_qe_output`).
#[derive(Debug)]
pub struct PlainQe;

impl QeTables for PlainQe {
    fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
        -> Bounds {
        network_model_nc_qe_output::COMPUTE_C
            .get(&(t, n))
            .map(|f| f(a, l, s, l0))
    }

    fn b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
    ) -> Bounds {
        network_model_nc_qe_output::COMPUTE_B
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, c))
    }

    fn q(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        crate::COMPUTE_Q.get(&(t, n)).map(|f| f(a, l, s, l0, c, b))
    }

    fn s(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        crate::COMPUTE_S.get(&(t, n)).map(|f| f(a, l, s, l0, c, b))
    }

    fn l_obs(
        &self,
        t: i32,
        prior: i32,
        total: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        crate::COMPUTE_L_OBS
            .get(&(t, prior, total))
            .map(|f| f(a, l, s, l0, c, b))
    }
}

/// (T, losses) keys of `network_model_bursty_qe_output::COMPUTE_C` that were
/// checked to come from the bursty model.  Add a key once its query input
/// mentions K (`P_t >= A_t - K`).
pub const BURSTY_C_RESOLVED: &[(i32, i32)] = &[];

/// CBR-delay behind a sender whose arrivals may be perturbed by up to `k`
/// bytes (`network_model_bursty_qe_output`).  QE leaves `k` and `pert` (the
/// bound `k < pert * B`) free, so they are supplied here as fixed values.
#[derive(Debug)]
pub struct BurstyQe {
    pub k: RealNumRep,
    pub pert: RealNumRep,
}

impl QeTables for BurstyQe {
    fn k_values(&self) -> Vec<RealNumRep> {
        vec![self.k]
    }

    fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
        -> Bounds {
        // The only C result currently generated, T=5 with 1 loss, was
        // solved from a query without the burst constraints (its input in
        // qe/bursty_cbrdelay_T6/ uses A instead of P and has no K): it is a
        // plain CBR-delay bound, tighter than the bursty one, and excludes
        // the true C of bursty traces.  Ignore it until it is re-solved; see
        // `BURSTY_C_RESOLVED`.
        if !BURSTY_C_RESOLVED.contains(&(t, n)) {
            return None;
        }
        network_model_bursty_qe_output::COMPUTE_C
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, self.k, self.pert))
    }

    fn c_from_b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> Bounds {
        network_model_bursty_qe_output::COMPUTE_C_FROM_B
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, self.k, self.pert))
    }

    fn b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
    ) -> Bounds {
        network_model_bursty_qe_output::COMPUTE_B
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, c, self.k, self.pert))
    }

    fn q(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        // Generated tables are named by the queried index, Q_{T-1}; only
        // T = 6 has been solved for the bursty model.
        let table = match t {
            6 => &*network_model_bursty_qe_output::COMPUTE_Q_5,
            _ => return None,
        };
        table
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, c, b, self.k, self.pert))
    }

    fn s(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        let table = match t {
            6 => &*network_model_bursty_qe_output::COMPUTE_S_5,
            _ => return None,
        };
        table
            .get(&(t, n))
            .map(|f| f(a, l, s, l0, c, b, self.k, self.pert))
    }

    fn l_obs(
        &self,
        t: i32,
        prior: i32,
        total: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        use network_model_bursty_qe_output as g;
        // COMPUTE_L_{total} is keyed by (T, prior losses observed).
        let table = match total {
            0 => &*g::COMPUTE_L_0,
            1 => &*g::COMPUTE_L_1,
            2 => &*g::COMPUTE_L_2,
            3 => &*g::COMPUTE_L_3,
            4 => &*g::COMPUTE_L_4,
            5 => &*g::COMPUTE_L_5,
            _ => return None,
        };
        table
            .get(&(t, prior))
            .map(|f| f(a, l, s, l0, c, b, self.k, self.pert))
    }
}

/// The bursty model when the burstiness is only known to be one of `ks`:
/// every bound is the union of the per-K bounds.  Feasibility is not
/// monotone in K (bursts rule out small K, a small buffer rules out large
/// K), so no single K is conservative; the union is.  Functions taking B
/// only consult the K with `K < pert * B`, the constraint every bursty
/// result assumes.  A lookup is `None` only if it is `None` for every K.
#[derive(Debug)]
pub struct UnionBurstyQe {
    pub ks: Vec<RealNumRep>,
    pub pert: RealNumRep,
}

impl UnionBurstyQe {
    /// Union of `f` over every K whose preconditions hold.  A generated
    /// function asserts the model constraints that do not involve its
    /// result, i.e. that its (C, B) inputs are feasible for its K.  A (C, B)
    /// from the union belief can be feasible for one K and not another; such
    /// a K contributes nothing, so its assertion failure is skipped.
    fn union(&self, b: Option<RealNumRep>, f: impl Fn(&BurstyQe) -> Bounds) -> Bounds {
        self.ks
            .iter()
            .filter(|&&k| b.map_or(true, |b| k < self.pert * b))
            .filter_map(|&k| quietly_catch(|| f(&BurstyQe { k, pert: self.pert })).flatten())
            .reduce(|acc, x| acc.union(&x))
    }
}

thread_local! {
    static QUIET_PANICS: Cell<bool> = const { Cell::new(false) };
}
static QUIET_HOOK: Once = Once::new();

/// Run `f`, returning `None` instead of unwinding (and without printing the
/// panic message) if it panics.
pub(crate) fn quietly_catch<T>(f: impl FnOnce() -> T) -> Option<T> {
    QUIET_HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !QUIET_PANICS.with(Cell::get) {
                previous(info)
            }
        }));
    });
    QUIET_PANICS.with(|q| q.set(true));
    let ret = panic::catch_unwind(AssertUnwindSafe(f));
    QUIET_PANICS.with(|q| q.set(false));
    ret.ok()
}

impl QeTables for UnionBurstyQe {
    fn k_values(&self) -> Vec<RealNumRep> {
        self.ks.clone()
    }

    fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
        -> Bounds {
        // A K whose C is unsolved would make the union unsound, so either
        // every K has this C result or none is used.
        let per_k: Vec<Bounds> = self
            .ks
            .iter()
            .map(|&k| BurstyQe { k, pert: self.pert }.c(t, n, a, l, s, l0))
            .collect();
        if per_k.iter().any(Option::is_none) {
            return None;
        }
        per_k.into_iter().flatten().reduce(|acc, x| acc.union(&x))
    }

    fn c_from_b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
    ) -> Bounds {
        self.union(None, |q| q.c_from_b(t, n, a, l, s, l0))
    }

    fn b(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
    ) -> Bounds {
        self.union(None, |q| q.b(t, n, a, l, s, l0, c))
    }

    fn q(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        self.union(Some(b), |q| q.q(t, n, a, l, s, l0, c, b))
    }

    fn s(
        &self,
        t: i32,
        n: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        self.union(Some(b), |q| q.s(t, n, a, l, s, l0, c, b))
    }

    fn l_obs(
        &self,
        t: i32,
        prior: i32,
        total: i32,
        a: &[RealNumRep],
        l: &[RealNumRep],
        s: &[RealNumRep],
        l0: RealNumRep,
        c: RealNumRep,
        b: RealNumRep,
    ) -> Bounds {
        // The newest service may have been chosen from another K's range;
        // only a K that allows it may explain the losses, or the resulting
        // observation would be feasible for no K at all.
        let newest = IntervalList::interval_point(Bound::Included(*s.last().unwrap()));
        let (s_prefix, l_prior) = (&s[..s.len() - 1], &l[..prior as usize]);
        self.union(Some(b), |q| {
            let allows_s = q
                .s(t, prior, a, l_prior, s_prefix, l0, c, b)
                .is_some_and(|range| !range.intersection(&newest).is_empty());
            if allows_s {
                q.l_obs(t, prior, total, a, l, s, l0, c, b)
            } else {
                None
            }
        })
    }
}
