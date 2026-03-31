export default {
  test: {
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      'dashboard/tests/e2e/**',
      'dashboard_web/tests/e2e/**',
      'evolution/tests/e2e/**',
      'src/ts/evolution/tests/e2e/**',
      'scripts/platform/visual-qa/tests/e2e/**',
      'integrations/openclaw-bridge/src/*.test.ts',
    ],
  },
};
