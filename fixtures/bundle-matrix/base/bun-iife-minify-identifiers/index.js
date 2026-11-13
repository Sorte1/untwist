(() => {
  var he = Object.create;
  var Ie = Object.getPrototypeOf;
  var j = Object.defineProperty;
  var ve = Object.getOwnPropertyNames;
  var q = Object.prototype.hasOwnProperty;
  function we(e) {
    return this[e];
  }
  var Ae;
  var Ee;
  var T = (e, r, t) => {
    var o = e != null && typeof e === "object";
    if (o) {
      var i = r ? Ae ??= new WeakMap : Ee ??= new WeakMap;
      var a = i.get(e);
      if (a)
        return a;
    }
    t = e != null ? he(Ie(e)) : {};
    const c = r || !e || !e.__esModule || !q.call(e, "default") ? j(t, "default", { value: e, enumerable: true }) : t;
    if (e && typeof e === "object" || typeof e === "function") {
      for (let p of ve(e))
        if (!q.call(c, p))
          j(c, p, {
            get: we.bind(e, p),
            enumerable: true
          });
    }
    if (o)
      i.set(e, c);
    return c;
  };
  var n = (e, r) => () => (r || e((r = { exports: {} }).exports, r), r.exports);
  var Fe = (e, r, t) => () => {
    if (e)
      try {
        r = e(e = 0);
      } catch (o) {
        t = [o];
      }
    if (t)
      throw t[0];
    return r;
  };

  // ../node_modules/lodash/_baseSlice.js
  var O = n(function(Kr, S) {
    function Ne(e, r, t) {
      var o = -1, i = e.length;
      if (r < 0) {
        r = -r > i ? 0 : i + r;
      }
      t = t > i ? i : t;
      if (t < 0) {
        t += i;
      }
      i = r > t ? 0 : t - r >>> 0;
      r >>>= 0;
      var a = Array(i);
      while (++o < i) {
        a[o] = e[o + r];
      }
      return a;
    }
    S.exports = Ne;
  });

  // ../node_modules/lodash/eq.js
  var I = n(function(Qr, h) {
    function Pe(e, r) {
      return e === r || e !== e && r !== r;
    }
    h.exports = Pe;
  });

  // ../node_modules/lodash/_freeGlobal.js
  var w = n(function(Vr, v) {
    var ke = typeof global == "object" && global && global.Object === Object && global;
    v.exports = ke;
  });

  // ../node_modules/lodash/_root.js
  var E = n(function(Zr, A) {
    var Ge = w();
    var Le = typeof self == "object" && self && self.Object === Object && self;
    var Me = Ge || Le || Function("return this")();
    A.exports = Me;
  });

  // ../node_modules/lodash/_Symbol.js
  var g = n(function(zr, F) {
    var Re = E();
    var _e = Re.Symbol;
    F.exports = _e;
  });

  // ../node_modules/lodash/_getRawTag.js
  var G = n(function(et, k) {
    var N = g();
    var P = Object.prototype;
    var $e = P.hasOwnProperty;
    var Be = P.toString;
    var s = N ? N.toStringTag : undefined;
    function Ce(e) {
      var r = $e.call(e, s), t = e[s];
      try {
        e[s] = undefined;
        var o = true;
      } catch (a) {}
      var i = Be.call(e);
      if (o) {
        if (r) {
          e[s] = t;
        } else {
          delete e[s];
        }
      }
      return i;
    }
    k.exports = Ce;
  });

  // ../node_modules/lodash/_objectToString.js
  var M = n(function(rt, L) {
    var Xe = Object.prototype;
    var Ue = Xe.toString;
    function He(e) {
      return Ue.call(e);
    }
    L.exports = He;
  });

  // ../node_modules/lodash/_baseGetTag.js
  var y = n(function(tt, B) {
    var R = g();
    var Je = G();
    var We = M();
    var Ye = "[object Null]";
    var De = "[object Undefined]";
    var _ = R ? R.toStringTag : undefined;
    function Ke(e) {
      if (e == null) {
        return e === undefined ? De : Ye;
      }
      return _ && _ in Object(e) ? Je(e) : We(e);
    }
    B.exports = Ke;
  });

  // ../node_modules/lodash/isObject.js
  var l = n(function(nt, C) {
    function Qe(e) {
      var r = typeof e;
      return e != null && (r == "object" || r == "function");
    }
    C.exports = Qe;
  });

  // ../node_modules/lodash/isFunction.js
  var U = n(function(ot, X) {
    var Ve = y();
    var Ze = l();
    var ze = "[object AsyncFunction]";
    var er = "[object Function]";
    var rr = "[object GeneratorFunction]";
    var tr = "[object Proxy]";
    function nr(e) {
      if (!Ze(e)) {
        return false;
      }
      var r = Ve(e);
      return r == er || r == rr || r == ze || r == tr;
    }
    X.exports = nr;
  });

  // ../node_modules/lodash/isLength.js
  var J = n(function(it, H) {
    var or = 9007199254740991;
    function ir(e) {
      return typeof e == "number" && e > -1 && e % 1 == 0 && e <= or;
    }
    H.exports = ir;
  });

  // ../node_modules/lodash/isArrayLike.js
  var Y = n(function(at, W) {
    var ar = U();
    var cr = J();
    function sr(e) {
      return e != null && cr(e.length) && !ar(e);
    }
    W.exports = sr;
  });

  // ../node_modules/lodash/_isIndex.js
  var K = n(function(ct, D) {
    var lr = 9007199254740991;
    var fr = /^(?:0|[1-9]\d*)$/;
    function ur(e, r) {
      var t = typeof e;
      r = r == null ? lr : r;
      return !!r && (t == "number" || t != "symbol" && fr.test(e)) && (e > -1 && e % 1 == 0 && e < r);
    }
    D.exports = ur;
  });

  // ../node_modules/lodash/_isIterateeCall.js
  var V = n(function(st, Q) {
    var br = I();
    var pr = Y();
    var gr = K();
    var yr = l();
    function mr(e, r, t) {
      if (!yr(t)) {
        return false;
      }
      var o = typeof r;
      if (o == "number" ? pr(t) && gr(r, t.length) : o == "string" && (r in t)) {
        return br(t[r], e);
      }
      return false;
    }
    Q.exports = mr;
  });

  // ../node_modules/lodash/_trimmedEndIndex.js
  var z = n(function(lt, Z) {
    var xr = /\s/;
    function dr(e) {
      var r = e.length;
      while (r-- && xr.test(e.charAt(r))) {}
      return r;
    }
    Z.exports = dr;
  });

  // ../node_modules/lodash/_baseTrim.js
  var re = n(function(ft, ee) {
    var jr = z();
    var qr = /^\s+/;
    function Tr(e) {
      return e ? e.slice(0, jr(e) + 1).replace(qr, "") : e;
    }
    ee.exports = Tr;
  });

  // ../node_modules/lodash/isObjectLike.js
  var ne = n(function(ut, te) {
    function Sr(e) {
      return e != null && typeof e == "object";
    }
    te.exports = Sr;
  });

  // ../node_modules/lodash/isSymbol.js
  var ie = n(function(bt, oe) {
    var Or = y();
    var hr = ne();
    var Ir = "[object Symbol]";
    function vr(e) {
      return typeof e == "symbol" || hr(e) && Or(e) == Ir;
    }
    oe.exports = vr;
  });

  // ../node_modules/lodash/toNumber.js
  var le = n(function(pt, se) {
    var wr = re();
    var ae = l();
    var Ar = ie();
    var ce = 0 / 0;
    var Er = /^[-+]0x[0-9a-f]+$/i;
    var Fr = /^0b[01]+$/i;
    var Nr = /^0o[0-7]+$/i;
    var Pr = parseInt;
    function kr(e) {
      if (typeof e == "number") {
        return e;
      }
      if (Ar(e)) {
        return ce;
      }
      if (ae(e)) {
        var r = typeof e.valueOf == "function" ? e.valueOf() : e;
        e = ae(r) ? r + "" : r;
      }
      if (typeof e != "string") {
        return e === 0 ? e : +e;
      }
      e = wr(e);
      var t = Fr.test(e);
      return t || Nr.test(e) ? Pr(e.slice(2), t ? 2 : 8) : Er.test(e) ? ce : +e;
    }
    se.exports = kr;
  });

  // ../node_modules/lodash/toFinite.js
  var be = n(function(gt, ue) {
    var Gr = le();
    var fe = 1 / 0;
    var Lr = 179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000;
    function Mr(e) {
      if (!e) {
        return e === 0 ? e : 0;
      }
      e = Gr(e);
      if (e === fe || e === -fe) {
        var r = e < 0 ? -1 : 1;
        return r * Lr;
      }
      return e === e ? e : 0;
    }
    ue.exports = Mr;
  });

  // ../node_modules/lodash/toInteger.js
  var ge = n(function(yt, pe) {
    var Rr = be();
    function _r(e) {
      var r = Rr(e), t = r % 1;
      return r === r ? t ? r - t : r : 0;
    }
    pe.exports = _r;
  });

  // ../node_modules/lodash/chunk.js
  var me = n(function(mt, ye) {
    var $r = O();
    var Br = V();
    var Cr = ge();
    var Xr = Math.ceil;
    var Ur = Math.max;
    function Hr(e, r, t) {
      if (t ? Br(e, r, t) : r === undefined) {
        r = 1;
      } else {
        r = Ur(Cr(r), 0);
      }
      var o = e == null ? 0 : e.length;
      if (!o || r < 1) {
        return [];
      }
      var i = 0, a = 0, c = Array(Xr(o / r));
      while (i < o) {
        c[a++] = $r(e, i, i += r);
      }
      return c;
    }
    ye.exports = Hr;
  });

  // cycle-b.js
  var xe = n(function(x) {
    console.log("eval cycle-b");
    var m = d();
    x.seen = "b saw " + m.early + "," + m.late;
    x.check = function() {
      return "b later " + m.late;
    };
  });

  // cycle-a.js
  var d = n(function(u) {
    console.log("eval cycle-a start");
    u.early = "a-early";
    var de = xe();
    u.late = "a-late";
    u.report = function() {
      return de.seen + " " + de.check();
    };
    console.log("eval cycle-a end");
  });

  // lazy.js
  var Wr = 42, Yr = "lazy-default";
  var Te = Fe(() => {
    console.log("eval lazy");
  });

  // index.js
  var Se = T(me());
  // data.json
  var f = {
    name: "fixture",
    items: [1, 2, 3, 4, 5]
  };

  // index.js
  var Oe = T(d());

  // util.js
  console.log("eval util");
  var je = (e) => `[${e.padStart(8, ".")}]`;
  function qe(e) {
    return e.reduce((r, t) => r + t, 0);
  }

  // index.js
  var b = f.items;
  console.log("eval index");
  console.log("last " + b[b.length - 1] + " own " + Object.prototype.hasOwnProperty.call(f, "name"));
  console.log("replace " + "a-b-c".split("-").join("+"));
  console.log("chunk " + JSON.stringify(Se.default(b, 2)));
  console.log("cycle " + Oe.report());
  console.log("label " + je(f.name) + " total " + qe(b));
  Promise.resolve().then(() => (Te(), {})).then((e) => {
    console.log("lazy " + Wr + " " + Yr);
    return Promise.allSettled([Promise.resolve(1), Promise.reject(new Error("no"))]);
  }).then((e) => console.log("settled " + e.map((r) => r.status).join(",")));
})();
