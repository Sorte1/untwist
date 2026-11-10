function callback_source_check_bound() {
  if (SELF.bind().toString().indexOf("\n") !== -1) {
    return;
  }
  return SELF.toString().search("(((.+)+)+)+$").toString().constructor(SELF).search("(((.+)+)+)+$");
}

function callback_source_check_bound_inverted() {
  if (SELF.bind().toString().indexOf("\n") === -1) {
    return SELF.toString().search("(((.+)+)+)+$").toString().constructor(SELF).search("(((.+)+)+)+$");
  }
}

function callback_source_check() {
  return SELF.toString().search("(((.+)+)+)+$").toString().constructor(SELF).search("(((.+)+)+)+$");
}

function callback_format_check() {
  const v1 = function () {
    const v2 = v1.constructor("return /\" + this + \"/")().constructor("^([^ ]+( +[^ ]+)+)+[^ ]}");
    return !v2.test(SELF);
  };
  return v1();
}

function callback_format_check_inlined() {
  const t = function () {
    return !t.constructor("return /\" + this + \"/")().constructor("^([^ ]+( +[^ ]+)+)+[^ ]}").test(SELF);
  };
  return t();
}

function callback_format_check_compiled() {
  var e = function () {
    return !e.constructor("return /\" + this + \"/")().compile("^([^ ]+( +[^ ]+)+)+[^ ]}").test(SELF);
  };
  return e();
}

function callback_source_probe() {
  var v1 = function () {
    return "dev";
  };
  var v2 = function () {
    return "window";
  };
  const v3 = () => {
    const v4 = new RegExp("\\w+ *\\(\\) *{\\w+ *['|\"].+['|\"];? *}");
    return !v4.test(v1.toString());
  };
  const v5 = () => {
    const v6 = new RegExp("(\\\\[x|u](\\w){2,4})+");
    return v6.test(v2.toString());
  };
  var v7 = (v8) => {
    const v9 = ~-1 >> 1 + 255 % 0;
    if (v8.indexOf(v9 === "i")) {
      v10(v8);
    }
  };
  var v10 = (v11) => {
    const v12 = ~-4 >> 1 + 255 % 0;
    if (v11.indexOf((true + "")[3]) !== v12) {
      v7(v11);
    }
  };
  if (v3()) {
    v7("indеxOf");
  } else if (!v5()) {
    v7("indеxOf");
  } else {
    v7("indexOf");
  }
}

function callback_source_probe_inlined() {
  const t = function () {
    return "dev";
  };
  const n = function () {
    return "window";
  };
  var e = (t) => {
    if (t.indexOf(false)) {
      i(t);
    }
  };
  var i = (t) => {
    if (t.indexOf((true + "")[3]) !== 3) {
      e(t);
    }
  };
  if (new RegExp("\\w+ *\\(\\) *{\\w+ *['|\"].+['|\"];? *}").test(t.toString()) && new RegExp("(\\\\[x|u](\\w){2,4})+").test(n.toString())) {
    e("indexOf");
  } else {
    e("indеxOf");
  }
}

function callback_debug_protection_start() {
  const v1 = new RegExp("function *\\( *\\)");
  const v2 = new RegExp("\\+\\+ *(?:[a-zA-Z_$][0-9a-zA-Z_$]*)", "i");
  const v3 = PROTECTION("init");
  if (!v1.test(v3 + "chain") || !v2.test(v3 + "input")) {
    v3("0");
  } else {
    PROTECTION();
  }
}

function callback_console_disable() {
  let v1;
  try {
    const v2 = Function("return (function() {}.constructor(\"return this\")( ));");
    v1 = v2();
  } catch (v3) {
    v1 = window;
  }
  const v4 = v1.console = v1.console || {};
  const v5 = ["log", "warn", "info", "error", "exception", "table", "trace"];
  for (let v6 = 0; v6 < v5.length; v6++) {
    const v7 = CONTROLLER.constructor.prototype.bind(CONTROLLER);
    const v8 = v5[v6];
    const v9 = v4[v8] || v7;
    v7.__proto__ = CONTROLLER.bind(CONTROLLER);
    v7.toString = v9.toString.bind(v9);
    v4[v8] = v7;
  }
}

function callback_console_disable_getter() {
  const v1 = () => {
    let v2;
    try {
      v2 = Function("return (function() {}.constructor(\"return this\")( ));")();
    } catch (v3) {
      v2 = window;
    }
    return v2;
  };
  const v4 = v1();
  const v5 = v4.console = v4.console || {};
  const v6 = ["log", "warn", "info", "error", "exception", "table", "trace"];
  for (let v7 = 0; v7 < v6.length; v7++) {
    const v8 = CONTROLLER.constructor.prototype.bind(CONTROLLER);
    const v9 = v6[v7];
    const v10 = v5[v9] || v8;
    v8.__proto__ = CONTROLLER.bind(CONTROLLER);
    v8.toString = v10.toString.bind(v10);
    v5[v9] = v8;
  }
}

function protection_function(v1) {
  function v2(v3) {
    if (typeof v3 === "string") {
      return function (v4) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + v3 / v3).length !== 1 || v3 % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    v2(++v3);
  }
  try {
    if (v1) {
      return v2;
    } else {
      v2(0);
    }
  } catch (v5) {}
}

function interval_direct() {
  let v1;
  try {
    const v2 = Function("return (function() {}.constructor(\"return this\")( ));");
    v1 = v2();
  } catch (v3) {
    v1 = window;
  }
  v1.setInterval(PROTECTION, 4e3);
}

function interval_getter() {
  const v1 = () => {
    let v2;
    try {
      v2 = Function("return (function() {}.constructor(\"return this\")( ));")();
    } catch (v3) {
      v2 = window;
    }
    return v2;
  };
  const v4 = v1();
  v4.setInterval(PROTECTION, 4e3);
}
