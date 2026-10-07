//! Dispatch from (trace length, losses observed) to a generated QE function.
//!
//! The network model is identical for every QE backend; only the generated
//! functions differ.  Each lookup returns `None` when that (T, losses) key was
//! never solved, so the caller can fall back to a different trace length
//! instead of panicking inside a `HashMap` index.

use std::fmt::Debug;

use ds::interval::IntervalList;
use ds::*;

type Bounds = Option<IntervalList<RealNumRep>>;

pub trait QeTables: Debug + Send + Sync {
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

/// CBR-delay behind a sender whose arrivals may be perturbed by up to `k`
/// bytes (`network_model_bursty_qe_output`).  QE leaves `k` and `pert` (the
/// bound `k < pert * B`) free, so they are supplied here as fixed values.
#[derive(Debug)]
pub struct BurstyQe {
    pub k: RealNumRep,
    pub pert: RealNumRep,
}

impl QeTables for BurstyQe {
    fn c(&self, t: i32, n: i32, a: &[RealNumRep], l: &[RealNumRep], s: &[RealNumRep], l0: RealNumRep)
        -> Bounds {
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
