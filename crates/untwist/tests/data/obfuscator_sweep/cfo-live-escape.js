function l(a) {
  if (a <= 1) {
    return 1;
  } else {
    return a * l(a - 1);
  }
}
function m(a, b) {
  const f = {
    q: function (i, j) {
      return i(j);
    },
    o: function (i, j) {
      return i !== j;
    },
    k: 396
  };
  return function (d) {
    if (f.o(d, 20)) {
      return f.q(b, a(d));
    } else {
      return f.q(String, f.k) + [f].length + d;
    }
  };
}
function n() {
  const a = ["sCkmBq", "W57dMZzE", "CKFdJSkSWPy", "W6hdGmkWymkudq", "krpcNa", "imk7W5PQ", "hfFcR8ot", "WORdLMu", "FSkala", "ev3cOSojWQBdPLK", "kmoKW4tdMG", "auFcUG", "DSoRW65Exa", "dvldOLxcQa", "WR1MW7G", "WRDMhCokbq"];
  n = function () {
    return a;
  };
  return n();
}
function o(a) {
  let b = a;
  return {
    next: function () {
      b += 1;
      return b;
    },
    reset: function () {
      if (s(398, "8Xtm") !== "oLSdd") {
        r = a;
        return s;
      } else {
        b = a;
        return b;
      }
    }
  };
}
const p = (a) => a * 2;
const q = (a) => a + 1;
function r(y, z) {
  return s(y - 540, z);
}
function s(b, d) {
  b = b - 396;
  const e = n();
  let f = e[b];
  if (s.g === undefined) {
    var y = function (G) {
      const H = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
      let I = "";
      let J = "";
      for (let K = 0, M, O, P = 0; O = G.charAt(P++); ~O && (M = K % 4 ? M * 64 + O : O, K++ % 4) ? I += String.fromCharCode(M >> (K * -2 & 6) & 255) : 0) {
        O = H.indexOf(O);
      }
      for (let Q = 0, R = I.length; Q < R; Q++) {
        J += "%" + ("00" + I.charCodeAt(Q).toString(16)).slice(-2);
      }
      return decodeURIComponent(J);
    };
    const C = function (G, H) {
      const I = [];
      let J = 0;
      let K;
      let M = "";
      G = y(G);
      let N;
      for (N = 0; N < 256; N++) {
        I[N] = N;
      }
      for (N = 0; N < 256; N++) {
        J = (J + I[N] + H.charCodeAt(N % H.length)) % 256;
        K = I[N];
        I[N] = I[J];
        I[J] = K;
      }
      N = 0;
      J = 0;
      for (let O = 0; O < G.length; O++) {
        N = (N + 1) % 256;
        J = (J + I[N]) % 256;
        K = I[N];
        I[N] = I[J];
        I[J] = K;
        M += String.fromCharCode(G.charCodeAt(O) ^ I[(I[N] + I[J]) % 256]);
      }
      return M;
    };
    s.h = C;
    s.i = {};
    s.g = true;
  }
  const z = e[0];
  if (s.j !== z) {
    s.i = {};
    s.j = z;
  }
  const A = s.i[b];
  if (A === undefined) {
    if (s.k === undefined) {
      s.k = true;
    }
    f = s.h(f, d);
    s.i[b] = f;
  } else {
    f = A;
  }
  return f;
}
const t = m(p, q);
function u(...a) {
  return a[s(399, "tGUI")]((d, e) => d + e, 0);
}
function v(a = "hi", ...b) {
  return b[s(400, "*K8E")]((e) => a + " " + e).join("; ");
}
const w = o(10);
w[r(941, "9rMg")]();
w[r(942, "Thtg")]();
console[r(943, "7[SN")]("factorial", l(7));
console.log("pipeline", t(20));
console[r(944, "0BiL")](r(945, "Thtg"), w[r(946, "&cNR")](), w.reset());
console.log(r(947, "Thtg"), u(4, 8, 15, 16, 23, 42));
console.log(r(948, "tfR)"), v(undefined, r(949, "si0]"), "south"));
console[r(950, "WQxH")](r(951, "4*4b"), Math.max.apply(null, [3, 9, 1]));
