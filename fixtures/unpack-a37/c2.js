 (function(modules) { 
 	var c = {};
 	function r(moduleId) {
 		if(c[moduleId]) {
 			return c[moduleId].exports;
 		}
 		var module = c[moduleId] = {
 			i: moduleId,
 			l: false,
 			exports: {}
 		};
 		modules[moduleId].call(module.exports, module, module.exports, r);
 		module.l = true;
 		return module.exports;
 	}
 	r.m = modules;
 	r.c = c;
 	r.d = function(exports, name, getter) {
 		if(!r.o(exports, name)) {
 			Object.defineProperty(exports, name, { enumerable: true, get: getter });
 		}
 	};
 	r.r = function(exports) {
 		if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
 			Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
 		}
 		Object.defineProperty(exports, '__esModule', { value: true });
 	};
 	r.t = function(value, mode) {
 		if(mode & 1) value = r(value);
 		if(mode & 8) return value;
 		if((mode & 4) && typeof value === 'object' && value && value.__esModule) return value;
 		var ns = Object.create(null);
 		r.r(ns);
 		Object.defineProperty(ns, 'default', { enumerable: true, value: value });
 		if(mode & 2 && typeof value != 'string') for(var key in value) r.d(ns, key, function(key) { return value[key]; }.bind(null, key));
 		return ns;
 	};
 	r.n = function(module) {
 		var getter = module && module.__esModule ?
 			function getDefault() { return module['default']; } :
 			function getModuleExports() { return module; };
 		r.d(getter, 'a', getter);
 		return getter;
 	};
 	r.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
 	r.p = ""; r.zz = function () { return eval("typeof __webpack_module_paths__ + typeof r") };
 	return r(r.s = 0);
 })
([
  function (e, t, n) { var y = n(1); console.log(n.zz(), y.b); },
  function (e, t, n) { "use strict"; n.r(t); n.d(t, "b", function(){ return x }); var x = 2; },
]);
