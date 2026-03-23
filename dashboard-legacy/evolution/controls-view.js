export function initControlsView(container) {
  const host = container;
  let killSwitch = false;
  let experimentPause = false;
  const rateLimitStatus = 'normal';

  const render = () => {
    host.innerHTML = `
      <section class="mn-card mn-surface">
        <h2>Evolution Controls</h2>
        <div class="mn-system-status" data-component="mn-system-status" aria-live="polite">
          System: ${killSwitch ? 'Disabled (kill-switch)' : 'Enabled'}
        </div>
        <p class="mn-body">Rate: <strong>${rateLimitStatus}</strong></p>
        <button class="mn-btn mn-btn-danger" id="kill-switch-btn">STOP ALL</button>
        <button class="mn-btn" id="pause-exp-btn">${experimentPause ? 'Resume' : 'Pause'} Experiment</button>
      </section>`;

    const killBtn = host.querySelector('#kill-switch-btn');
    const pauseBtn = host.querySelector('#pause-exp-btn');

    killBtn?.addEventListener('click', () => {
      killSwitch = !killSwitch;
      console.log('killSwitch toggled', killSwitch);
      render();
    });

    pauseBtn?.addEventListener('click', () => {
      experimentPause = !experimentPause;
      console.log('experimentPause toggled', experimentPause);
      render();
    });
  };

  render();
}
