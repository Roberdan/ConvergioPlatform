export async function initProposalsView(container) {
  const host = container;
  host.innerHTML = `
    <section class="mn-card mn-surface">
      <h2 class="mn-title">Proposals Queue</h2>
      <div class="mn-data-table" role="table" aria-label="Proposals table">
        <div class="mn-row mn-header" role="row">
          <span role="columnheader">ID</span><span role="columnheader">Hypothesis</span>
          <span role="columnheader">Domain</span><span role="columnheader">Blast Radius</span>
          <span role="columnheader">Status</span><span role="columnheader">Score</span>
        </div>
        <div class="mn-row" role="row"><span>EVO-20250601-0001</span><span>Tree-shake lodash imports</span><span>bundle</span><span>SingleRepo</span><span class="mn-badge">Approved</span><span>0.87</span></div>
      </div>
    </section>`;
}
