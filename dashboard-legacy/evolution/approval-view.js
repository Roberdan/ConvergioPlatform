const state = {
  pending: [],
  approvalChain: [],
  decisionMatrix: {},
};

function blastRadiusBadge(radius) {
  return `<span class="mn-badge mn-badge-approval">${radius}</span>`;
}

function render() {
  const root = document.getElementById('approval-view');
  if (!root) return;

  root.innerHTML = state.pending
    .map(
      (proposal) => `
      <article class="mn-card mn-card-approval" data-proposal-id="${proposal.id}">
        <h3>${proposal.title}</h3>
        <p>Confidence: ${proposal.confidence}</p>
        ${blastRadiusBadge(proposal.blastRadius)}
        <div class="actions">
          <button class="approve" data-id="${proposal.id}">Approve</button>
          <button class="reject" data-id="${proposal.id}">Reject</button>
        </div>
      </article>`,
    )
    .join('');
}

async function loadPending() {
  const response = await fetch('/data/pending-approvals.json');
  state.pending = await response.json();
  render();
}

async function postDecision(proposalId, decision) {
  console.log('stub /api/approvals', { proposalId, decision });
  state.approvalChain.push({ proposalId, decision, ts: Date.now() });
  state.decisionMatrix[proposalId] = decision;
  state.pending = state.pending.filter((proposal) => proposal.id !== proposalId);
  render();
}

document.addEventListener('click', (event) => {
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;

  if (target.classList.contains('approve')) {
    void postDecision(target.dataset.id ?? '', 'approved');
  }
  if (target.classList.contains('reject')) {
    void postDecision(target.dataset.id ?? '', 'rejected');
  }
});

void loadPending();
