# Simulation Stack Epic Plan

**Goal:** Turn the existing simulation-stack design and partial implementation into an execution-grade epic that reflects what is already verified, what is still stubbed, and what must ship to make the stack real.

**Current Verified State:** The PyO3 bridge, Python simulation scaffold, world-delta transport, sim event transport, election input producer, and broad simulation tests exist. The remaining gaps are concentrated in four areas: Rust-side consumption of simulation outputs, real external model integrations, end-to-end validation/benchmarking, and GDD/documentation reconciliation.

## Status Snapshot

- Verified and closed:
  - PyO3 bridge and Python project scaffold
  - SimEvent bridge transport
  - WorldStateDelta transport
  - Election input producer on the Python side
- Open because implementation is still partial or stand-in:
  - `polit-4gvu` Rust election consumer seam
  - `polit-a12w` real `rti_synth_pop` integration
  - `polit-93ic` real PolicyEngine US integration
  - `polit-0bvl` real PyFRB/US integration
  - `polit-etef` dedicated long-run integration and benchmark coverage
  - `polit-ij82` GDD updates to match the final architecture

## Epic Shape

Use `polit-2pn` as the execution epic.

Direct child workstreams:

1. Runtime Consumerization
   - `polit-4gvu`
   - Purpose: prove Rust actually consumes simulation election outputs through a typed seam.

2. Real Population + Household Integration
   - `polit-a12w`
   - `polit-93ic`
   - Purpose: replace county-profile stand-ins with real bootstrap + household policy computation.

3. Real Macro Backbone Integration
   - `polit-0bvl`
   - Purpose: replace simplified macro drift with a real PyFRB/US-backed path.

4. Validation + Performance Gate
   - `polit-etef`
   - Purpose: prove the stack stays bounded, cross-language contracts hold, and Dawn tick cost is acceptable.

5. Documentation Reconciliation
   - `polit-ij82`
   - Purpose: align the GDD and architecture docs with what is actually implemented.

## Execution Order

1. `polit-4gvu`
   - Build the first real Rust consumer seam.
   - This turns the election payload from “available” into “used.”

2. `polit-a12w`
   - Establish a real population bootstrap source.
   - This is the cleanest foundation for household microsimulation.

3. `polit-0bvl` and `polit-93ic`
   - Run in parallel after the runtime seam is established.
   - `polit-0bvl` is independent of household bootstrap.
   - `polit-93ic` should target the post-bootstrap population profile path.

4. `polit-etef`
   - Run only after the consumer seam and real integrations land.
   - This is the system-level “is the stack actually real?” gate.

5. `polit-ij82`
   - Finalize after runtime behavior and validation are stable.

## Dependency Plan

- `polit-2pn` is the orchestration epic for:
  - `polit-4gvu`
  - `polit-a12w`
  - `polit-93ic`
  - `polit-0bvl`
  - `polit-etef`
  - `polit-ij82`
- `polit-a12w` blocks `polit-93ic`
- `polit-4gvu`, `polit-a12w`, `polit-93ic`, and `polit-0bvl` block `polit-etef`
- `polit-4gvu`, `polit-a12w`, `polit-93ic`, `polit-0bvl`, and `polit-etef` block `polit-ij82`

## Definition Of Done For The Epic

The simulation stack epic is not complete until all of the following are true:

- Rust consumes typed election inputs in a real runtime path.
- Population bootstrap uses real `rti_synth_pop` integration or the task is explicitly rescoped and closed with evidence.
- Household simulation uses real PolicyEngine US integration or the task is explicitly rescoped and closed with evidence.
- Macro simulation uses real PyFRB/US integration or the task is explicitly rescoped and closed with evidence.
- Dedicated integration and benchmark coverage prove 52-week bounded behavior and acceptable Dawn-phase performance.
- GDD sections 01, 05, 12, 15, 16, 17, 19, and 21 match the implemented architecture.

## Recommended Next Task

Start with `polit-4gvu`.

Reason:

- It exercises the Rust-side runtime boundary immediately.
- It reduces ambiguity in how the rest of the simulation outputs should be consumed.
- It is cheaper than external dependency integrations and gives faster feedback on architecture quality.
