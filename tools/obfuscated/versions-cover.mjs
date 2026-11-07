import { CONTAINERS, FACTORS, NA, domain } from "./versions-plan.mjs";

const CANDIDATES = 12;
const STRONG = [
  ["version", "encoding", "container", "minifier"],
  ["version", "program", "container"],
  ["version", "encoding", "rotate", "wrappers"],
];

export function random(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const key = (assignments) => assignments.map(([factor, value]) => `${factor}=${value}`).join("|");

function feasible(space, row) {
  const { version, program, container, extra } = row;
  if (version === undefined) return true;
  if (program !== undefined && extra !== undefined && space.excluded.has(`${version}/${program}/${extra}`)) return false;
  const programs = program === undefined ? space.programs : [program];
  const containers = container === undefined ? CONTAINERS : [container];
  return programs.some((p) => containers.some((c) => space.allowed(version, p, c)));
}

function tuplesOf(row) {
  const out = STRONG.map((factors) => key(factors.map((factor) => [factor, row[factor]])));
  const all = ["version", ...FACTORS];
  for (let i = 0; i < all.length; i++) {
    for (let j = i + 1; j < all.length; j++) {
      const [a, b] = [all[i], all[j]];
      if (row[a] === undefined || row[b] === undefined || row[a] === NA || row[b] === NA) continue;
      out.push(key([[a, row[a]], [b, row[b]]]));
    }
  }
  return out;
}

function combinations(space, version, factors) {
  let rows = [{ version }];
  for (const factor of factors.slice(1)) {
    rows = rows.flatMap((row) => domain(version, factor, space.programs).map((value) => ({ ...row, [factor]: value })));
  }
  return rows.filter((row) => feasible(space, row));
}

function required(space) {
  const tuples = new Set();
  for (const version of space.versions) {
    for (const factors of STRONG) {
      for (const row of combinations(space, version, factors)) tuples.add(key(factors.map((factor) => [factor, row[factor]])));
    }
    const all = ["version", ...FACTORS];
    const values = (factor) => (factor === "version" ? [version] : domain(version, factor, space.programs).filter((v) => v !== NA));
    for (let i = 0; i < all.length; i++) {
      for (let j = i + 1; j < all.length; j++) {
        for (const x of values(all[i])) {
          for (const y of values(all[j])) {
            const row = { version, [all[i]]: x, [all[j]]: y };
            if (feasible(space, row)) tuples.add(key([[all[i], x], [all[j], y]]));
          }
        }
      }
    }
  }
  return tuples;
}

function gain(uncovered, row) {
  return tuplesOf(row).filter((tuple) => uncovered.has(tuple)).length;
}

function parse(tuple) {
  return Object.fromEntries(tuple.split("|").map((part) => part.split("=")));
}

function admits(space, version, fixed) {
  return Object.entries(fixed).every(([factor, value]) => factor === "version" || domain(version, factor, space.programs).includes(value)) && feasible(space, { ...fixed, version });
}

function shuffled(items, next) {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(next() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

function candidate(space, uncovered, fixed, next) {
  const versions = fixed.version ? [fixed.version] : space.versions.filter((v) => admits(space, v, fixed));
  const row = { ...fixed, version: versions[Math.floor(next() * versions.length)] };
  for (const factor of shuffled(FACTORS.filter((f) => row[f] === undefined), next)) {
    const options = domain(row.version, factor, space.programs).filter((value) => feasible(space, { ...row, [factor]: value }));
    let best = [];
    let score = -1;
    for (const value of shuffled(options, next)) {
      const s = gain(uncovered, { ...row, [factor]: value });
      if (s > score) [best, score] = [[value], s];
    }
    row[factor] = best[0];
  }
  return row;
}

export function cover(space, seed) {
  const next = random(seed);
  const uncovered = required(space);
  const rows = [];
  while (uncovered.size > 0) {
    const fixed = parse(uncovered.values().next().value);
    let best = null;
    let score = -1;
    for (let i = 0; i < CANDIDATES; i++) {
      const row = candidate(space, uncovered, fixed, next);
      const s = gain(uncovered, row);
      if (s > score) [best, score] = [row, s];
    }
    if (score <= 0) throw new Error(`no row covers ${key(Object.entries(fixed))}`);
    for (const tuple of tuplesOf(best)) uncovered.delete(tuple);
    rows.push(best);
  }
  return rows;
}
