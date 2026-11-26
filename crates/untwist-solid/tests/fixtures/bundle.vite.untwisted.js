const he = (e, t) => e === t;
const de = Symbol("solid-track");
const V = { equals: he };
let pe = ie;
const N = 1;
const q = 2;
const ee = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var h = null;
let j = null;
let ge = null;
let d = null;
let $ = null;
let k = null;
let G = 0;
function U(e, t) {
  const n = d;
  const l = h;
  const s = e.length === 0;
  const i = t === undefined ? l : t;
  const o = s ? ee : {
    owned: null,
    cleanups: null,
    context: i ? i.context : null,
    owner: i
  };
  const r = s ? e : () => e($e(o));
  h = o;
  d = null;
  try {
    return I(r, true);
  } finally {
    d = n;
    h = l;
  }
}
function $e(e) {
  return () => T(() => D(e));
}
function v(e, t) {
  t = t ? Object.assign({}, V, t) : V;
  const n = {
    value: e,
    observers: null,
    observerSlots: null,
    comparator: t.equals || undefined
  };
  const l = (s) => {
    if (typeof s == "function") {
      s = s(n.value);
    }
    return le(n, s);
  };
  return [ne.bind(n), l];
}
function A(e, t, n) {
  const l = se(e, t, false, N);
  H(l);
}
function E(e, t, n) {
  n = n ? Object.assign({}, V, n) : V;
  const l = se(e, t, true, 0);
  l.observers = null;
  l.observerSlots = null;
  l.comparator = n.equals || undefined;
  H(l);
  return ne.bind(l);
}
function T(e) {
  if (d === null) {
    return e();
  }
  const t = d;
  d = null;
  try {
    return e();
  } finally {
    d = t;
  }
}
function be(e) {
  if (h !== null) {
    if (h.cleanups === null) {
      h.cleanups = [e];
    } else {
      h.cleanups.push(e);
    }
  }
  return e;
}
function we(e, t) {
  const n = Symbol("context");
  return {
    id: n,
    Provider: Ae(n),
    defaultValue: e
  };
}
function me(e) {
  let t;
  if (h && h.context && (t = h.context[e.id]) !== undefined) {
    return t;
  } else {
    return e.defaultValue;
  }
}
function ye(e) {
  const t = E(e);
  const n = E(() => Q(t()));
  n.toArray = () => {
    const l = n();
    if (Array.isArray(l)) {
      return l;
    } else if (l != null) {
      return [l];
    } else {
      return [];
    }
  };
  return n;
}
function te(e) {
  e.observerSlots = e.observers.map((t, n) => {
    const l = t.sources;
    if (!l) {
      return -1;
    }
    for (let s = l.length - 1; s >= 0; s--) {
      if (l[s] === e && t.sourceSlots && t.sourceSlots[s] === n) {
        return s;
      }
    }
    return l.lastIndexOf(e);
  });
}
function ne() {
  if (this.sources && this.state) {
    if (this.state === N) {
      H(this);
    } else {
      const e = $;
      $ = null;
      I(() => R(this), false);
      $ = e;
    }
  }
  if (d) {
    const e = this.observers;
    if (!e || e[e.length - 1] !== d) {
      const t = e ? e.length : 0;
      if (d.sources) {
        d.sources.push(this);
        d.sourceSlots.push(t);
      } else {
        const n = [this];
        const l = [t];
        d.sources = n;
        d.sourceSlots = l;
      }
      if (e) {
        if (!this.observerSlots || this.observerSlots.length !== e.length) {
          te(this);
        }
        e.push(d);
        this.observerSlots.push(d.sources.length - 1);
      } else {
        const n = [d];
        const l = [d.sources.length - 1];
        this.observers = n;
        this.observerSlots = l;
      }
    }
  }
  return this.value;
}
function le(e, t, n) {
  const l = e.value;
  if (!e.comparator || !e.comparator(l, t)) {
    e.value = t;
    if (e.observers && e.observers.length) {
      I(() => {
        for (let s = 0; s < e.observers.length; s += 1) {
          const i = e.observers[s];
          const o = j && j.running;
          if (o) {
            j.disposed.has(i);
          }
          if (o ? !i.tState : !i.state) {
            if (i.pure) {
              $.push(i);
            } else {
              k.push(i);
            }
            if (i.observers) {
              oe(i);
            }
          }
          if (!o) {
            i.state = N;
          }
        }
        if ($.length > 1e6) {
          $ = [];
          throw new Error();
        }
      }, false);
    }
  }
  return t;
}
function H(e) {
  if (!e.fn) {
    return;
  }
  D(e, true);
  const t = G;
  ve(e, e.value, t);
}
function ve(e, t, n) {
  let l;
  const s = h;
  const i = d;
  d = h = e;
  try {
    l = e.fn(t);
  } catch (o) {
    if (e.pure) {
      e.state = N;
      if (e.owned) {
        e.owned.forEach((r) => D(r));
      }
      e.owned = null;
    }
    e.updatedAt = n + 1;
    return ue(o);
  } finally {
    d = i;
    h = s;
  }
  if (!e.updatedAt || e.updatedAt <= n) {
    if (e.updatedAt != null && "observers" in e) {
      le(e, l);
    } else {
      e.value = l;
    }
    e.updatedAt = n;
  }
}
function se(e, t, n, l = N, s) {
  const i = {
    fn: e,
    state: l,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: t,
    owner: h,
    context: h ? h.context : null,
    pure: n
  };
  if (h !== null) {
    if (h !== ee) {
      if (h.owned) {
        h.owned.push(i);
      } else {
        h.owned = [i];
      }
    }
  }
  return i;
}
function re(e) {
  if (e.state === 0) {
    return;
  }
  if (e.state === q) {
    return R(e);
  }
  if (e.suspense && T(e.suspense.inFallback)) {
    return e.suspense.effects.push(e);
  }
  const t = [e];
  while ((e = e.owner) && (!e.updatedAt || e.updatedAt < G)) {
    if (e.state) {
      t.push(e);
    }
  }
  for (let n = t.length - 1; n >= 0; n--) {
    e = t[n];
    if (e.state === N) {
      H(e);
    } else if (e.state === q) {
      const l = $;
      $ = null;
      I(() => R(e, t[0]), false);
      $ = l;
    }
  }
}
function I(e, t) {
  if ($) {
    return e();
  }
  let n = false;
  if (!t) {
    $ = [];
  }
  if (k) {
    n = true;
  } else {
    k = [];
  }
  G++;
  try {
    const l = e();
    _e(n);
    return l;
  } catch (l) {
    if (!n) {
      k = null;
    }
    $ = null;
    ue(l);
  }
}
function _e(e) {
  if ($) {
    ie($);
    $ = null;
  }
  if (e) {
    return;
  }
  const t = k;
  k = null;
  if (t.length) {
    I(() => pe(t), false);
  }
}
function ie(e) {
  for (let t = 0; t < e.length; t++) {
    re(e[t]);
  }
}
function R(e, t) {
  e.state = 0;
  for (let n = 0; n < e.sources.length; n += 1) {
    const l = e.sources[n];
    if (l.sources) {
      const s = l.state;
      if (s === N) {
        if (l !== t && (!l.updatedAt || l.updatedAt < G)) {
          re(l);
        }
      } else if (s === q) {
        R(l, t);
      }
    }
  }
}
function oe(e) {
  for (let t = 0; t < e.observers.length; t += 1) {
    const n = e.observers[t];
    if (!n.state) {
      n.state = q;
      if (n.pure) {
        $.push(n);
      } else {
        k.push(n);
      }
      if (n.observers) {
        oe(n);
      }
    }
  }
}
function D(e, t) {
  let n;
  if (e.sources) {
    while (e.sources.length) {
      const l = e.sources.pop();
      const s = e.sourceSlots.pop();
      const i = l.observers;
      if (i && i.length) {
        if (!l.observerSlots || l.observerSlots.length !== i.length) {
          te(l);
        }
        const o = i.pop();
        const r = l.observerSlots.pop();
        if (s < i.length) {
          o.sourceSlots[r] = s;
          i[s] = o;
          l.observerSlots[s] = r;
        }
      }
      if (!t) {
        if (i) {
          i.length;
        }
      }
    }
  }
  if (e.tOwned) {
    for (n = e.tOwned.length - 1; n >= 0; n--) {
      D(e.tOwned[n]);
    }
    delete e.tOwned;
  }
  if (e.owned) {
    for (n = e.owned.length - 1; n >= 0; n--) {
      D(e.owned[n]);
    }
    e.owned = null;
  }
  if (e.cleanups) {
    for (n = e.cleanups.length - 1; n >= 0; n--) {
      e.cleanups[n]();
    }
    e.cleanups = null;
  }
  e.state = 0;
}
function Se(e) {
  if (e instanceof Error) {
    return e;
  } else {
    return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
  }
}
function ue(e, t = h) {
  throw Se(e);
}
function Q(e) {
  if (typeof e == "function" && !e.length) {
    return Q(e());
  }
  if (Array.isArray(e)) {
    const t = [];
    for (let n = 0; n < e.length; n++) {
      const l = Q(e[n]);
      if (Array.isArray(l)) {
        if (l.length < 32768) {
          t.push.apply(t, l);
        } else {
          for (let s = 0; s < l.length; s++) {
            t.push(l[s]);
          }
        }
      } else {
        t.push(l);
      }
    }
    return t;
  }
  return e;
}
function Ae(e, t) {
  return function (l) {
    let s;
    A(() => s = T(() => {
      h.context = {
        ...h.context,
        [e]: l.value
      };
      return ye(() => l.children);
    }), undefined);
    return s;
  };
}
const xe = Symbol("fallback");
function J(e) {
  for (let t = 0; t < e.length; t++) {
    e[t]();
  }
}
function Ce(e, t, n = {}) {
  let l = [];
  let s = [];
  let i = [];
  let o = 0;
  let r = t.length > 1 ? [] : null;
  be(() => J(i));
  return () => {
    const f = e() || [];
    const c = f.length;
    let a;
    let u;
    f[de];
    return T(() => {
      let b;
      let x;
      let C;
      let P;
      let M;
      let w;
      let m;
      let S;
      let O;
      if (c === 0) {
        if (o !== 0) {
          J(i);
          i = [];
          l = [];
          s = [];
          o = 0;
          r &&= [];
        }
        if (n.fallback) {
          l = [xe];
          s[0] = U((ae) => {
            i[0] = ae;
            return n.fallback();
          });
          o = 1;
        }
      } else if (o === 0) {
        s = new Array(c);
        u = 0;
        for (; u < c; u++) {
          l[u] = f[u];
          s[u] = U(_);
        }
        o = c;
      } else {
        C = new Array(c);
        P = new Array(c);
        if (r) {
          M = new Array(c);
        }
        w = 0;
        m = Math.min(o, c);
        for (; w < m && l[w] === f[w]; w++);
        m = o - 1;
        S = c - 1;
        for (; m >= w && S >= w && l[m] === f[S]; m--, S--) {
          C[S] = s[m];
          P[S] = i[m];
          if (r) {
            M[S] = r[m];
          }
        }
        b = new Map();
        x = new Array(S + 1);
        u = S;
        for (; u >= w; u--) {
          O = f[u];
          a = b.get(O);
          x[u] = a === undefined ? -1 : a;
          b.set(O, u);
        }
        for (a = w; a <= m; a++) {
          O = l[a];
          u = b.get(O);
          if (u !== undefined && u !== -1) {
            C[u] = s[a];
            P[u] = i[a];
            if (r) {
              M[u] = r[a];
            }
            u = x[u];
            b.set(O, u);
          } else {
            i[a]();
          }
        }
        for (u = w; u < c; u++) {
          if (u in C) {
            s[u] = C[u];
            i[u] = P[u];
            if (r) {
              r[u] = M[u];
              r[u](u);
            }
          } else {
            s[u] = U(_);
          }
        }
        s = s.slice(0, o = c);
        l = f.slice(0);
      }
      return s;
    });
    function _(b) {
      i[u] = b;
      if (r) {
        const [x, C] = v(u);
        r[u] = C;
        return t(f[u], x);
      }
      return t(f[u]);
    }
  };
}
function y(e, t) {
  return T(() => e(t || {}));
}
const ke = (e) => `Stale read from <${e}>.`;
function fe(e) {
  const t = "fallback" in e && { fallback: () => e.fallback };
  return E(Ce(() => e.each, e.children, t || undefined));
}
function W(e) {
  const t = e.keyed;
  const n = E(() => e.when, undefined, undefined);
  const l = t ? n : E(n, undefined, { equals: (s, i) => !s == !i });
  return E(() => {
    const s = l();
    if (s) {
      const i = e.children;
      if (typeof i == "function" && i.length > 0) {
        return T(() => i(t ? s : () => {
          if (!T(l)) {
            throw ke("Show");
          }
          return n();
        }));
      } else {
        return i;
      }
    }
    return e.fallback;
  }, undefined, undefined);
}
const Y = (e) => E(() => e());
function Ee(e, t, n) {
  const l = n.length;
  let s = t.length;
  let i = l;
  let o = 0;
  let r = 0;
  const f = t[s - 1].nextSibling;
  let c = null;
  while (o < s || r < i) {
    if (t[o] === n[r]) {
      o++;
      r++;
      continue;
    }
    while (t[s - 1] === n[i - 1]) {
      s--;
      i--;
    }
    if (s === o) {
      const a = i < l ? r ? n[r - 1].nextSibling : n[i - r] : f;
      while (r < i) {
        e.insertBefore(n[r++], a);
      }
    } else if (i === r) {
      while (o < s) {
        if (!c || !c.has(t[o])) {
          t[o].remove();
        }
        o++;
      }
    } else if (t[o] === n[i - 1] && n[r] === t[s - 1]) {
      const a = t[--s].nextSibling;
      e.insertBefore(n[r++], t[o++].nextSibling);
      e.insertBefore(n[--i], a);
      t[s] = n[i];
    } else {
      if (!c) {
        c = new Map();
        let u = r;
        while (u < i) {
          c.set(n[u], u++);
        }
      }
      const a = c.get(t[o]);
      if (a != null) {
        if (r < a && a < i) {
          let u = o;
          let _ = 1;
          let b;
          while (++u < s && u < i && (b = c.get(t[u])) != null && b === a + _) {
            _++;
          }
          if (_ > a - r) {
            const x = t[o];
            while (r < a) {
              e.insertBefore(n[r++], x);
            }
          } else {
            e.replaceChild(n[r++], t[o++]);
          }
        } else {
          o++;
        }
      } else {
        t[o++].remove();
      }
    }
  }
}
const Z = "_$DX_DELEGATE";
function Te(e, t, n, l = {}) {
  let s;
  U((i) => {
    s = i;
    if (t === document) {
      e();
    } else {
      p(t, e(), !n && t.firstChild ? null : undefined, n);
    }
  }, l.owner);
  return () => {
    s();
    t.textContent = "";
  };
}
function g(e, t, n, l) {
  let s;
  const i = () => {
    const r = document.createElement("template");
    r.innerHTML = e;
    return r.content.firstChild;
  };
  const o = () => (s ||= i()).cloneNode(true);
  o.cloneNode = o;
  return o;
}
function B(e, t = window.document) {
  const n = t[Z] ||= new Set();
  for (let l = 0, s = e.length; l < s; l++) {
    const i = e[l];
    if (!n.has(i)) {
      n.add(i);
      t.addEventListener(i, Le);
    }
  }
}
function Ne(e, t, n) {
  if (n == null) {
    e.removeAttribute(t);
  } else {
    e.setAttribute(t, n);
  }
}
function Oe(e, t) {
  if (t == null) {
    e.removeAttribute("class");
  } else {
    e.className = t;
  }
}
function p(e, t, n, l) {
  if (n !== undefined && !l) {
    l = [];
  }
  if (typeof t != "function") {
    return F(e, t, l, n);
  }
  A((s) => F(e, t(), s, n), l);
}
function Le(e) {
  let t = e.target;
  const n = `$$${e.type}`;
  const l = e.target;
  const s = e.currentTarget;
  const i = (f) => Object.defineProperty(e, "target", {
    configurable: true,
    value: f
  });
  const o = () => {
    const f = t[n];
    if (f && !t.disabled) {
      const c = t[`${n}Data`];
      if (c !== undefined) {
        f.call(t, c, e);
      } else {
        f.call(t, e);
      }
      if (e.cancelBubble) {
        return;
      }
    }
    if (t.host && typeof t.host != "string" && !t.host._$host && t.contains(e.target)) {
      i(t.host);
    }
    return true;
  };
  const r = () => {
    while (o() && (t = t._$host || t.parentNode || t.host));
  };
  Object.defineProperty(e, "currentTarget", {
    configurable: true,
    get() {
      return t || document;
    }
  });
  try {
    if (e.composedPath) {
      const f = e.composedPath();
      i(f[0]);
      for (let c = 0; c < f.length - 2 && (t = f[c], !!o()); c++) {
        if (t._$host) {
          t = t._$host;
          r();
          break;
        }
        if (t.parentNode === s) {
          break;
        }
      }
    } else {
      r();
    }
    i(l);
  } finally {
    e = t = null;
  }
}
function F(e, t, n, l, s) {
  while (typeof n == "function") {
    n = n();
  }
  if (t === n) {
    return n;
  }
  const i = typeof t;
  const o = l !== undefined;
  e = o && n[0] && n[0].parentNode || e;
  if (i === "string" || i === "number") {
    if (i === "number" && (t = t.toString(), t === n)) {
      return n;
    }
    if (o) {
      let r = n[0];
      if (r && r.nodeType === 3) {
        if (r.data !== t) {
          r.data = t;
        }
      } else {
        r = document.createTextNode(t);
      }
      n = L(e, n, l, r);
    } else if (n !== "" && typeof n == "string") {
      n = e.firstChild.data = t;
    } else {
      n = e.textContent = t;
    }
  } else if (t == null || i === "boolean") {
    n = L(e, n, l);
  } else {
    if (i === "function") {
      A(() => {
        let r = t();
        while (typeof r == "function") {
          r = r();
        }
        n = F(e, r, n, l);
      });
      return () => n;
    }
    if (Array.isArray(t)) {
      const r = [];
      const f = n && Array.isArray(n);
      if (X(r, t, n, s)) {
        A(() => n = F(e, r, n, l, true));
        return () => n;
      }
      if (r.length === 0) {
        n = L(e, n, l);
        if (o) {
          return n;
        }
      } else if (f) {
        if (n.length === 0) {
          z(e, r, l);
        } else {
          Ee(e, n, r);
        }
      } else {
        if (n) {
          L(e);
        }
        z(e, r);
      }
      n = r;
    } else if (t.nodeType) {
      if (Array.isArray(n)) {
        if (o) {
          return n = L(e, n, l, t);
        }
        L(e, n, null, t);
      } else if (n == null || n === "" || !e.firstChild) {
        e.appendChild(t);
      } else {
        e.replaceChild(t, e.firstChild);
      }
      n = t;
    }
  }
  return n;
}
function X(e, t, n, l) {
  let s = false;
  for (let i = 0, o = t.length; i < o; i++) {
    let r = t[i];
    const f = n && n[e.length];
    let c;
    if (r != null && r !== true && r !== false) {
      if ((c = typeof r) == "object" && r.nodeType) {
        e.push(r);
      } else if (Array.isArray(r)) {
        s = X(e, r, f) || s;
      } else if (c === "function") {
        if (l) {
          while (typeof r == "function") {
            r = r();
          }
          s = X(e, Array.isArray(r) ? r : [r], Array.isArray(f) ? f : [f]) || s;
        } else {
          e.push(r);
          s = true;
        }
      } else {
        const a = String(r);
        if (f && f.nodeType === 3 && f.data === a) {
          e.push(f);
        } else {
          e.push(document.createTextNode(a));
        }
      }
    }
  }
  return s;
}
function z(e, t, n = null) {
  for (let l = 0, s = t.length; l < s; l++) {
    e.insertBefore(t[l], n);
  }
}
function L(e, t, n, l) {
  if (n === undefined) {
    return e.textContent = "";
  }
  const s = l || document.createTextNode("");
  if (t.length) {
    let i = false;
    for (let o = t.length - 1; o >= 0; o--) {
      const r = t[o];
      if (s !== r) {
        const f = r.parentNode === e;
        if (!i && !o) {
          if (f) {
            e.replaceChild(s, r);
          } else {
            e.insertBefore(s, n);
          }
        } else if (f) {
          r.remove();
        }
      } else {
        i = true;
      }
    }
  } else {
    e.insertBefore(s, n);
  }
  return [s];
}
var Be = g("<span class=badge>");
function K(e) {
  return (() => {
    const t = Be();
    p(t, () => e.label, null);
    p(t, () => e.children, null);
    A(() => Ne(t, "data-tone", e.tone ?? "plain"));
    return t;
  })();
}
var De = g("<em>child ");
var Ie = g("<div class=parent><button>more");
function Pe() {
  const [e, t] = v(0);
  return (() => {
    const n = Ie();
    const l = n.firstChild;
    p(n, y(K, { label: "static" }), l);
    p(n, y(K, {
      get label() {
        return `count ${e()}`;
      },
      get tone() {
        if (e() > 1) {
          return "hot";
        } else {
          return undefined;
        }
      },
      get children() {
        const s = De();
        s.firstChild;
        p(s, e, null);
        return s;
      }
    }), l);
    p(n, y(K, {
      label: "text",
      children: "plain text"
    }), l);
    l.$$click = () => t(e() + 1);
    return n;
  })();
}
B(["click"]);
var Me = g("<div><p>Count: </p><button>+1");
function je() {
  const [e, t] = v(0);
  return (() => {
    const n = Me();
    const l = n.firstChild;
    l.firstChild;
    const s = l.nextSibling;
    p(l, e, null);
    s.$$click = () => t(e() + 1);
    return n;
  })();
}
B(["click"]);
var Ue = g("<ul>");
var Ve = g("<p> left");
var qe = g("<div class=todo><input><button class=add>add");
var Re = g("<p>nothing to do");
var Fe = g("<li><button class=rm>x");
function Ge() {
  const [e, t] = v([]);
  const [n, l] = v("");
  let s = 0;
  const i = () => {
    if (n()) {
      t([...e(), {
        id: s++,
        text: n()
      }]);
      l("");
    }
  };
  return (() => {
    const o = qe();
    const r = o.firstChild;
    const f = r.nextSibling;
    r.$$input = (c) => l(c.currentTarget.value);
    f.$$click = i;
    p(o, y(W, {
      get when() {
        return e().length;
      },
      get fallback() {
        return Re();
      },
      get children() {
        return [(() => {
          const c = Ue();
          p(c, y(fe, {
            get each() {
              return e();
            },
            children: (a) => (() => {
              const u = Fe();
              const _ = u.firstChild;
              p(u, () => a.text, _);
              _.$$click = () => t(e().filter((b) => b.id !== a.id));
              return u;
            })()
          }));
          return c;
        })(), (() => {
          const c = Ve();
          const a = c.firstChild;
          p(c, () => e().length, a);
          return c;
        })()];
      }
    }), null);
    A(() => r.value = n());
    return o;
  })();
}
B(["input", "click"]);
var He = g("<span>");
var Ke = g("<div><button>flip");
const ce = we("light");
function Qe() {
  const e = me(ce);
  return (() => {
    const t = He();
    p(t, e);
    A(() => Oe(t, e()));
    return t;
  })();
}
function We() {
  const [e, t] = v("dark");
  return (() => {
    const n = Ke();
    const l = n.firstChild;
    p(n, y(ce.Provider, {
      value: e,
      get children() {
        return y(Qe, {});
      }
    }), l);
    l.$$click = () => t(e() === "dark" ? "light" : "dark");
    return n;
  })();
}
B(["click"]);
var Xe = g("<button>more");
var Je = g("<div class=mixed><h3>");
var Ye = g("<span class=tag>");
var Ze = g("<s>hidden");
function ze(e) {
  const [t, n] = v(true);
  const [l, s] = v(["x"]);
  return (() => {
    const i = Je();
    const o = i.firstChild;
    o.$$click = () => n(!t());
    p(o, () => e.heading);
    p(i, y(W, {
      get when() {
        return t();
      },
      get children() {
        return [y(fe, {
          get each() {
            return l();
          },
          children: (r) => y(W, {
            when: r !== "hide",
            get fallback() {
              return Ze();
            },
            get children() {
              const f = Ye();
              p(f, r);
              return f;
            }
          })
        }), (() => {
          const r = Xe();
          r.$$click = () => s([...l(), "hide", "y"]);
          return r;
        })()];
      }
    }), null);
    A(() => i.classList.toggle("open", !!t()));
    return i;
  })();
}
B(["click"]);
var et = g("<div><button class=t>toggle</button><button class=n>inc");
var tt = g("<b>on");
var nt = g("<i>off");
var lt = g("<span>more than one");
function st() {
  const [e, t] = v(false);
  const [n, l] = v(0);
  return (() => {
    const s = et();
    const i = s.firstChild;
    const o = i.nextSibling;
    p(s, (() => {
      const r = Y(() => !!e());
      return () => r() ? tt() : nt();
    })(), i);
    p(s, (() => {
      const r = Y(() => n() > 1);
      return () => r() && lt();
    })(), i);
    p(s, () => n() > 0 ? "positive" : "zero", i);
    i.$$click = () => t(!e());
    o.$$click = () => l(n() + 1);
    return s;
  })();
}
B(["click"]);
window.__render = Te;
window.__component = [Pe, je, Ge, We, ze, st];