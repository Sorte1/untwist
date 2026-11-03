const SVG = 'http://www.w3.org/2000/svg';

const STROKE: readonly (readonly [string, string])[] = [
  ['viewBox', '0 0 16 16'],
  ['fill', 'none'],
  ['stroke', 'currentColor'],
  ['stroke-linecap', 'round'],
  ['stroke-linejoin', 'round'],
  ['aria-hidden', 'true'],
];

export function strokeIcon(path: string, size: number, width = 1.5): SVGSVGElement {
  const svg = document.createElementNS(SVG, 'svg');
  for (const [name, value] of [...STROKE, ['width', String(size)], ['height', String(size)], ['stroke-width', String(width)]]) {
    svg.setAttribute(name ?? '', value ?? '');
  }
  const shape = document.createElementNS(SVG, 'path');
  shape.setAttribute('d', path);
  svg.append(shape);
  return svg;
}

export const CLOSE_PATH = 'M4 4l8 8M12 4l-8 8';
export const CHEVRON_PATH = 'M4 6l4 4 4-4';

export function maskOf(path: string): string {
  const svg = `<svg xmlns='${SVG}' viewBox='0 0 16 16' fill='none' stroke='black' stroke-width='1.5' stroke-linecap='round'><path d='${path}'/></svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}
