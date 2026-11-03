import './update-prompt.css';

const CLOSE_ICON =
  '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8"></path></svg>';

function button(id: string, className: string, label: string): HTMLButtonElement {
  const element = document.createElement('button');
  element.type = 'button';
  element.id = id;
  element.className = className;
  element.textContent = label;
  return element;
}

export function showUpdatePrompt(worker: ServiceWorker): void {
  if (document.getElementById('update-prompt') !== null) {
    return;
  }
  const prompt = document.createElement('div');
  prompt.id = 'update-prompt';
  prompt.className = 'update-prompt';
  prompt.setAttribute('role', 'status');
  const text = document.createElement('span');
  text.className = 'update-text';
  text.textContent = 'A new version of Untwist is ready.';
  const reload = button('update-reload', 'btn btn-small update-reload', 'Reload');
  const dismiss = button('update-dismiss', 'btn btn-square', '');
  dismiss.innerHTML = CLOSE_ICON;
  dismiss.setAttribute('aria-label', 'Later');
  dismiss.title = 'Later';
  reload.addEventListener('click', () => {
    reload.disabled = true;
    reload.textContent = 'Reloading…';
    navigator.serviceWorker.addEventListener('controllerchange', () => window.location.reload(), { once: true });
    worker.postMessage({ kind: 'skip-waiting' });
  });
  dismiss.addEventListener('click', () => prompt.remove());
  prompt.append(text, reload, dismiss);
  document.body.append(prompt);
}
