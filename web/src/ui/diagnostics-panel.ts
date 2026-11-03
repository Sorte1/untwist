import { byId } from '../dom';

export function mountDiagnosticsPanel(): void {
  const toggle = byId<HTMLButtonElement>('diag-toggle');
  const panel = byId('diagnostics');
  toggle.addEventListener('click', () => {
    panel.hidden = !panel.hidden;
    toggle.setAttribute('aria-expanded', String(!panel.hidden));
  });
}
