export async function initAgentsView(container) {
  const host = container;
  host.innerHTML = `
    <section class="mn-card mn-surface">
      <h2 class="mn-title">Agents Health</h2>
      <div class="mn-data-table" role="table" aria-label="Agents table">
        <div class="mn-row mn-header" role="row">
          <span role="columnheader">Agent</span><span role="columnheader">Efficiency</span><span role="columnheader">Cost</span>
        </div>
        <div class="mn-row" role="row"><span>NaSra</span><span>0.91</span><span>$0.82</span></div>
      </div>
      <div class="agentCostBreakdown mn-card">agentCostBreakdown</div>
      <div class="tokenMeter mn-card">tokenMeter</div>
    </section>`;
}
