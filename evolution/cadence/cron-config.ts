export interface ParsedCron {
  cronExpression: string;
  minute: string;
  hour: string;
  dayOfMonth: string;
  month: string;
  dayOfWeek: string;
}

export class CronConfig {
  parse(cronExpression: string): ParsedCron {
    const parts = cronExpression.trim().split(/\s+/);
    if (parts.length !== 5) {
      throw new Error(`Invalid cron expression: ${cronExpression}`);
    }
    const [minute, hour, dayOfMonth, month, dayOfWeek] = parts;
    return { cronExpression, minute, hour, dayOfMonth, month, dayOfWeek };
  }

  matches(parsed: ParsedCron, date: Date): boolean {
    return (
      this.matchField(parsed.minute, date.getUTCMinutes()) &&
      this.matchField(parsed.hour, date.getUTCHours()) &&
      this.matchField(parsed.dayOfMonth, date.getUTCDate()) &&
      this.matchField(parsed.month, date.getUTCMonth() + 1) &&
      this.matchField(parsed.dayOfWeek, date.getUTCDay())
    );
  }

  private matchField(token: string, value: number): boolean {
    if (token === '*') return true;
    if (token.startsWith('*/')) {
      const step = Number(token.slice(2));
      return step > 0 && value % step === 0;
    }
    return Number(token) === value;
  }
}
