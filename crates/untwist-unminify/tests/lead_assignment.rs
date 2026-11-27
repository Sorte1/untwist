use untwist_testing::assert_transform;
use untwist_unminify::{lead_assignment, sequence};

#[test]
fn a_leading_assignment_becomes_its_own_statement() {
    assert_transform!(
        "function f(o) { var c; (c = o.k || {}).n++; (c = o.m).run(); return (c = o.j) ? c.a : 0; }",
        [lead_assignment::boxed, sequence::boxed],
        @"
    function f(o) {
      var c;
      c = o.k || {};
      c.n++;
      c = o.m;
      c.run();
      c = o.j;
      return c ? c.a : 0;
    }
    "
    );
}

#[test]
fn logical_and_member_assignment_heads_are_split() {
    assert_transform!(
        "function f(s) { var m, g; (m = /a/.exec(s)) && use(m); (g = s.o).hit = g.hit + 1; throw (m = s.e).msg; }",
        [lead_assignment::boxed, sequence::boxed],
        @"
    function f(s) {
      var m, g;
      m = /a/.exec(s);
      m && use(m);
      g = s.o;
      g.hit = g.hit + 1;
      m = s.e;
      throw m.msg;
    }
    "
    );
}

#[test]
fn a_first_argument_moves_ahead_of_a_stable_local_callee() {
    assert_transform!(
        "function f(o) { function rec(x) { return x; } var y, z; rec(y = o.v); g(z = o.w); return y + z; }",
        [lead_assignment::boxed, sequence::boxed],
        @"
    function f(o) {
      function rec(x) {
        return x;
      }
      var y, z;
      y = o.v;
      rec(y);
      g(z = o.w);
      return y + z;
    }
    "
    );
}

#[test]
fn heads_that_are_not_plain_bindings_stay() {
    assert_transform!(
        "function f(o) { const k = 1; (u = o).p++; (o.q = 1).toString(); delete (o = 1); if (o = 2) g(); } function h() { (h = 1).x; }",
        [lead_assignment::boxed, sequence::boxed],
        @"
    function f(o) {
      const k = 1;
      (u = o).p++;
      (o.q = 1).toString();
      delete (o = 1);
      if (o = 2) g();
    }
    function h() {
      (h = 1).x;
    }
    "
    );
}

#[test]
fn a_scope_hazard_keeps_every_head() {
    assert_transform!(
        "function f(o) { var c; eval(''); (c = o).p++; }",
        [lead_assignment::boxed, sequence::boxed],
        @r#"
    function f(o) {
      var c;
      eval("");
      (c = o).p++;
    }
    "#
    );
}
