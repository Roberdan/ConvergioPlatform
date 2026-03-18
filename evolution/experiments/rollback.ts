export interface RollbackCapable {
  rollback(experimentId: string): Promise<void>;
}

export interface RollbackInfo {
  rollback: boolean;
  revertPR: {
    prUrl: string;
    title: string;
  };
}

export class RollbackManager {
  async rollback(experimentId: string, adapter: RollbackCapable): Promise<RollbackInfo> {
    await adapter.rollback(experimentId);
    return {
      rollback: true,
      revertPR: {
        prUrl: `https://github.com/convergio/rollback/${experimentId}`,
        title: `revertPR: rollback ${experimentId}`,
      },
    };
  }
}
