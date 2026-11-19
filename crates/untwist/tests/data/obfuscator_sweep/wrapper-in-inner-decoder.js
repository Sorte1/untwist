function obfQQ(Q, V) {
  Q = Q - 436;
  const M = obfQT();
  let b = M[Q];
  if (obfQQ[X(1234, "ai3P")] === undefined) {
    var c = function (R) {
      const t = p(1365, "eXII");
      function p(Y, F) {
        return X(Y - 130, F);
      }
      let y = "";
      let v = "";
      for (let z = 0, C, d, m = 0; d = R[p(1366, "bDaY")](m++); ~d && (C = z % 4 ? C * 64 + d : d, z++ % 4) ? y += String.fromCharCode(C >> (z * -2 & 6) & 255) : 0) {
        d = t.indexOf(d);
      }
      for (let U = 0, f = y[p(1369, "XyZk")]; U < f; U++) {
        if (p(1370, "nNCM") !== "exZuu") {
          v += "%" + ("00" + y[p(1371, "qh)q")](U).toString(16)).slice(-2);
        } else {
          b = c.indexOf(T);
        }
      }
      return decodeURIComponent(v);
    };
    var P = function (x, z) {
      {
        function J(I, s) {
          return X(I - -1523, s);
        }
        var A;
        const C = [];
        let d = 0;
        var m;
        let U = "";
        x = c(x);
        var f;
        for (f = 0; f < 256; f++) {
          C[f] = f;
        }
        for (f = 0; f < 256; f++) {
          if (J(-281, "0oSi") !== "jZnuJ") {
            d = (d + C[f] + z[J(-280, "F6Ce")](f % z[J(-279, "]%Ok")])) % 256;
            m = C[f];
            C[f] = C[d];
            C[d] = m;
          } else {
            var A = [];
            let Y = 0;
            var F;
            let i = "";
            x = z(C);
            var B;
            for (B = 0; B < 256; B++) {
              A[B] = B;
            }
            for (B = 0; B < 256; B++) {
              Y = (Y + A[B] + h[J(-278, "ai3P")](B % x[J(-277, "vF)l")])) % 256;
              F = A[B];
              A[B] = A[Y];
              A[Y] = F;
            }
            B = 0;
            Y = 0;
            for (let j = 0; j < U[J(-276, "9Vv*")]; j++) {
              B = (B + 1) % 256;
              Y = (Y + A[B]) % 256;
              F = A[B];
              A[B] = A[Y];
              A[Y] = F;
              i += z[J(-275, "%0Xs")](C[J(-274, "nNCM")](j) ^ A[(A[B] + A[Y]) % 256]);
            }
            return i;
          }
        }
        f = 0;
        d = 0;
        for (let g = 0; g < x[J(-273, "QIy]")]; g++) {
          if (J(-272, "Iein") === "cFsFG") {
            f = (f + 1) % 256;
            d = (d + C[f]) % 256;
            m = C[f];
            C[f] = C[d];
            C[d] = m;
            U += String.fromCharCode(x.charCodeAt(g) ^ C[(C[f] + C[d]) % 256]);
          } else {
            T[h(-309, "abL9")](P(x[z]));
          }
        }
        return U;
      }
    };
    obfQQ.BDrMOk = P;
    obfQQ.JKMQUL = {};
    obfQQ[X(1255, "&69e")] = true;
  }
  var T = M[0];
  if (obfQQ.usvncj !== T) {
    obfQQ.JKMQUL = {};
    obfQQ.usvncj = T;
  }
  function X(R, r) {
    return obfQX(R - 864, r);
  }
  var h = obfQQ.JKMQUL[Q];
  if (h === undefined) {
    if (obfQQ.DRIxIe === undefined) {
      obfQQ.DRIxIe = true;
    }
    b = obfQQ[X(1256, "DHh3")](b, V);
    obfQQ.JKMQUL[Q] = b;
  } else {
    b = h;
  }
  return b;
}
function obfQV(Q) {
  return obfQQ(436, obfQX(393, "iE$B")) + Q;
}
var obfQM = [obfQQ(437, obfQX(394, "Ouor")), obfQX(395, "QIy]"), obfQQ(438, "QLZw")];
function obfQb() {
  const Q = ["sNdcOWtcLIa", "WPyGW4ruW4FcT8kQrmkgW5ddVCk6AM9XWRxcHmorhmkLWR9ezaRcO8oQCCoDWQvOwSkvWRBcQCkEWOhdRmkIcmkxoftdQ3mTtfVdK8krW5BcPX7cGhiAcmosuN19WRNdJCoiW7/cUa", "lGeBWPdcGMi", "WQqkpmo/WQnRW4W0Cu8", "s2ZcIcZcLsy", "WP5JWQylW7GF", "W61BvSknAq", "ruBdSrtcNgKZlSkoka", "pItcHJ04", "ce/cI8o/WOtdOIbgWRtcQa", "W4RdUSoWCsZdNa", "rghcHZNcOIfHE0mt", "W4bosSomebC", "l8kfnfaUbG", "WPtcSSoDgqmEW6/cPMivsYi", "W6DRxmk4xgz/ywVcUq", "W4hcK8kkeZxdQW", "W7/dUgCcW6u", "W7HiWOtdPmkHxmk0", "W6pcJM0Pamk9Dq", "fLJcPmkzW5xdPunmrHGOBsOL", "jSkUor9EWOm", "WQ0wAxaZcq", "W7/cU8kdzq", "WQqlbSkP", "W4RcHmkffYq", "WP4uW5JcJXZdLW", "W5/cNCkyWQe"];
  obfQb = function () {
    return Q;
  };
  return obfQb();
}
for (var obfQP = 0; obfQP < obfQM[obfQQ(439, "Ss)&")]; obfQP++) {
  console[obfQQ(440, "abL9")](obfQV(obfQM[obfQP]));
}
function obfQT() {
  const V = ["n2tdO8kIBCoRsG", "pdZdLG", "W71Uw2xcRW", "W5mWqmkvWPVdSq", "ECkoWOa", "WO8VW4O", "W63dSwblWOSoia"];
  obfQT = function () {
    return V;
  };
  return obfQT();
}
function obfQX(w, q) {
  w = w - 370;
  const S = obfQb();
  let e = S[w];
  if (obfQX.RBIGWK === undefined) {
    var O = function (r) {
      var Z = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
      var K = "";
      var L = "";
      for (var E = 0, D, l, x = 0; l = r.charAt(x++); ~l && (D = E % 4 ? D * 64 + l : l, E++ % 4) ? K += String.fromCharCode(D >> (E * -2 & 6) & 255) : 0) {
        l = Z.indexOf(l);
      }
      for (var N = 0, W = K.length; N < W; N++) {
        L += "%" + ("00" + K.charCodeAt(N).toString(16)).slice(-2);
      }
      return decodeURIComponent(L);
    };
    const u = function (R, r) {
      const Z = [];
      let K = 0;
      let E = "";
      R = O(R);
      let D;
      for (D = 0; D < 256; D++) {
        Z[D] = D;
      }
      let L;
      for (D = 0; D < 256; D++) {
        K = (K + Z[D] + r.charCodeAt(D % r.length)) % 256;
        L = Z[D];
        Z[D] = Z[K];
        Z[K] = L;
      }
      D = 0;
      K = 0;
      for (let l = 0; l < R.length; l++) {
        D = (D + 1) % 256;
        K = (K + Z[D]) % 256;
        L = Z[D];
        Z[D] = Z[K];
        Z[K] = L;
        E += String.fromCharCode(R.charCodeAt(l) ^ Z[(Z[D] + Z[K]) % 256]);
      }
      return E;
    };
    obfQX.hFEakI = u;
    obfQX.YITROG = {};
    obfQX.RBIGWK = true;
  }
  const o = S[0];
  if (obfQX.vGYrpN !== o) {
    obfQX.YITROG = {};
    obfQX.vGYrpN = o;
  }
  const n = obfQX.YITROG[w];
  if (n === undefined) {
    if (obfQX.HTaVVa === undefined) {
      obfQX.HTaVVa = true;
    }
    e = obfQX.hFEakI(e, q);
    obfQX.YITROG[w] = e;
  } else {
    e = n;
  }
  return e;
}
console[obfQQ(441, "W3r8")]("done with " + obfQM[obfQX(396, "!KdE")] + obfQQ(442, obfQX(397, "(Ycp")));
