# Execute — Validation, Merge & Completion

## Per-Wave Validation (Thor)

After ALL tasks in a wave are `submitted`:

1. Invoke Thor: `bash claude-config/scripts/validate-wave.sh {wave_db_id} {plan_id}`
2. Thor reads `test_criteria.verify[]` for EACH task and runs them
3. Thor reads `validator_agent` — uses domain-specific gates:
   - `output_type: pr` → Thor 10 code gates
   - `output_type: document` → doc-validator (completeness, structure, sources)
   - `output_type: analysis` → strategy-validator (data quality, feasibility)
   - `output_type: design` → design-validator (a11y, consistency)
   - `output_type: legal_opinion` → compliance-validator (regulations, gaps)
4. If ALL pass → tasks promoted to `done`, wave marked `done`
5. If ANY fail → tasks back to `in_progress`, fix and resubmit

## Wave Merge

After Thor passes:
1. `bash claude-config/scripts/wave-worktree.sh merge {plan_id} {wave_db_id}`
2. Squash merge wave branch into main
3. Cleanup: remove worktree, delete branch

For parallel waves (W2a, W2b, W2c):
- Each needs its own worktree/branch
- Merge sequentially after all pass Thor
- Rebase each on main before merge to avoid conflicts

## Next Wave

After merge:
1. Create worktree for next wave: `bash claude-config/scripts/wave-worktree.sh create {plan_id} {next_wave_id}`
2. Continue with pending tasks in next wave

## Plan Completion

After ALL waves done:
1. `bash claude-config/scripts/plan-db.sh complete {plan_id}`
2. Verify: `git worktree list` shows only main
3. Verify: all PRs merged
4. Post-mortem: `Agent(subagent_type="plan-post-mortem")`
5. Calibrate: `bash claude-config/scripts/plan-db.sh calibrate-estimates`
