export const LIBS = {
  polyfill: {
    "core-js": ["2.5.7", "2.6.12"],
    "es5-shim": ["4.6.7"],
    "es6-shim": ["0.35.8"],
    "whatwg-fetch": ["2.0.4", "3.6.20"],
    "promise-polyfill": ["6.1.0", "7.1.2", "8.3.0"],
    "es6-promise": ["3.3.1", "4.2.8"],
    "babel-polyfill": ["6.26.0"],
    "@babel/polyfill": ["7.12.1"],
    "regenerator-runtime": ["0.10.5", "0.11.1", "0.13.3", "0.13.11", "0.14.1"],
  },
  node: {
    "node-libs-browser": ["2.2.1"],
    buffer: ["4.9.2", "5.7.1", "6.0.3"],
    process: ["0.11.10"],
    events: ["1.1.1", "3.3.0"],
    util: ["0.10.4", "0.11.1", "0.12.5"],
    "path-browserify": ["0.0.1", "1.0.1"],
    "stream-browserify": ["2.0.2", "3.0.0"],
    "readable-stream": ["2.3.8", "3.6.2"],
    "crypto-browserify": ["3.12.1"],
    "bn.js": ["4.12.5", "5.2.5"],
    elliptic: ["6.6.1"],
    inherits: ["2.0.3", "2.0.4"],
    "safe-buffer": ["5.1.2", "5.2.1"],
    "base64-js": ["1.3.1", "1.5.1"],
    ieee754: ["1.1.13", "1.2.1"],
  },
  framework: {
    react: ["16.14.0", "17.0.2", "18.3.1", "19.3.1"],
    "react-dom": ["16.14.0", "17.0.2", "18.3.1", "19.3.1"],
    preact: ["8.5.3", "10.5.15", "10.29.8"],
    vue: ["2.6.14", "2.7.16", "3.2.47", "3.5.45"],
    svelte: ["3.59.2", "4.2.20", "5.57.3"],
    "solid-js": ["1.6.16", "1.9.19"],
    "zone.js": ["0.11.8", "0.14.10", "0.16.3"],
    jquery: ["1.12.4", "2.2.4", "3.5.1", "3.7.1", "4.0.0"],
  },
  utility: {
    lodash: ["3.10.1", "4.17.21", "4.18.1"],
    "lodash-es": ["4.17.21", "4.18.1"],
    underscore: ["1.9.1", "1.13.8"],
    ramda: ["0.27.2", "0.32.0"],
    axios: ["0.21.4", "0.27.2", "1.21.1"],
    moment: ["2.29.4", "2.31.0"],
    dayjs: ["1.11.23"],
    "date-fns": ["2.30.0", "4.4.0"],
    uuid: ["8.3.2", "9.0.1", "14.0.3"],
    immer: ["9.0.21", "10.2.0"],
    rxjs: ["6.6.7", "7.8.2"],
    mobx: ["5.15.7", "6.16.1"],
    redux: ["4.2.1", "5.0.1"],
    "@reduxjs/toolkit": ["1.9.7", "2.14.0"],
    zustand: ["4.5.7", "5.1.0"],
    eventemitter3: ["4.0.7", "5.0.4"],
    "msgpack-lite": ["0.1.26"],
    "@msgpack/msgpack": ["2.8.0", "3.1.3"],
    protobufjs: ["6.11.6", "7.6.6"],
    pako: ["1.0.11", "2.2.0"],
    fflate: ["0.7.4", "0.8.3"],
    "js-cookie": ["2.2.1", "3.0.8"],
    qs: ["6.5.3", "6.17.1"],
  },
  game: {
    three: ["0.105.2", "0.118.3", "0.125.2", "0.128.0", "0.137.5", "0.147.0", "0.150.1", "0.160.1", "0.170.0", "0.187.0"],
    cannon: ["0.6.2"],
    "cannon-es": ["0.20.0"],
    howler: ["2.1.3", "2.2.4"],
    "pixi.js": ["4.8.9", "5.3.12", "8.23.0"],
    "socket.io-client": ["2.5.0", "3.1.3", "4.8.4"],
    "engine.io-client": ["3.5.6", "6.6.8"],
    "gl-matrix": ["2.8.1", "3.4.4"],
    "stats.js": ["0.17.0"],
    "@tweenjs/tween.js": ["18.6.4", "25.0.0"],
    "tween.js": ["16.6.0"],
  },
};

export const ISOLATED = {
  game: { "pixi.js": ["6.5.10", "7.4.3"] },
};

export const TOOLS = {
  webpack: "5.112.2",
  rollup: "4.66.0",
  "@rollup/plugin-commonjs": "29.0.3",
  "@rollup/plugin-node-resolve": "16.0.3",
  "@rollup/plugin-alias": "6.0.1",
  "@rollup/plugin-replace": "6.0.3",
  "@rollup/plugin-json": "6.1.0",
  esbuild: "0.28.2",
  vite: "8.3.5",
  terser: "5.51.2",
  "javascript-obfuscator": "4.2.2",
};

export const alias = (name, version) => `${name.replace(/^@/, "").replace("/", "-")}--${version}`;

export function entries() {
  const out = [];
  for (const [group, libs] of Object.entries(LIBS)) {
    for (const [name, versions] of Object.entries(libs)) {
      for (const version of versions) out.push({ group, name, version, slug: alias(name, version), alias: alias(name, version), root: "." });
    }
  }
  for (const [group, libs] of Object.entries(ISOLATED)) {
    for (const [name, versions] of Object.entries(libs)) {
      for (const version of versions) out.push({ group, name, version, slug: alias(name, version), alias: name, root: `isolated/${alias(name, version)}` });
    }
  }
  return out;
}
