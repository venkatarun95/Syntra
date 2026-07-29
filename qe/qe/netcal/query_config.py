from dataclasses import dataclass
from typing import Callable, Dict, List, Type

from qe.environment import CBRDelay, Ideal
from qe.netcal.common import QEQuery
from qe.util import flatten


@dataclass
class QueryConfig:
    """
    Bundles which network model to solve queries against with how to
    generate the query list for it. qe_queries and transpile only depend on
    this abstraction, never on a specific network model directly.
    """
    name: str
    network_model: Type[Ideal]
    generate_queries: Callable[[int, int], List[QEQuery]]


def _cbrdelay_generate_queries(T: int, queue_tol_bdp: int) -> List[QEQuery]:
    c = CBRDelay.Config(T=T)
    v = CBRDelay(name="", c=c).v
    # Each query is a list of variables to eliminate

    queries: List[QEQuery] = []
    for st in range(T + 1):
        # st starts from 0, as no losses may have been observed
        queries.append(QEQuery(
            "C", flatten([v.I, v.L[st:], v.B]), st, T-1))

        queries.append(QEQuery(
            "B", flatten([v.I, v.L[st:]]), st, T-1))

        queries.append(QEQuery(
            f"Q_{T-1}", flatten([v.I, v.L[st:]]), st, T-1))

        if st < T:
            queries.append(QEQuery(
                f"S_{T-1}", flatten([v.I, v.L[st:]]), st, T-2))
            # We may be able to observe last L depending on the trace and
            # actions choices. So explore the case where none of the losses
            # are eliminated.

    for total_prior_obs_losses in range(T):
        # starts from 0, as no losses may have been observed
        for newly_obs_losses in range(0, total_prior_obs_losses + 1):
            # All the observed losses may be new.
            total_loss_vars = total_prior_obs_losses + 1  # +1 for the lhs
            non_newly_obs_losses = total_prior_obs_losses - newly_obs_losses

            newly_obs_l_idx = total_loss_vars - 1 - 1  # -1 for lhs and -1 for 0-indexing
            # This is basically idx of loss before the loss being computed.
            # Since we are computing current loss, the previous one must have
            # been observed. Think of this as the last observed loss including
            # newly observed loss.

            newly_obs_s_idx = T - 1

            queries.append(
                QEQuery(f"L_{total_loss_vars-1}", flatten([v.I, v.L[total_loss_vars:]]),
                        non_newly_obs_losses, T-2, False, False, newly_obs_s_idx,
                        newly_obs_l_idx,))

    return queries


CBRDELAY = QueryConfig(
    name="cbrdelay",
    network_model=CBRDelay,
    generate_queries=_cbrdelay_generate_queries,
)


QUERY_CONFIGS: Dict[str, QueryConfig] = {
    CBRDELAY.name: CBRDELAY,
}


def get_query_config(name: str) -> QueryConfig:
    if name not in QUERY_CONFIGS:
        raise ValueError(f"Unknown query config {name!r}, expected one of {list(QUERY_CONFIGS)}")
    return QUERY_CONFIGS[name]
