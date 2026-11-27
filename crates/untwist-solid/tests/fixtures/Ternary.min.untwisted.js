(function () {
  "use strict";
  const e = { equals: (e, t) => e === t };
  const t = A;
  const n = 1;
  const o = 2;
  const r = {
    owned: null,
    cleanups: null,
    context: null,
    owner: null
  };
  var s = null;
  const l = null;
  let u = null;
  let i = null;
  let c = null;
  let f = 0;
  function a(e, t) {
    const n = u;
    const o = s;
    const l = e.length === 0;
    const i = t === undefined ? o : t;
    const c = l ? r : {
      owned: null,
      cleanups: null,
      context: i ? i.context : null,
      owner: i
    };
    const f = l ? e : () => e(function (e) {
      return () => p(() => C(e));
    }(c));
    s = c;
    u = null;
    try {
      return m(f, true);
    } finally {
      u = n;
      s = o;
    }
  }
  function d(t, n) {
    const o = {
      value: t,
      observers: null,
      observerSlots: null,
      comparator: (n = n ? Object.assign({}, e, n) : e).equals || undefined
    };
    return [b.bind(o), (e) => {
      if (typeof e == "function") {
        e = e(o.value);
      }
      return v(o, e);
    }];
  }
  function h(e, t, o) {
    y(w(e, t, false, n));
  }
  function p(e) {
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
  function g(e) {
    e.observerSlots = e.observers.map((t, n) => {
      const o = t.sources;
      if (!o) {
        return -1;
      }
      for (let r = o.length - 1; r >= 0; r--) {
        if (o[r] === e && t.sourceSlots && t.sourceSlots[r] === n) {
          return r;
        }
      }
      return o.lastIndexOf(e);
    });
  }
  function b() {
    if (this.sources && this.state) {
      if (this.state === n) {
        y(this);
      } else {
        const e = i;
        i = null;
        m(() => x(this), false);
        i = e;
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
          const e = [this];
          const n = [t];
          u.sources = e;
          u.sourceSlots = n;
        }
        if (e) {
          if (!this.observerSlots || this.observerSlots.length !== e.length) {
            g(this);
          }
          e.push(u);
          this.observerSlots.push(u.sources.length - 1);
        } else {
          const e = [u];
          const t = [u.sources.length - 1];
          this.observers = e;
          this.observerSlots = t;
        }
      }
    }
    return this.value;
  }
  function v(e, t, o) {
    const r = e.value;
    if (!e.comparator || !e.comparator(r, t)) {
      e.value = t;
      if (e.observers && e.observers.length) {
        m(() => {
          for (let t = 0; t < e.observers.length; t += 1) {
            const o = e.observers[t];
            const r = l && l.running;
            if (r) {
              l.disposed.has(o);
            }
            if (!(r ? o.tState : o.state)) {
              if (o.pure) {
                i.push(o);
              } else {
                c.push(o);
              }
              if (o.observers) {
                $(o);
              }
            }
            if (!r) {
              o.state = n;
            }
          }
          if (i.length > 1e6) {
            i = [];
            throw new Error();
          }
        }, false);
      }
    }
    return t;
  }
  function y(e) {
    if (!e.fn) {
      return;
    }
    C(e, true);
    const t = f;
    (function (e, t, o) {
      let r;
      const l = s;
      const i = u;
      u = s = e;
      try {
        r = e.fn(t);
      } catch (t) {
        if (e.pure) {
          e.state = n;
          if (e.owned) {
            e.owned.forEach((e) => C(e));
          }
          e.owned = null;
        }
        e.updatedAt = o + 1;
        return T(t);
      } finally {
        u = i;
        s = l;
      }
      if (!e.updatedAt || e.updatedAt <= o) {
        if (e.updatedAt != null && "observers" in e) {
          v(e, r);
        } else {
          e.value = r;
        }
        e.updatedAt = o;
      }
    })(e, e.value, t);
  }
  function w(e, t, o, l = n, u) {
    const i = {
      fn: e,
      state: l,
      updatedAt: null,
      owned: null,
      sources: null,
      sourceSlots: null,
      cleanups: null,
      value: t,
      owner: s,
      context: s ? s.context : null,
      pure: o
    };
    if (s !== null) {
      if (s !== r) {
        if (s.owned) {
          s.owned.push(i);
        } else {
          s.owned = [i];
        }
      }
    }
    return i;
  }
  function S(e) {
    if (e.state === 0) {
      return;
    }
    if (e.state === o) {
      return x(e);
    }
    if (e.suspense && p(e.suspense.inFallback)) {
      return e.suspense.effects.push(e);
    }
    const t = [e];
    while ((e = e.owner) && (!e.updatedAt || e.updatedAt < f)) {
      if (e.state) {
        t.push(e);
      }
    }
    for (let r = t.length - 1; r >= 0; r--) {
      if ((e = t[r]).state === n) {
        y(e);
      } else if (e.state === o) {
        const n = i;
        i = null;
        m(() => x(e, t[0]), false);
        i = n;
      }
    }
  }
  function m(e, n) {
    if (i) {
      return e();
    }
    let o = false;
    if (!n) {
      i = [];
    }
    if (c) {
      o = true;
    } else {
      c = [];
    }
    f++;
    try {
      const n = e();
      (function (e) {
        if (i) {
          A(i);
          i = null;
        }
        if (e) {
          return;
        }
        const n = c;
        c = null;
        if (n.length) {
          m(() => t(n), false);
        }
      })(o);
      return n;
    } catch (e) {
      if (!o) {
        c = null;
      }
      i = null;
      T(e);
    }
  }
  function A(e) {
    for (let t = 0; t < e.length; t++) {
      S(e[t]);
    }
  }
  function x(e, t) {
    e.state = 0;
    for (let r = 0; r < e.sources.length; r += 1) {
      const s = e.sources[r];
      if (s.sources) {
        const e = s.state;
        if (e === n) {
          if (s !== t && (!s.updatedAt || s.updatedAt < f)) {
            S(s);
          }
        } else if (e === o) {
          x(s, t);
        }
      }
    }
  }
  function $(e) {
    for (let t = 0; t < e.observers.length; t += 1) {
      const n = e.observers[t];
      if (!n.state) {
        n.state = o;
        if (n.pure) {
          i.push(n);
        } else {
          c.push(n);
        }
        if (n.observers) {
          $(n);
        }
      }
    }
  }
  function C(e, t) {
    let n;
    if (e.sources) {
      while (e.sources.length) {
        const n = e.sources.pop();
        const o = e.sourceSlots.pop();
        const r = n.observers;
        if (r && r.length) {
          if (!n.observerSlots || n.observerSlots.length !== r.length) {
            g(n);
          }
          const e = r.pop();
          const t = n.observerSlots.pop();
          if (o < r.length) {
            e.sourceSlots[t] = o;
            r[o] = e;
            n.observerSlots[o] = t;
          }
        }
        if (!t) {
          if (r) {
            r.length;
          }
        }
      }
    }
    if (e.tOwned) {
      for (n = e.tOwned.length - 1; n >= 0; n--) {
        C(e.tOwned[n]);
      }
      delete e.tOwned;
    }
    if (e.owned) {
      for (n = e.owned.length - 1; n >= 0; n--) {
        C(e.owned[n]);
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
  function T(e, t = s) {
    const n = function (e) {
      if (e instanceof Error) {
        return e;
      } else {
        return new Error(typeof e == "string" ? e : "Unknown error", { cause: e });
      }
    }(e);
    throw n;
  }
  const E = (t) => function (t, n, o) {
    o = o ? Object.assign({}, e, o) : e;
    const r = w(t, n, true, 0);
    r.observers = null;
    r.observerSlots = null;
    r.comparator = o.equals || undefined;
    y(r);
    return b.bind(r);
  }(() => t());
  const N = "_$DX_DELEGATE";
  function O(e, t, n, o) {
    let r;
    const s = () => (r ||= (() => {
      const t = document.createElement("template");
      t.innerHTML = e;
      return t.content.firstChild;
    })()).cloneNode(true);
    s.cloneNode = s;
    return s;
  }
  function B(e, t, n, o) {
    if (n !== undefined && !o) {
      o = [];
    }
    if (typeof t != "function") {
      return _(e, t, o, n);
    }
    h((o) => _(e, t(), o, n), o);
  }
  function k(e) {
    let t = e.target;
    const n = `$$${e.type}`;
    const o = e.target;
    const r = e.currentTarget;
    const s = (t) => Object.defineProperty(e, "target", {
      configurable: true,
      value: t
    });
    const l = () => {
      const o = t[n];
      if (o && !t.disabled) {
        const r = t[`${n}Data`];
        if (r !== undefined) {
          o.call(t, r, e);
        } else {
          o.call(t, e);
        }
        if (e.cancelBubble) {
          return;
        }
      }
      if (t.host && typeof t.host != "string" && !t.host._$host && t.contains(e.target)) {
        s(t.host);
      }
      return true;
    };
    const u = () => {
      while (l() && (t = t._$host || t.parentNode || t.host));
    };
    Object.defineProperty(e, "currentTarget", {
      configurable: true,
      get: () => t || document
    });
    try {
      if (e.composedPath) {
        const n = e.composedPath();
        s(n[0]);
        for (let e = 0; e < n.length - 2 && (t = n[e], l()); e++) {
          if (t._$host) {
            t = t._$host;
            u();
            break;
          }
          if (t.parentNode === r) {
            break;
          }
        }
      } else {
        u();
      }
      s(o);
    } finally {
      e = t = null;
    }
  }
  function _(e, t, n, o, r) {
    while (typeof n == "function") {
      n = n();
    }
    if (t === n) {
      return n;
    }
    const s = typeof t;
    const l = o !== undefined;
    e = l && n[0] && n[0].parentNode || e;
    if (s === "string" || s === "number") {
      if (s === "number" && (t = t.toString()) === n) {
        return n;
      }
      if (l) {
        let r = n[0];
        if (r && r.nodeType === 3) {
          if (r.data !== t) {
            r.data = t;
          }
        } else {
          r = document.createTextNode(t);
        }
        n = q(e, n, o, r);
      } else {
        n = n !== "" && typeof n == "string" ? e.firstChild.data = t : e.textContent = t;
      }
    } else if (t == null || s === "boolean") {
      n = q(e, n, o);
    } else {
      if (s === "function") {
        h(() => {
          let r = t();
          while (typeof r == "function") {
            r = r();
          }
          n = _(e, r, n, o);
        });
        return () => n;
      }
      if (Array.isArray(t)) {
        const s = [];
        const u = n && Array.isArray(n);
        if (j(s, t, n, r)) {
          h(() => n = _(e, s, n, o, true));
          return () => n;
        }
        if (s.length === 0) {
          n = q(e, n, o);
          if (l) {
            return n;
          }
        } else if (u) {
          if (n.length === 0) {
            P(e, s, o);
          } else {
            (function (e, t, n) {
              const o = n.length;
              let r = t.length;
              let s = o;
              let l = 0;
              let u = 0;
              const i = t[r - 1].nextSibling;
              let c = null;
              while (l < r || u < s) {
                if (t[l] !== n[u]) {
                  while (t[r - 1] === n[s - 1]) {
                    r--;
                    s--;
                  }
                  if (r === l) {
                    const t = s < o ? u ? n[u - 1].nextSibling : n[s - u] : i;
                    while (u < s) {
                      e.insertBefore(n[u++], t);
                    }
                  } else if (s === u) {
                    while (l < r) {
                      if (!c || !c.has(t[l])) {
                        t[l].remove();
                      }
                      l++;
                    }
                  } else if (t[l] === n[s - 1] && n[u] === t[r - 1]) {
                    const o = t[--r].nextSibling;
                    e.insertBefore(n[u++], t[l++].nextSibling);
                    e.insertBefore(n[--s], o);
                    t[r] = n[s];
                  } else {
                    if (!c) {
                      c = new Map();
                      let e = u;
                      while (e < s) {
                        c.set(n[e], e++);
                      }
                    }
                    const o = c.get(t[l]);
                    if (o != null) {
                      if (u < o && o < s) {
                        let i;
                        let f = l;
                        let a = 1;
                        while (++f < r && f < s && (i = c.get(t[f])) != null && i === o + a) {
                          a++;
                        }
                        if (a > o - u) {
                          const r = t[l];
                          while (u < o) {
                            e.insertBefore(n[u++], r);
                          }
                        } else {
                          e.replaceChild(n[u++], t[l++]);
                        }
                      } else {
                        l++;
                      }
                    } else {
                      t[l++].remove();
                    }
                  }
                } else {
                  l++;
                  u++;
                }
              }
            })(e, n, s);
          }
        } else {
          if (n) {
            q(e);
          }
          P(e, s);
        }
        n = s;
      } else if (t.nodeType) {
        if (Array.isArray(n)) {
          if (l) {
            return n = q(e, n, o, t);
          }
          q(e, n, null, t);
        } else if (n != null && n !== "" && e.firstChild) {
          e.replaceChild(t, e.firstChild);
        } else {
          e.appendChild(t);
        }
        n = t;
      }
    }
    return n;
  }
  function j(e, t, n, o) {
    let r = false;
    for (let s = 0, l = t.length; s < l; s++) {
      let l;
      let u = t[s];
      const i = n && n[e.length];
      if (u == null || u === true || u === false);
      else if ((l = typeof u) == "object" && u.nodeType) {
        e.push(u);
      } else if (Array.isArray(u)) {
        r = j(e, u, i) || r;
      } else if (l === "function") {
        if (o) {
          while (typeof u == "function") {
            u = u();
          }
          r = j(e, Array.isArray(u) ? u : [u], Array.isArray(i) ? i : [i]) || r;
        } else {
          e.push(u);
          r = true;
        }
      } else {
        const t = String(u);
        if (i && i.nodeType === 3 && i.data === t) {
          e.push(i);
        } else {
          e.push(document.createTextNode(t));
        }
      }
    }
    return r;
  }
  function P(e, t, n = null) {
    for (let o = 0, r = t.length; o < r; o++) {
      e.insertBefore(t[o], n);
    }
  }
  function q(e, t, n, o) {
    if (n === undefined) {
      return e.textContent = "";
    }
    const r = o || document.createTextNode("");
    if (t.length) {
      let o = false;
      for (let s = t.length - 1; s >= 0; s--) {
        const l = t[s];
        if (r !== l) {
          const t = l.parentNode === e;
          if (o || s) {
            if (t) {
              l.remove();
            }
          } else if (t) {
            e.replaceChild(r, l);
          } else {
            e.insertBefore(r, n);
          }
        } else {
          o = true;
        }
      }
    } else {
      e.insertBefore(r, n);
    }
    return [r];
  }
  var D = O("<div><button class=t>toggle</button><button class=n>inc");
  var L = O("<b>on");
  var M = O("<i>off");
  var z = O("<span>more than one");
  function F() {
    const [e, t] = d(false);
    const [n, o] = d(0);
    const l = D();
    const u = l.firstChild;
    const i = u.nextSibling;
    let r;
    B(l, (r = E(() => !!e()), () => r() ? L() : M()), u);
    let s;
    B(l, (s = E(() => n() > 1), () => s() && z()), u);
    B(l, () => n() > 0 ? "positive" : "zero", u);
    u.$$click = () => t(!e());
    i.$$click = () => o(n() + 1);
    return l;
  }
  (function (e, t = window.document) {
    const n = t[N] ||= new Set();
    for (let o = 0, r = e.length; o < r; o++) {
      const r = e[o];
      if (!n.has(r)) {
        n.add(r);
        t.addEventListener(r, k);
      }
    }
  })(["click"]);
  (function (e, t, n, o = {}) {
    let r;
    a((o) => {
      r = o;
      if (t === document) {
        e();
      } else {
        B(t, e(), t.firstChild ? null : undefined, n);
      }
    }, o.owner);
  })(() => {
    const e = F;
    const t = {};
    return p(() => e(t || {}));
  }, document.body);
})();