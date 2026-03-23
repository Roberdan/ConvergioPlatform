export async function initExperimentView(container) {
  const host = container;
  host.innerHTML = `
    <section class="mn-card mn-surface">
      <h2 class="mn-title">Active Experiments</h2>
      <div class="mn-data-table" role="table" aria-label="Experiments table">
        <div class="mn-row mn-header" role="row">
          <span role="columnheader">Experiment</span><span role="columnheader">Mode</span>
          <span role="columnheader">Progress</span><span role="columnheader">Delta</span>
        </div>
        <div class="mn-row" role="row"><span>EXP-001</span><span>Canary</span><span><progress value="40" max="100">40%</progress></span><span>+3.1%</span></div>
      </div>
      <div class="mn-chart" aria-label="Experiment trend chart"></div>
    </section>`;
}
