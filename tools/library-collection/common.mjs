export function parseArgs(argv, defaults = {}) {
  const options = { ...defaults, _: [] };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) options[argv[i].slice(2)] = argv[++i];
    else options._.push(argv[i]);
  }
  return options;
}

const parts = (version) => version.split(/[.+-]/).map((part) => (/^\d+$/.test(part) ? Number(part) : 0));

export function compareVersions(a, b) {
  const left = parts(a);
  const right = parts(b);
  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const difference = (left[i] ?? -1) - (right[i] ?? -1);
    if (difference) return difference;
  }
  return a.localeCompare(b);
}

export const ascending = (versions) => [...new Set(versions)].sort(compareVersions);

export const isRelease = (version) => /^\d+\.\d+\.\d+$/.test(version);

export function owner(path) {
  const normal = path.replace(/\\/g, "/").replace(/^\0/, "").replace(/\?.*$/, "");
  const at = normal.lastIndexOf("node_modules/");
  if (at < 0) return null;
  const rest = normal.slice(at + "node_modules/".length).split("/");
  const name = rest[0].startsWith("@") ? `${rest[0]}/${rest[1]}` : rest[0];
  const file = rest.slice(rest[0].startsWith("@") ? 2 : 1).join("/");
  if (!name || !file) return null;
  return { name, file, key: `node_modules/${name}/${file}` };
}
