/**
 * KillSwitch — emergency halt for the Evolution Engine.
 *
 * When enabled, the engine must not start new cycles or apply proposals.
 * State is stored in-memory; persistence is the caller's responsibility.
 */

export interface KillSwitchState {
  enabled: boolean;
  reason: string;
  enabledAt: number | null;
  enabledBy: string | null;
}

export class KillSwitch {
  private state: KillSwitchState = {
    enabled: false,
    reason: '',
    enabledAt: null,
    enabledBy: null,
  };

  /** Returns true if the kill switch is active — engine must not proceed. */
  isEnabled(): boolean {
    return this.state.enabled;
  }

  /**
   * Activates the kill switch.
   * @param reason Human-readable justification logged in AuditTrail.
   * @param actor  Identity of the operator (e.g. `human:roberdan`).
   */
  enable(reason: string, actor: string): void {
    this.state = {
      enabled: true,
      reason,
      enabledAt: Date.now(),
      enabledBy: actor,
    };
  }

  /**
   * Deactivates the kill switch.
   * Caller must record an AuditEntry explaining the resumption.
   */
  disable(): void {
    this.state = {
      enabled: false,
      reason: '',
      enabledAt: null,
      enabledBy: null,
    };
  }

  /** Snapshot of current state for audit and dashboard display. */
  getState(): Readonly<KillSwitchState> {
    return { ...this.state };
  }
}
