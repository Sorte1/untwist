import './refusals.css';
import { explainRefusals, type Explanation } from './explain';

export interface RefusalView {
  readonly report: string;
  readonly text: string;
  readonly linked: boolean;
  readonly reveal: (span: readonly [number, number]) => void;
}

function make(tag: string, className: string, text = ''): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function row(explanation: Explanation, view: RefusalView): HTMLElement {
  const { span } = explanation;
  const linked = view.linked && span !== undefined;
  const element = make(linked ? 'button' : 'div', 'why-row');
  element.append(make('span', 'why-loc', explanation.place), make('span', 'why-text', explanation.sentence));
  if (explanation.code !== '') {
    element.append(make('code', 'why-code', explanation.code));
  }
  if (element instanceof HTMLButtonElement && span !== undefined) {
    element.type = 'button';
    element.title = 'Show in the input';
    element.addEventListener('click', () => view.reveal(span));
  }
  const item = document.createElement('li');
  item.append(element);
  return item;
}

function placesText(places: number): string {
  return `${places.toLocaleString('en-US')} ${places === 1 ? 'place' : 'places'}`;
}

export function renderRefusals(holder: HTMLElement, view: RefusalView): void {
  const { shown, more, reasons } = explainRefusals(view.report, view.text);
  const title = make('p', 'why-title', 'Why Safe mode left the decoder calls in place');
  if (shown.length === 0) {
    holder.replaceChildren(title, make('p', 'why-note', 'This run did not record which code kept the decoder.'));
    return;
  }
  const list = make('ol', 'why-list');
  list.append(...shown.map((explanation) => row(explanation, view)));
  const counted = more > 0 ? reasons.map((reason) => make('p', 'why-note why-count', `${placesText(reason.places)} ${reason.sentence}`)) : [];
  const notes = [
    ...counted,
    ...(more > 0 ? [make('p', 'why-note', `${more.toLocaleString('en-US')} more not shown.`)] : []),
    ...(view.linked ? [] : [make('p', 'why-note', 'The input changed after this run, so the lines are not linked.')]),
  ];
  holder.replaceChildren(title, list, ...notes);
}
