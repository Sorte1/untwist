const k = (e, t) => e === t;
const B = { equals: k };
let H = R;
const w = 1;
const E = 2;
const P = {
  owned: null,
  cleanups: null,
  context: null,
  owner: null
};
var h = null;
let A = null;
let Q = null;
let u = null;
let a = null;
let d = null;
let v = 0;
function W(e, t) {
  const s = u;
  const n = h;
  const i = e.length === 0;
  const o = t === undefined ? n : t;
  const r = i ? P : {
    owned: null,
    cleanups: null,
    context: o ? o.context : null,
    owner: o
  };
  const l = i ? e : () => e(X(r));
  h = r;
  u = null;
  try {
    return S(l, true);
  } finally {
    u = s;
    h = n;
  }
}
function X(e) {
  return () => O(() => y(e));
}
function J(e, t) {
  t = t ? Object.assign({}, B, t) : B;
  const s = {
    value: e,
    observers: null,
    observerSlots: null,
    comparator: t.equals || undefined
  };
  const n = (i) => {
    if (typeof i == "function") {
      i = i(s.value);
    }
    return V(s, i);
  };
  return [K.bind(s), n];
}
function $(e, t, s) {
  const n = Z(e, t, false, w);
  _(n);
}
function O(e) {
  if (u === null) {
    return e();
  }
  const t = u;
  u = null;
  try {
    return e();
  } finally {
    u = t;
  }
}
function M(e) {
  e.observerSlots = e.observers.map((t, s) => {
    const n = t.sources;
    if (!n) {
      return -1;
    }
    for (let i = n.length - 1; i >= 0; i--) {
      if (n[i] === e && t.sourceSlots && t.sourceSlots[i] === s) {
        return i;
      }
    }
    return n.lastIndexOf(e);
  });
}
function K() {
  if (this.sources && this.state) {
    if (this.state === w) {
      _(this);
    } else {
      const e = a;
      a = null;
      S(() => x(this), false);
      a = e;
    }
  }
  if (u) {
    const e = this.observers;
    if (!e || e[e.length - 1] !== u) {
      const t = e ? e.length : 0;
      if (u.sources) {
        u.sources.push(this);
        u.sourceSlots.push(t);
      } else {
        const s = [this];
        const n = [t];
        u.sources = s;
        u.sourceSlots = n;
      }
      if (e) {
        if (!this.observerSlots || this.observerSlots.length !== e.length) {
          M(this);
        }
        e.push(u);
        this.observerSlots.push(u.sources.length - 1);
      } else {
        const s = [u];
        const n = [u.sources.length - 1];
        this.observers = s;
        this.observerSlots = n;
      }
    }
  }
  return this.value;
}
function V(e, t, s) {
  const n = e.value;
  if (!e.comparator || !e.comparator(n, t)) {
    e.value = t;
    if (e.observers && e.observers.length) {
      S(() => {
        for (let i = 0; i < e.observers.length; i += 1) {
          const o = e.observers[i];
          const r = A && A.running;
          if (r) {
            A.disposed.has(o);
          }
          if (r ? !o.tState : !o.state) {
            if (o.pure) {
              a.push(o);
            } else {
              d.push(o);
            }
            if (o.observers) {
              q(o);
            }
          }
          if (!r) {
            o.state = w;
          }
        }
        if (a.length > 1e6) {
          a = [];
          throw new Error();
        }
      }, false);
    }
  }
  return t;
}
function _(e) {
  if (!e.fn) {
    return;
  }
  y(e, true);
  const t = v;
  Y(e, e.value, t);
}
function Y(e, t, s) {
  let n;
  const i = h;
  const o = u;
  u = h = e;
  try {
    n = e.fn(t);
  } catch (r) {
    if (e.pure) {
      e.state = w;
      if (e.owned) {
        e.owned.forEach((l) => y(l));
      }
      e.owned = null;
    }
    e.updatedAt = s + 1;
    return F(r);
  } finally {
    u = o;
    h = i;
  }
  if (!e.updatedAt || e.updatedAt <= s) {
    if (e.updatedAt != null && "observers" in e) {
      V(e, n);
    } else {
      e.value = n;
    }
    e.updatedAt = s;
  }
}
function Z(e, t, s, n = w, i) {
  const o = {
    fn: e,
    state: n,
    updatedAt: null,
    owned: null,
    sources: null,
    sourceSlots: null,
    cleanups: null,
    value: t,
    owner: h,
    context: h ? h.context : null,
    pure: s
  };
  if (h !== null) {
    if (h !== P) {
      if (h.owned) {
        h.owned.push(o);
      } else {
        h.owned = [o];
      }
    }
  }
  return o;
}
function j(e) {
  if (e.state === 0) {
    return;
  }
  if (e.state === E) {
    return x(e);
  }
  if (e.suspense && O(e.suspense.inFallback)) {
    return e.suspense.effects.push(e);
  }
  const t = [e];
  while ((e = e.owner) && (!e.updatedAt || e.updatedAt < v)) {
    if (e.state) {
      t.push(e);
    }
  }
  for (let s = t.length - 1; s >= 0; s--) {
    e = t[s];
    if (e.state === w) {
      _(e);
    } else if (e.state === E) {
      const n = a;
      a = null;
      S(() => x(e, t[0]), false);
      a = n;
    }
  }
}
function S(e, t) {
  if (a) {
    return e();
  }
  let s = false;
  if (!t) {
    a = [];
  }
  if (d) {
    s = true;
  } else {
    d = [];
  }
  v++;
  try {
    const n = e();
    z(s);
    return n;
  } catch (n) {
    if (!s) {
      d = null;
    }
    a = null;
    F(n);
  }
}
function z(e) {
  if (a) {
    R(a);
    a = null;
  }
  if (e) {
    return;
  }
  const t = d;
  d = null;
  if (t.length) {
    S(() => H(t), false);
  }
}
function R(e) {
  for (let t = 0; t < e.length; t++) {
    j(e[t]);
  }
}
function x(e, t) {
  e.state = 0;
  for (let s = 0; s < e.sources.length; s += 1) {
    const n = e.sources[s];
    if (n.sources) {
      const i = n.state;
      if (i === w) {
        if (n !== t && (!n.updatedAt || n.updatedAt < v)) {
          j(n);
        }
      } else if (i === E) {
        x(n, t);
      }
    }
  }
}
function q(e) {
  for (let t = 0; t < e.observers.length; t += 1) {
    const s = e.observers[t];
    if (!s.state) {
      s.state = E;
      if (s.pure) {
        a.push(s);
      } else {
        d.push(s);
      }
      if (s.observers) {
        q(s);
      }
    }
  }
}
function y(e, t) {
  let s;
  if (e.sources) {
    while (e.sources.length) {
      const n = e.sources.pop();
      const i = e.sourceSlots.pop();
      const o = n.observers;
      if (o && o.length) {
        if (!n.observerSlots || n.observerSlots.length !== o.length) {
          M(n);
        }
        const r = o.pop();
        const l = n.observerSlots.pop();
        if (i < o.length) {
          r.sourceSlots[l] = i;
          o[i] = r;
          n.observerSlots[i] = l;
        }
      }
      if (!t) {
        if (o) {
          o.length;
        }
      }
    }
  }
  if (e.tOwned) {
    for (s = e.tOwned.length - 1; s >= 0; s--) {
      y(e.tOwned[s]);
    }
    delete e.tOwned;
  }
  if (e.owned) {
    for (s = e.owned.length - 1; s >= 0; s--) {
      y(e.owned[s]);
    }
    e.owned = null;
  }
  if (e.cleanups) {
    for (s = e.cleanups.length - 1; s >= 0; s--) {
      e.cleanups[s]();
    }
    e.cleanups = null;
  }
  e.state = 0;
}
function ee(e) {
  if (e instanceof Error) {
    return e;
  } else {
    return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
  }
}
function F(e, t = h) {
  throw ee(e);
}
function te(e, t) {
  return O(() => e(t || {}));
}
function se(e, t, s) {
  const n = s.length;
  let i = t.length;
  let o = n;
  let r = 0;
  let l = 0;
  const f = t[i - 1].nextSibling;
  let c = null;
  while (r < i || l < o) {
    if (t[r] === s[l]) {
      r++;
      l++;
      continue;
    }
    while (t[i - 1] === s[o - 1]) {
      i--;
      o--;
    }
    if (i === r) {
      const p = o < n ? l ? s[l - 1].nextSibling : s[o - l] : f;
      while (l < o) {
        e.insertBefore(s[l++], p);
      }
    } else if (o === l) {
      while (r < i) {
        if (!c || !c.has(t[r])) {
          t[r].remove();
        }
        r++;
      }
    } else if (t[r] === s[o - 1] && s[l] === t[i - 1]) {
      const p = t[--i].nextSibling;
      e.insertBefore(s[l++], t[r++].nextSibling);
      e.insertBefore(s[--o], p);
      t[i] = s[o];
    } else {
      if (!c) {
        c = new Map();
        let g = l;
        while (g < o) {
          c.set(s[g], g++);
        }
      }
      const p = c.get(t[r]);
      if (p != null) {
        if (l < p && p < o) {
          let g = r;
          let T = 1;
          let D;
          while (++g < i && g < o && (D = c.get(t[g])) != null && D === p + T) {
            T++;
          }
          if (T > p - l) {
            const G = t[r];
            while (l < p) {
              e.insertBefore(s[l++], G);
            }
          } else {
            e.replaceChild(s[l++], t[r++]);
          }
        } else {
          r++;
        }
      } else {
        t[r++].remove();
      }
    }
  }
}
const L = "_$DX_DELEGATE";
function ne(e, t, s, n = {}) {
  let i;
  W((o) => {
    i = o;
    if (t === document) {
      e();
    } else {
      m(t, e(), t.firstChild ? null : undefined, s);
    }
  }, n.owner);
  return () => {
    i();
    t.textContent = "";
  };
}
function le(e, t, s, n) {
  let i;
  const o = () => {
    const l = document.createElement("template");
    l.innerHTML = e;
    return l.content.firstChild;
  };
  const r = () => (i ||= o()).cloneNode(true);
  r.cloneNode = r;
  return r;
}
function ie(e, t = window.document) {
  const s = t[L] ||= new Set();
  for (let n = 0, i = e.length; n < i; n++) {
    const o = e[n];
    if (!s.has(o)) {
      s.add(o);
      t.addEventListener(o, oe);
    }
  }
}
function m(e, t, s, n) {
  if (s !== undefined && !n) {
    n = [];
  }
  if (typeof t != "function") {
    return C(e, t, n, s);
  }
  $((i) => C(e, t(), i, s), n);
}
function oe(e) {
  let t = e.target;
  const s = `$$${e.type}`;
  const n = e.target;
  const i = e.currentTarget;
  const o = (f) => Object.defineProperty(e, "target", {
    configurable: true,
    value: f
  });
  const r = () => {
    const f = t[s];
    if (f && !t.disabled) {
      const c = t[`${s}Data`];
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
      o(t.host);
    }
    return true;
  };
  const l = () => {
    while (r() && (t = t._$host || t.parentNode || t.host));
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
      o(f[0]);
      for (let c = 0; c < f.length - 2 && (t = f[c], !!r()); c++) {
        if (t._$host) {
          t = t._$host;
          l();
          break;
        }
        if (t.parentNode === i) {
          break;
        }
      }
    } else {
      l();
    }
    o(n);
  } finally {
    e = t = null;
  }
}
function C(e, t, s, n, i) {
  while (typeof s == "function") {
    s = s();
  }
  if (t === s) {
    return s;
  }
  const o = typeof t;
  const r = n !== undefined;
  e = r && s[0] && s[0].parentNode || e;
  if (o === "string" || o === "number") {
    if (o === "number" && (t = t.toString(), t === s)) {
      return s;
    }
    if (r) {
      let l = s[0];
      if (l && l.nodeType === 3) {
        if (l.data !== t) {
          l.data = t;
        }
      } else {
        l = document.createTextNode(t);
      }
      s = b(e, s, n, l);
    } else if (s !== "" && typeof s == "string") {
      s = e.firstChild.data = t;
    } else {
      s = e.textContent = t;
    }
  } else if (t == null || o === "boolean") {
    s = b(e, s, n);
  } else {
    if (o === "function") {
      $(() => {
        let l = t();
        while (typeof l == "function") {
          l = l();
        }
        s = C(e, l, s, n);
      });
      return () => s;
    }
    if (Array.isArray(t)) {
      const l = [];
      const f = s && Array.isArray(s);
      if (N(l, t, s, i)) {
        $(() => s = C(e, l, s, n, true));
        return () => s;
      }
      if (l.length === 0) {
        s = b(e, s, n);
        if (r) {
          return s;
        }
      } else if (f) {
        if (s.length === 0) {
          U(e, l, n);
        } else {
          se(e, s, l);
        }
      } else {
        if (s) {
          b(e);
        }
        U(e, l);
      }
      s = l;
    } else if (t.nodeType) {
      if (Array.isArray(s)) {
        if (r) {
          return s = b(e, s, n, t);
        }
        b(e, s, null, t);
      } else if (s == null || s === "" || !e.firstChild) {
        e.appendChild(t);
      } else {
        e.replaceChild(t, e.firstChild);
      }
      s = t;
    }
  }
  return s;
}
function N(e, t, s, n) {
  let i = false;
  for (let o = 0, r = t.length; o < r; o++) {
    let l = t[o];
    const f = s && s[e.length];
    let c;
    if (l != null && l !== true && l !== false) {
      if ((c = typeof l) == "object" && l.nodeType) {
        e.push(l);
      } else if (Array.isArray(l)) {
        i = N(e, l, f) || i;
      } else if (c === "function") {
        if (n) {
          while (typeof l == "function") {
            l = l();
          }
          i = N(e, Array.isArray(l) ? l : [l], Array.isArray(f) ? f : [f]) || i;
        } else {
          e.push(l);
          i = true;
        }
      } else {
        const p = String(l);
        if (f && f.nodeType === 3 && f.data === p) {
          e.push(f);
        } else {
          e.push(document.createTextNode(p));
        }
      }
    }
  }
  return i;
}
function U(e, t, s = null) {
  for (let n = 0, i = t.length; n < i; n++) {
    e.insertBefore(t[n], s);
  }
}
function b(e, t, s, n) {
  if (s === undefined) {
    return e.textContent = "";
  }
  const i = n || document.createTextNode("");
  if (t.length) {
    let o = false;
    for (let r = t.length - 1; r >= 0; r--) {
      const l = t[r];
      if (i !== l) {
        const f = l.parentNode === e;
        if (!o && !r) {
          if (f) {
            e.replaceChild(i, l);
          } else {
            e.insertBefore(i, s);
          }
        } else if (f) {
          l.remove();
        }
      } else {
        o = true;
      }
    }
  } else {
    e.insertBefore(i, s);
  }
  return [i];
}
var re = le("<div><p></p><p></p><button>inc");
const fe = (e) => {
  console.log("wrapped");
  return e;
};
const I = (e) => fe(() => e());
function ue() {
  const [e, t] = J(1);
  const s = I(() => e() * 2);
  return (() => {
    const n = re();
    const i = n.firstChild;
    const o = i.nextSibling;
    const r = o.nextSibling;
    m(i, s);
    m(o, () => I(() => e())());
    r.$$click = () => t(e() + 1);
    return n;
  })();
}
ie(["click"]);
ne(() => te(ue, {}), document.body);