import argparse
import os

from qe.netcal import qe_queries, transpile
from qe.netcal.common import OUTPUT_DIR as DEFAULT_OUTPUT_DIR
from qe.netcal.query_config import QUERY_CONFIGS, get_query_config


class Main:
    """
    Entry point for the QE pipeline. Owns the query list for a given
    T/config and dispatches to the solve stage (qe_queries), the codegen
    stage (transpile), or both, depending on --mode.
    """

    def __init__(
        self,
        T: int,
        queue_tol_bdp: int,
        mode: str,
        config_name: str,
        output_dir: str,
        input_dir: str | None,
        output_path: str | None,
    ):
        self.T = T
        self.queue_tol_bdp = queue_tol_bdp
        self.mode = mode
        # Where the solve stage writes solved SMT2 files.
        self.output_dir = output_dir
        # Where the transpile stage reads solved SMT2 files from.
        # A normal `--mode all --output-dir X` run should consume precisely
        # the SMT files it just produced.  The old independent defaults made
        # that surprisingly easy to get wrong.
        self.input_dir = input_dir if input_dir is not None else output_dir
        # Where the transpile stage writes the generated Rust file.
        self.output_path = (
            output_path if output_path is not None else os.path.join(self.input_dir, "lib.rs")
        )
        os.makedirs(os.path.dirname(self.output_path) or ".", exist_ok=True)
        self.config = get_query_config(config_name)
        self.queries = self.config.generate_queries(T, queue_tol_bdp)
        print("Total queries: ", len(self.queries))

    def solve(self):
        qe_queries.solve_queries(self.config.network_model, self.output_dir, self.T, self.queries)

    def transpile(self):
        transpile.transpile_all(self.config.network_model, self.input_dir, self.output_path, self.T, self.queries)

    def run(self):
        if self.mode in ("solve", "all"):
            self.solve()
        if self.mode in ("transpile", "all"):
            self.transpile()


def get_args():
    parser = argparse.ArgumentParser(description="QE pipeline: solve queries and/or transpile results to Rust")
    parser.add_argument("-t", "--tsteps", action="store", type=int, default=5)
    parser.add_argument("--queue-tol-bdp", action="store", type=int, default=3)
    parser.add_argument(
        "--mode", choices=["solve", "transpile", "all"], default="all",
        help="solve: run the parallel QE queries only. "
             "transpile: read already-solved results and generate Rust only. "
             "all: solve then transpile (default).",
    )
    parser.add_argument(
        "--config", choices=list(QUERY_CONFIGS), default="cbrdelay",
        help="Which network model + query generation strategy to use.",
    )
    parser.add_argument(
        "--output-dir", action="store", type=str, default=DEFAULT_OUTPUT_DIR,
        help="Where the solve stage writes solved SMT2 query results.",
    )
    parser.add_argument(
        "--input-dir", action="store", type=str,
        help="Where the transpile stage reads solved SMT2 query results from. "
             "Defaults to --output-dir.",
    )
    parser.add_argument(
        "--output-path", action="store", type=str,
        help="Where the transpile stage writes the generated Rust file. "
             "Defaults to <input-dir>/lib.rs.",
    )
    return parser.parse_args()


if __name__ == "__main__":
    args = get_args()
    Main(
        args.tsteps, args.queue_tol_bdp, args.mode, args.config,
        args.output_dir, args.input_dir, args.output_path,
    ).run()
