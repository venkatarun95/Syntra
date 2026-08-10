# Syntra QE Workbench

Build the CBR-delay QE WebAssembly module, then serve this directory:

```bash
cd /Users/venkatar/Projects/Syntra
cargo build --target wasm32-unknown-unknown --release \
  --manifest-path visualizer/cbrdelay-qe-wasm/Cargo.toml

cd /Users/venkatar/Projects/Syntra/visualizer
python3 -m http.server 8080
```

Open `http://localhost:8080`. The CBR QE plot runs in the browser from the
same-origin WebAssembly module—there is no local HTTP bridge or CORS setup.
It samples the generated QE functions conditionally: `C`, then `B | C`, then
`Q | C,B`, using the editable arrival, service, and cumulative-loss curves.

The workbench accepts a JSON trace shaped as follows:

```json
{
  "schema_version": 1,
  "models": ["bursty_cbrdelay_elim_k"],
  "steps": [
    {"t": 0, "arrival": 10, "service": 0, "loss": 0},
    {"t": 1, "arrival": 20, "service": 8, "loss": 0}
  ],
  "beliefs": {
    "bursty_cbrdelay_elim_k": {
      "capacity": [5, 30], "buffer": [5, 40], "queue": [0, 12]
    }
  },
  "events": [
    {"t": 1, "cca_action": {"rate": 10}, "candidate_actions": [5, 10, 15],
     "network_action": {"service": 8, "loss": 0}, "objective": [0, 1]}
  ]
}
```

`beliefs` is optional. It is the QE-produced belief state recorded by the
simulation. The editor itself is plain JavaScript: it carries out a local,
epsilon-relaxed (`0.001`) projection of the CBR-delay constraints while you
drag a point. No Wasm module is needed for that interaction.

For `cbrdelay`, the right pane also draws a sampled `C × B × Q` belief
geometry. `C` is capacity, `B` is buffer, and `Q` is the latent queue at the
selected time. If a trace contains `beliefs.cbrdelay`, those QE intervals set
the plot ranges; otherwise the current trace provides reasonable exploratory
ranges. The perturbation value `K` is a trace-editor control and is shown in
the perturbed-model diagram; it does not require QE merely to visualize it.

The current local projector hard-codes the CBR-delay family and selects the
least-service representative for the model's nondeterministic initial service.
That makes the diagram concrete while retaining a valid `D = 1` delay envelope
(`0…1` RTT of added delay). QE remains the source of authoritative belief
intervals.
