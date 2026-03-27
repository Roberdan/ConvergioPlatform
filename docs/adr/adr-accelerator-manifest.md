# ADR: Accelerator Manifest Schema

**Status**: Accepted | **Date**: 27 Marzo 2026

## Context

Accelerators are pre-configured domain packages (banking, healthcare, retail) that bundle agents, workflows, eval scenarios, and data schemas into deployable units. This ADR establishes the `accelerator.yaml` vocabulary for Plan Q (2027).

## Decision

```yaml
# accelerator.yaml schema
name: string                    # e.g., "healthcare-claims"
domain: string                  # banking | healthcare | retail | ...
version: string                 # semver
agents:                         # references to artifact registry
  - name: string
    artifact_id: number
    role: string                # coordinator | executor | validator
workflows:                      # orchestration rules
  - name: string
    trigger: string             # event | schedule | manual
    steps: [{agent, action}]
eval_scenarios:                 # test cases for validation
  - name: string
    input: object
    expected_output: object
    timeout_s: number
data_schemas:                   # for synthetic data generation
  - name: string
    format: json_schema | avro | protobuf
    schema: object
deploy_target: local | azure | aws
```

## Consequences

Future Plan Q implements the runtime. This ADR only establishes vocabulary.
