import os
from dataclasses import dataclass
from typing import List, Tuple

import z3  # type: ignore

# Shared between qe_queries (solve stage) and transpile (codegen stage), so
# neither module needs to import the other. Which network model to use is a
# QueryConfig concern (see query_config.py), not hardcoded here.

THIS_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(os.path.dirname(THIS_SCRIPT_DIR))
OUTPUT_DIR = os.path.join(REPO_ROOT, "outputs/qe/cbrdelay_l0_t6/")
OUTPUT_PATH = os.path.join(OUTPUT_DIR, "lib.rs")


def get_string_list(eliminate: List[z3.ExprRef] = []):
    return [str(e) for e in eliminate]


def get_output_filename(T: int, eliminate: List[str], lhs: str, ideal: bool, prior_l_obs: int):
    lhs_tag = f",lhs={lhs}"
    ideal_tag = ",ideal" if ideal else ""
    prior_l_obs_tag = f",pl={prior_l_obs}" if prior_l_obs >= 0 else ""
    return f'T={T},eliminate=[{",".join(eliminate)}{lhs_tag}{ideal_tag}]{prior_l_obs_tag}.smt2'


def get_function_name(lhs: str, T: int, n_losses_observed: int, sim: bool, ideal: bool, newly_obs_l: int) -> Tuple[str, str]:
    sim_tag = "sim_" if sim else ""
    ideal_tag = "ideal_" if ideal else ""
    fn_prefix = f"compute_{sim_tag}{ideal_tag}{lhs.lower()}"
    newly_obs_l_tag = f"_nl_{newly_obs_l}" if newly_obs_l >= 0 else ""
    return fn_prefix, f"{fn_prefix}_t_{T}_l_{n_losses_observed}{newly_obs_l_tag}"


@dataclass
class QEQuery:
    lhs: str
    eliminate: List[z3.ExprRef]
    n_losses_observed: int
    last_observed_s: int
    sim: bool = False
    ideal: bool = False
    newly_obs_s: int = -1  # -1 means unspecified.
    newly_obs_l: int = -2  # -2 means unspecified, -1 means specified and no new losses.
