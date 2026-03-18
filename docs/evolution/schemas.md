# Evolution Engine — Interface Schemas

All types are defined in `evolution/core/types/index.ts`. Tables use compact format.

## Metric

| Field | Type | Description | Example |
|---|---|---|---|
| `name` | `string` | Dot-namespaced metric name | `http.p95_latency_ms` |
| `value` | `number` | Raw numeric reading | `342.7` |
| `timestamp` | `number` | Unix epoch milliseconds | `1718000000000` |
| `labels` | `Record<string,string>` | Dimension key-value pairs | `{ service: "api", env: "prod" }` |
| `family` | `MetricFamily` | High-level grouping | `Runtime` |

`MetricFamily` values: `Agent` \| `Runtime` \| `Workload` \| `Mesh` \| `Database` \| `Build` \| `Bundle`

## EvaluationResult

| Field | Type | Description | Example |
|---|---|---|---|
| `domain` | `string` | Evaluator domain name | `latency` |
| `anomalies` | `Anomaly[]` | Detected anomalies | `[{ metric, severity, detail }]` |
| `opportunities` | `OptimizationOpportunity[]` | Suggested improvements | see below |
| `timestamp` | `number` | Evaluation time (ms epoch) | `1718000000000` |

**Anomaly sub-fields**: `metric: string`, `severity: 'low'|'medium'|'high'`, `detail: string`

**OptimizationOpportunity sub-fields**: `title`, `description`, `estimatedGain`, `domain`, `suggestedBlastRadius`

## Proposal

| Field | Type | Description | Example |
|---|---|---|---|
| `id` | `string` | EVO-YYYYMMDD-NNNN format | `EVO-20240610-0001` |
| `title` | `string` | Human-readable title | `Enable edge caching` |
| `description` | `string` | Detailed rationale | `Reduce p95 by 20%…` |
| `hypothesis` | `string` | Testable prediction | `If we cache /api/static…` |
| `targetMetric` | `string` | Primary success metric | `http.p95_latency_ms` |
| `estimatedGain` | `string` | Expected improvement | `-20% p95 latency` |
| `blastRadius` | `BlastRadius` | Change scope | `SingleRepo` |
| `status` | `ProposalStatus` | Lifecycle state | `Canary` |
| `score` | `number` | Priority score [0–1] | `0.82` |
| `source` | `SourceType` | Origin of proposal | `Internal` |
| `targetAdapter` | `string` | Adapter to apply via | `claude-config` |
| `createdAt` | `number` | Creation epoch ms | `1718000000000` |

`BlastRadius`: `SingleFile` \| `SingleRepo` \| `MultiRepo` \| `Ecosystem`

`ProposalStatus`: `Draft` \| `Shadow` \| `Canary` \| `PendingApproval` \| `Approved` \| `Applied` \| `Rejected` \| `RolledBack`

## Experiment

| Field | Type | Description | Example |
|---|---|---|---|
| `id` | `string` | UUID | `exp-abc123` |
| `proposalId` | `string` | Linked proposal | `EVO-20240610-0001` |
| `mode` | `ExperimentMode` | Deployment mode | `Canary` |
| `startedAt` | `number` | Start epoch ms | `1718000000000` |
| `completedAt` | `number?` | Completion epoch ms | `1718086400000` |
| `beforeMetrics` | `Metric[]` | Baseline snapshot | `[...]` |
| `afterMetrics` | `Metric[]` | Post-change snapshot | `[...]` |
| `result` | `ExperimentResult?` | Statistical outcome | see below |

**ExperimentResult sub-fields**: `confidence: number`, `pValue: number`, `recommendation: 'apply'|'rollback'|'extend'`, `deltaScore: number`

`ExperimentMode`: `Shadow` \| `Canary` \| `Full`

## AuditEntry

| Field | Type | Description | Example |
|---|---|---|---|
| `id` | `string` | UUID | `audit-xyz789` |
| `timestamp` | `number` | Event epoch ms | `1718000000000` |
| `actor` | `string` | Identity (`engine`, `human:<login>`, `ci`) | `human:roberdan` |
| `action` | `string` | What happened | `proposal.approved` |
| `proposalId` | `string?` | Linked proposal if applicable | `EVO-20240610-0001` |
| `detail` | `string?` | Free-form detail | `Manual approval via dashboard` |

## RoiSummary

| Field | Type | Description | Example |
|---|---|---|---|
| `period` | `string` | ISO week label | `2024-W24` |
| `proposalsGenerated` | `number` | Proposals created this period | `7` |
| `experimentsRun` | `number` | Experiments completed | `4` |
| `rollbacks` | `number` | Experiments rolled back | `1` |
| `netDeltaScore` | `number` | Sum of deltaScore across experiments | `2.34` |
| `estimatedSavingsUsd` | `number` | Placeholder: successful × $0.10 | `0.30` |
