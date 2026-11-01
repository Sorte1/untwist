export interface VirtualList<T> {
  readonly element: HTMLElement;
  setRows(rows: readonly T[]): void;
}

const OVERSCAN = 6;

export function virtualList<T>(rowHeight: number, maxRows: number, draw: (row: T) => HTMLElement): VirtualList<T> {
  const element = document.createElement('div');
  element.className = 'vlist';
  const spacer = document.createElement('div');
  const window_ = document.createElement('div');
  window_.className = 'vlist-window';
  spacer.append(window_);
  element.append(spacer);
  let rows: readonly T[] = [];
  const paint = () => {
    const first = Math.max(0, Math.floor(element.scrollTop / rowHeight) - OVERSCAN);
    const last = Math.min(rows.length, first + maxRows + OVERSCAN * 2);
    window_.style.transform = `translateY(${first * rowHeight}px)`;
    window_.replaceChildren(...rows.slice(first, last).map(draw));
  };
  element.addEventListener('scroll', paint, { passive: true });
  return {
    element,
    setRows(next) {
      rows = next;
      element.style.maxHeight = `${maxRows * rowHeight}px`;
      spacer.style.height = `${rows.length * rowHeight}px`;
      paint();
    },
  };
}
