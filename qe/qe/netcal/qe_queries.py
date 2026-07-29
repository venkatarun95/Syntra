import multiprocessing
import os
import time
from typing import List, Type

import z3  # type: ignore

from qe.environment import Ideal
from qe.netcal.common import (
    QEQuery,
    get_function_name,
    get_output_filename,
    get_string_list,
)
from qe.util import qe_simplify


# TODO: test if everything works well when we have small enough trace lengths
# After that we can see longer traces.


def lhs_specific_constraints(
    c: Ideal.Config, v: Ideal.Variables, lhs: str
) -> List[z3.BoolRef]:
    ret = []
    T = c.T
    bq = [v.A[t] - v.L[t] - v.I[t] for t in range(T)]

    if lhs == f"Q_{T-1}":
        bq_var: z3.ExprRef = z3.Real(f"Q_{T-1}")
        ret.append(bq_var == bq[T - 1])

    elif lhs.startswith(f"A_{T-1}"):
        # Max ACK rate for CCA

        # The extra constraints encode the bad behavior. The QE query is then,
        # we want bounds on A[-1] such that for all unobserved, there is no
        # solution to "feasible \land bad".

        queue_tol_bdp_str = lhs.replace(f"A_{T-1}_", "")
        queue_tol_bdp = int(queue_tol_bdp_str)
        loss_rate_tol = z3.Real("loss_rate_tol")  # loss_rate = losses / time duration

        # TODO: update this to be able to compute max allowed rate for
        # different probing durations.
        duration = 1
        first = T - 1 - duration
        last = T - 1
        # Note, above the first A is selected at time index first+1, that is
        # why we have first+1 below.

        # This makes sense as our current action affects losses from first+1 to
        # last so we restrict those losses.

        good = []
        for t in range(first + 1, last + 1):
            good.append(v.L[t] - v.L[t - 1] <= loss_rate_tol)

        # If we keep queue_tol_bdp as Real, then non-linearity inflates solving
        # time, so substituting with a constant.
        for t in range(first + 1, last + 1):
            good.append(bq[t] <= v.C * (c.R * queue_tol_bdp))

        bad = z3.Not(z3.And(good))
        assert isinstance(bad, z3.BoolRef)
        ret.append(bad)

    return ret


def write_expr_smt2_file(expr: z3.ExprRef, fname: str):
    with open(fname, "w") as f:
        dummy = z3.Solver()
        dummy.add(expr)
        f.write(dummy.to_smt2())


def write_expr_file(expr: z3.ExprRef, fname: str):
    with open(fname, "w") as f:
        f.write(str(expr))


def run_query_parallel(
    network_model: Type[Ideal],
    output_dir: str,
    T: int,
    eliminate: List[str],
    lhs: str,
    ideal: bool,
    n_losses_observed: int,
    last_observed_s: int,
    sim: bool,
    newly_obs_s: int,
    newly_obs_l: int,
):
    assert n_losses_observed >= 0
    assert last_observed_s >= 0

    fname = get_output_filename(T, eliminate, lhs, ideal, -2 if newly_obs_l == -2 else n_losses_observed)
    fpath = os.path.join(output_dir, fname)
    _, fn_name = get_function_name(lhs, T, n_losses_observed, sim, ideal, newly_obs_l)
    print("For fn: ", fn_name)

    if os.path.exists(fpath):
        print("Skipping: ", T, eliminate, lhs)
        return

    print("Running: ", T, eliminate, lhs)

    start = time.time()

    c = network_model.Config(T=T)
    m = network_model(name="", c=c)
    v = m.v

    cl = m.all_constraints()

    extra_model = []
    if ideal:
        for t in range(c.T):
            extra_model.append(v.S[t] == v.I[t])

    obs_constraints = []
    # Used for all queries
    first_unobserved_l = n_losses_observed
    if first_unobserved_l <= last_observed_s:
        obs_constraints.append(v.A[first_unobserved_l] - v.L[first_unobserved_l] > v.S[last_observed_s])
    # TODO: Rename to last_observed_l
    first_observed_l = first_unobserved_l - 1
    if first_observed_l >= 0 and first_observed_l <= last_observed_s:
        obs_constraints.append(v.A[first_observed_l] - v.L[first_observed_l] <= v.S[last_observed_s])

    # Prior observed losses are observed according to last_observed_s.
    # Where as newly observed are observed according to newly_obs_s.
    # This is why we differentiate old and newly obs losses.
    if newly_obs_s != -1 and newly_obs_l >= 0:
        # Used when computing future loss observations when already some loss
        # observations have been computed for the currently computed S.
        assert newly_obs_s >= newly_obs_l
        obs_constraints.append(v.A[newly_obs_l] - v.L[newly_obs_l] <= v.S[newly_obs_s])

    extra = lhs_specific_constraints(c, v, lhs)
    feasible = z3.And(cl + extra + extra_model + obs_constraints)
    assert isinstance(feasible, z3.BoolRef)

    g = z3.Goal()
    remove: List[z3.ExprRef] = [z3.Real(x) for x in eliminate]
    print("Eliminating: ", remove)
    g.add(z3.Exists(remove, feasible))

    # Write inputs (mostly for debugging)
    input_smt2 = os.path.join(output_dir, "input_" + fname)
    input_human = os.path.join(output_dir, "input_" + fname + ".z3")
    write_expr_smt2_file(z3.Exists(remove, feasible), input_smt2)
    write_expr_file(g, input_human)

    # Main slow call
    ret = qe_simplify(g)

    print("Writing: ", fpath)
    write_expr_smt2_file(ret, fpath)

    end = time.time()
    print(f"Done in {end-start:.2}s: ", T, eliminate)


def run_all_queries(network_model: Type[Ideal], output_dir: str, T: int, queries: List[QEQuery]):
    pool = multiprocessing.Pool(64)
    futures = []

    # Since we check if output file already exists, if two queries produce same
    # output file, we'd skip executing the query anyway.

    for q in queries:
        eliminate = get_string_list(q.eliminate)
        f = pool.apply_async(
            func=run_query_parallel,
            args=(
                network_model,
                output_dir,
                T,
                eliminate,
                q.lhs,
                q.ideal,
                q.n_losses_observed,
                q.last_observed_s,
                q.sim,
                q.newly_obs_s,
                q.newly_obs_l,
            ),
        )
        futures.append(f)

    # Since the individual processes write to disk, we don't need to do
    # anything with the futures.
    pool.close()
    print("Sleeping 5 seconds")  # In case everything is finished but there is slight lag is function returning.
    time.sleep(5)
    start = time.time()
    while True:
        remaining = 0
        for f in futures:
            if not f.ready():
                remaining += 1
            else:
                if not f.successful():
                    print("Process raised exception: ", f.get())

        if remaining == 0:
            break
        now = time.time()
        print("Remaining tasks: ", remaining, end=", ")
        print("Elapsed time: ", now - start)
        time.sleep(60 * 5)

    pool.join()


def solve_queries(network_model: Type[Ideal], output_dir: str, T: int, queries: List[QEQuery]):
    os.makedirs(output_dir, exist_ok=True)
    run_all_queries(network_model, output_dir, T, queries)
