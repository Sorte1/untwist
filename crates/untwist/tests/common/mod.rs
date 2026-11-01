#![allow(dead_code)]

pub const WEBPACK_4: &str = "!(function (e) {\n  var a = {};\n  function n(t) {\n    if (a[t]) return a[t].exports;\n    var r = (a[t] = { exports: {} });\n    return e[t].call(r.exports, r, r.exports, n), r.exports;\n  }\n  return n((n.s = 0));\n})([\n  function (e, t, i) {\n    const lib = i(1);\n    console.log(lib);\n  },\n  function (e, t, i) {\n    e.exports = 1;\n  },\n]);\n";

pub const CHUNK_A: &str = "(window.webpackJsonp = window.webpackJsonp || []).push([\n  [15],\n  {\n    '2wwy': function (e, t, i) {\n      const a = i('E8gZ');\n      const b = i('w6GO');\n    },\n    E8gZ: function (e, t, i) {\n      e.exports = 4;\n    },\n  },\n]);\n";
pub const CHUNK_B: &str = "(window.webpackJsonp = window.webpackJsonp || []).push([\n  [16],\n  {\n    w6GO: function (e, t, i) {\n      e.exports = 5;\n    },\n  },\n]);\n";
