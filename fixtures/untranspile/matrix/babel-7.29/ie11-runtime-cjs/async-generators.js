(function () {
  'use strict';

  var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");
  var _regenerator = _interopRequireDefault(require("@babel/runtime/regenerator"));
  var _asyncToGenerator2 = _interopRequireDefault(require("@babel/runtime/helpers/asyncToGenerator"));
  var _awaitAsyncGenerator2 = _interopRequireDefault(require("@babel/runtime/helpers/awaitAsyncGenerator"));
  var _wrapAsyncGenerator2 = _interopRequireDefault(require("@babel/runtime/helpers/wrapAsyncGenerator"));
  function _asyncIterator(r) { var n, t, o, e = 2; for ("undefined" != typeof Symbol && (t = Symbol.asyncIterator, o = Symbol.iterator); e--;) { if (t && null != (n = r[t])) return n.call(r); if (o && null != (n = r[o])) return new AsyncFromSyncIterator(n.call(r)); t = "@@asyncIterator", o = "@@iterator"; } throw new TypeError("Object is not async iterable"); }
  function AsyncFromSyncIterator(r) { function AsyncFromSyncIteratorContinuation(r) { if (Object(r) !== r) return Promise.reject(new TypeError(r + " is not an object.")); var n = r.done; return Promise.resolve(r.value).then(function (r) { return { value: r, done: n }; }); } return AsyncFromSyncIterator = function AsyncFromSyncIterator(r) { this.s = r, this.n = r.next; }, AsyncFromSyncIterator.prototype = { s: null, n: null, next: function next() { return AsyncFromSyncIteratorContinuation(this.n.apply(this.s, arguments)); }, return: function _return(r) { var n = this.s.return; return void 0 === n ? Promise.resolve({ value: r, done: true }) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); }, throw: function _throw(r) { var n = this.s.throw; return void 0 === n ? Promise.reject(r) : AsyncFromSyncIteratorContinuation(n.apply(this.s, arguments)); } }, new AsyncFromSyncIterator(r); }
  function ticker(_x) {
    return _ticker.apply(this, arguments);
  }
  function _ticker() {
    _ticker = (0, _wrapAsyncGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee(n) {
      var i;
      return _regenerator.default.wrap(function (_context) {
        while (1) switch (_context.prev = _context.next) {
          case 0:
            i = 0;
          case 1:
            if (!(i < n)) {
              _context.next = 4;
              break;
            }
            _context.next = 2;
            return (0, _awaitAsyncGenerator2.default)(null);
          case 2:
            _context.next = 3;
            return i * 10;
          case 3:
            i++;
            _context.next = 1;
            break;
          case 4:
          case "end":
            return _context.stop();
        }
      }, _callee);
    }));
    return _ticker.apply(this, arguments);
  }
  function relay(_x2) {
    return _relay.apply(this, arguments);
  }
  function _relay() {
    _relay = (0, _wrapAsyncGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee2(src) {
      var _iteratorAbruptCompletion, _didIteratorError, _iteratorError, _iterator, _step, v, _t;
      return _regenerator.default.wrap(function (_context2) {
        while (1) switch (_context2.prev = _context2.next) {
          case 0:
            _iteratorAbruptCompletion = false;
            _didIteratorError = false;
            _context2.prev = 1;
            _iterator = _asyncIterator(src);
          case 2:
            _context2.next = 3;
            return (0, _awaitAsyncGenerator2.default)(_iterator.next());
          case 3:
            if (!(_iteratorAbruptCompletion = !(_step = _context2.sent).done)) {
              _context2.next = 5;
              break;
            }
            v = _step.value;
            _context2.next = 4;
            return v + 1;
          case 4:
            _iteratorAbruptCompletion = false;
            _context2.next = 2;
            break;
          case 5:
            _context2.next = 7;
            break;
          case 6:
            _context2.prev = 6;
            _t = _context2["catch"](1);
            _didIteratorError = true;
            _iteratorError = _t;
          case 7:
            _context2.prev = 7;
            _context2.prev = 8;
            if (!(_iteratorAbruptCompletion && _iterator.return != null)) {
              _context2.next = 9;
              break;
            }
            _context2.next = 9;
            return (0, _awaitAsyncGenerator2.default)(_iterator.return());
          case 9:
            _context2.prev = 9;
            if (!_didIteratorError) {
              _context2.next = 10;
              break;
            }
            throw _iteratorError;
          case 10:
            return _context2.finish(9);
          case 11:
            return _context2.finish(7);
          case 12:
          case "end":
            return _context2.stop();
        }
      }, _callee2, null, [[1, 6, 7, 12], [8,, 9, 11]]);
    }));
    return _relay.apply(this, arguments);
  }
  function main() {
    return _main.apply(this, arguments);
  }
  function _main() {
    _main = (0, _asyncToGenerator2.default)(/*#__PURE__*/_regenerator.default.mark(function _callee3() {
      var seen, _iteratorAbruptCompletion2, _didIteratorError2, _iteratorError2, _iterator2, _step2, v, it, mixed, _iteratorAbruptCompletion3, _didIteratorError3, _iteratorError3, _iterator3, _step3, _v, _t2, _t3, _t4, _t5, _t6, _t7, _t8;
      return _regenerator.default.wrap(function (_context3) {
        while (1) switch (_context3.prev = _context3.next) {
          case 0:
            seen = [];
            _iteratorAbruptCompletion2 = false;
            _didIteratorError2 = false;
            _context3.prev = 1;
            _iterator2 = _asyncIterator(relay(ticker(3)));
          case 2:
            _context3.next = 3;
            return _iterator2.next();
          case 3:
            if (!(_iteratorAbruptCompletion2 = !(_step2 = _context3.sent).done)) {
              _context3.next = 5;
              break;
            }
            v = _step2.value;
            seen.push(v);
          case 4:
            _iteratorAbruptCompletion2 = false;
            _context3.next = 2;
            break;
          case 5:
            _context3.next = 7;
            break;
          case 6:
            _context3.prev = 6;
            _t2 = _context3["catch"](1);
            _didIteratorError2 = true;
            _iteratorError2 = _t2;
          case 7:
            _context3.prev = 7;
            _context3.prev = 8;
            if (!(_iteratorAbruptCompletion2 && _iterator2.return != null)) {
              _context3.next = 9;
              break;
            }
            _context3.next = 9;
            return _iterator2.return();
          case 9:
            _context3.prev = 9;
            if (!_didIteratorError2) {
              _context3.next = 10;
              break;
            }
            throw _iteratorError2;
          case 10:
            return _context3.finish(9);
          case 11:
            return _context3.finish(7);
          case 12:
            console.log(seen.join(","));
            it = ticker(5);
            _t3 = console;
            _t4 = JSON;
            _context3.next = 13;
            return it.next();
          case 13:
            _t5 = _context3.sent;
            _context3.next = 14;
            return it.return("x");
          case 14:
            _t6 = _context3.sent;
            _context3.next = 15;
            return it.next();
          case 15:
            _t7 = _context3.sent;
            _t3.log.call(_t3, _t4.stringify.call(_t4, [_t5, _t6, _t7]));
            mixed = [];
            _iteratorAbruptCompletion3 = false;
            _didIteratorError3 = false;
            _context3.prev = 16;
            _iterator3 = _asyncIterator([Promise.resolve("a"), "b", later("c")]);
          case 17:
            _context3.next = 18;
            return _iterator3.next();
          case 18:
            if (!(_iteratorAbruptCompletion3 = !(_step3 = _context3.sent).done)) {
              _context3.next = 20;
              break;
            }
            _v = _step3.value;
            mixed.push(_v);
          case 19:
            _iteratorAbruptCompletion3 = false;
            _context3.next = 17;
            break;
          case 20:
            _context3.next = 22;
            break;
          case 21:
            _context3.prev = 21;
            _t8 = _context3["catch"](16);
            _didIteratorError3 = true;
            _iteratorError3 = _t8;
          case 22:
            _context3.prev = 22;
            _context3.prev = 23;
            if (!(_iteratorAbruptCompletion3 && _iterator3.return != null)) {
              _context3.next = 24;
              break;
            }
            _context3.next = 24;
            return _iterator3.return();
          case 24:
            _context3.prev = 24;
            if (!_didIteratorError3) {
              _context3.next = 25;
              break;
            }
            throw _iteratorError3;
          case 25:
            return _context3.finish(24);
          case 26:
            return _context3.finish(22);
          case 27:
            console.log(mixed.join(""));
          case 28:
          case "end":
            return _context3.stop();
        }
      }, _callee3, null, [[1, 6, 7, 12], [8,, 9, 11], [16, 21, 22, 27], [23,, 24, 26]]);
    }));
    return _main.apply(this, arguments);
  }
  function later(v) {
    return new Promise(function (resolve) {
      return setTimeout(function () {
        return resolve(v);
      }, 0);
    });
  }
  main();

})();
