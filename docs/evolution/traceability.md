# Evolution Engine — Proposal Traceability

## Proposal ID Format

```
EVO-YYYYMMDD-NNNN
```

- `EVO` — namespace prefix for all evolution proposals
- `YYYYMMDD` — date of proposal generation (UTC)
- `NNNN` — zero-padded sequential index within the day

Example: `EVO-20240610-0003` = third proposal generated on 10 June 2024.

## Proposal Lifecycle

| Status | Who Sets It | Required Fields | Next Status |
|---|---|---|---|
| `Draft` | ProposalGenerator | id, title, description, hypothesis, blastRadius, score | `Shadow` or `Rejected` |
| `Shadow` | EvolutionEngine | experimentId, mode=Shadow | `Canary` or `Rejected` |
| `Canary` | EvolutionEngine | experimentId, mode=Canary, beforeMetrics | `PendingApproval` or `RolledBack` |
| `PendingApproval` | Engine (auto) or Guardrails | confidence, pValue | `Approved` or `Rejected` |
| `Approved` | Reviewer or Engine (SingleFile) | approvedBy, approvedAt | `Applied` |
| `Applied` | PlatformAdapter | appliedAt, prUrl | terminal |
| `Rejected` | Engine or Reviewer | rejectionReason | terminal |
| `RolledBack` | ExperimentRunner | rollbackAt, rollbackReason | terminal |

## Hypothesis Tracking

Each proposal carries a `hypothesis` field: a falsifiable statement of the expected outcome.

Format: `"If [change], then [metric] will [direction] by [amount] within [timeframe]"`

The HypothesisStore links:
- **Proposal** to hypothesis string and targetMetric
- **Experiment** to beforeMetrics, afterMetrics, and result.deltaScore
- **Outcome** to result.recommendation and AuditEntry outcome validation

## Example: Full Proposal Lifecycle

```
EVO-20240610-0001  "Enable edge caching for /api/static"
  blastRadius: SingleRepo   score: 0.84   targetMetric: http.p95_latency_ms

  2024-06-10T06:02Z  Draft            ProposalGenerator scored from LatencyEvaluator opportunity
  2024-06-10T06:03Z  Shadow           EvolutionEngine ran exp-sh-001 (no traffic, metrics only)
  2024-06-10T18:00Z  Canary           EvolutionEngine ran exp-cn-002 (10% traffic)
  2024-06-11T06:00Z  PendingApproval  confidence=0.91, pValue=0.03 auto-gate passed
  2024-06-11T09:14Z  Approved         Reviewer @roberdan approved PR #412
  2024-06-11T09:15Z  Applied          ClaudeConfigAdapter merged PR, prUrl: github.com/.../412
```
