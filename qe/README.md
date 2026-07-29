# qe

Symbolic derivation of network-belief bounds for congestion control, via z3
quantifier elimination (QE). Given a network model (e.g. a constant-bitrate
link with delay) and a set of observations (arrivals, service, losses), this
computes closed-form bounds on unobserved quantities (link rate `C`, buffer
size `B`, losses `L`, etc.) and compiles them into Rust functions that the
`synthesizer` crate's minimax search calls at runtime.

## Requirements

- Python 3 with `z3-solver` (must include the `qe2` tactic), `numpy`
- `rustfmt` on `PATH` (used to format the generated Rust output)

## Quick start

Run from this directory (`qe/`):

```bash
python3 -m qe.main
```

This solves and transpiles with all defaults: `T=5`, the `cbrdelay` network
model, writing solved SMT2 queries to and reading them back from
`outputs/qe/cbrdelay_l0_t6/`, and writing the generated Rust to
`outputs/qe/cbrdelay_l0_t6/lib.rs`.

To reproduce the T=6 model the `synthesizer` crate currently expects:

```bash
python3 -m qe.main -t 6
```

### CLI flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `-t`, `--tsteps` | `5` | Trace length `T` |
| `--queue-tol-bdp` | `3` | Queueing-delay tolerance (in BDPs); accepted by `generate_queries` but currently unused by the registered `cbrdelay` config — the CCA rate-bound query type that consumed it was dropped from query generation. Kept for now since `lhs_specific_constraints` in `qe_queries.py` still has the matching (currently unreachable) code path |
| `--mode` | `all` | `solve` (run the parallel QE queries only), `transpile` (read already-solved results and generate Rust only), or `all` (both, in order) |
| `--config` | `cbrdelay` | Which network model + query-generation strategy to use — see [Query configs](#query-configs) |
| `--output-dir` | `outputs/qe/cbrdelay_l0_t6/` | Where the solve stage writes solved SMT2 query results |
| `--input-dir` | `outputs/qe/cbrdelay_l0_t6/` | Where the transpile stage reads solved SMT2 query results from |
| `--output-path` | `outputs/qe/cbrdelay_l0_t6/lib.rs` | Where the transpile stage writes the generated Rust file |

`--output-dir` and `--input-dir` are separate flags so you can transpile
against a directory of results produced by a different run (or machine)
without re-solving:

```bash
python3 -m qe.main --mode solve -t 6 --output-dir /scratch/t6-results
python3 -m qe.main --mode transpile -t 6 --input-dir /scratch/t6-results --output-path /tmp/lib.rs
```

## How it works

The pipeline has two independent stages, joined only by `main.py`:

1. **Solve** (`qe/netcal/qe_queries.py`) — for each query (a choice of which
   variables to existentially quantify away, and which left-hand-side
   variable to derive bounds for), runs z3's QE tactics in parallel
   (`multiprocessing.Pool`) and writes the simplified result as an `.smt2`
   file into `--output-dir`. Already-solved queries (file already exists)
   are skipped, so re-running is incremental.
2. **Transpile** (`qe/netcal/transpile.py`) — reads each solved `.smt2` file
   from `--input-dir`, converts the z3 CNF into a Rust function that computes
   an `IntervalList` bound on the query's left-hand-side variable, and
   appends it to the generated `--output-path` (a single `lib.rs`). All
   generated functions are also registered in per-quantity `HashMap<(T,
   n_losses_observed), fn_ptr>` lookup tables so `synthesizer` can dispatch
   on trace length and how many losses have been observed.

Neither module imports the other or hardcodes a network model — both take a
`network_model` and take their solved/query file locations as parameters.
`qe/netcal/main.py`'s `Main` class is the only place that wires a
`QueryConfig`, a `T`, and file locations together and calls both stages.

`qe/netcal/qe_queries.py` has no `__main__` — it's a library only callable
from `main.py`. `transpile.py` keeps a small `__main__` for ad hoc manual
testing of the expression-transpiling helpers.

## Query configs

`qe/netcal/query_config.py` is the one place a network model and its query
generation logic are wired together, so `qe_queries`/`transpile` never
hardcode a specific model. A `QueryConfig` bundles:

- `network_model` — a network model class (see [Network models](#network-models))
- `generate_queries(T, queue_tol_bdp) -> List[QEQuery]` — builds the list of
  QE queries to solve for that model at a given trace length

Registered configs live in `QUERY_CONFIGS` and are looked up by name via
`get_query_config(name)`; `main.py --config` exposes exactly this registry
(so adding an entry automatically becomes a valid `--config` value with no
other code changes needed). Currently registered:

- `cbrdelay` — the constant-bitrate-with-delay network model

## Network models

`qe/environment/` defines the network models the QE queries are built
against:

- `Ideal` (`ideal.py`) — the base model: arrival/loss/service curves with
  no queueing delay (`I` aliases `S`)
- `CBRDelay` (`cbr_delay.py`) — `Ideal` plus a constant per-hop delay `D`
  before the ideal service curve is realized in `S`, with optional
  queueing-delay (`qdel`) and per-hop-delay (`hop_delay`) instrumentation

Each model exposes one class with nested `Config` (a dataclass of model
parameters), `Variables` (the z3 symbols for one trace), and `Constraints`
(the methods that build z3 constraints over those symbols). Construct one
and get everything at once — no need to build `Variables`/`Constraints`
separately and wire them together:

```python
from qe.environment import CBRDelay

m = CBRDelay(name="", c=CBRDelay.Config(T=6, use_loss_detect=True))
constraints = m.all_constraints()
v = m.v  # v.A, v.S, v.L, v.C, v.B, ...
```

Models subclass each other the same way their `Config`/`Variables`/
`Constraints` do (`CBRDelay(Ideal)`, with `CBRDelay.Constraints(Ideal.Constraints)`
etc.), so a subclass only needs to define what's different from its parent.

## Output layout

`outputs/qe/<config>_l0_t<T>/` holds one `.smt2` file per solved query
(named by trace length, eliminated variables, and left-hand-side variable)
plus `lib.rs`, the generated Rust consumed by `synthesizer`'s
`network_model_nc_qe_output` crate. These are build artifacts — regenerate
them with `python3 -m qe.main` rather than hand-editing.
