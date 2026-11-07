/******/ (function(modules) { // webpackBootstrap
/******/ 	// The module cache
/******/ 	var installedModules = {};
/******/
/******/ 	// The require function
/******/ 	function __webpack_require__(moduleId) {
/******/
/******/ 		// Check if module is in cache
/******/ 		if(installedModules[moduleId]) {
/******/ 			return installedModules[moduleId].exports;
/******/ 		}
/******/ 		// Create a new module (and put it into the cache)
/******/ 		var module = installedModules[moduleId] = {
/******/ 			i: moduleId,
/******/ 			l: false,
/******/ 			exports: {}
/******/ 		};
/******/
/******/ 		// Execute the module function
/******/ 		modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
/******/
/******/ 		// Flag the module as loaded
/******/ 		module.l = true;
/******/
/******/ 		// Return the exports of the module
/******/ 		return module.exports;
/******/ 	}
/******/
/******/
/******/ 	// expose the modules object (__webpack_modules__)
/******/ 	__webpack_require__.m = modules;
/******/
/******/ 	// expose the module cache
/******/ 	__webpack_require__.c = installedModules;
/******/
/******/ 	// define getter function for harmony exports
/******/ 	__webpack_require__.d = function(exports, name, getter) {
/******/ 		if(!__webpack_require__.o(exports, name)) {
/******/ 			Object.defineProperty(exports, name, { enumerable: true, get: getter });
/******/ 		}
/******/ 	};
/******/
/******/ 	// define __esModule on exports
/******/ 	__webpack_require__.r = function(exports) {
/******/ 		if(typeof Symbol !== 'undefined' && Symbol.toStringTag) {
/******/ 			Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
/******/ 		}
/******/ 		Object.defineProperty(exports, '__esModule', { value: true });
/******/ 	};
/******/
/******/ 	// create a fake namespace object
/******/ 	// mode & 1: value is a module id, require it
/******/ 	// mode & 2: merge all properties of value into the ns
/******/ 	// mode & 4: return value when already ns object
/******/ 	// mode & 8|1: behave like require
/******/ 	__webpack_require__.t = function(value, mode) {
/******/ 		if(mode & 1) value = __webpack_require__(value);
/******/ 		if(mode & 8) return value;
/******/ 		if((mode & 4) && typeof value === 'object' && value && value.__esModule) return value;
/******/ 		var ns = Object.create(null);
/******/ 		__webpack_require__.r(ns);
/******/ 		Object.defineProperty(ns, 'default', { enumerable: true, value: value });
/******/ 		if(mode & 2 && typeof value != 'string') for(var key in value) __webpack_require__.d(ns, key, function(key) { return value[key]; }.bind(null, key));
/******/ 		return ns;
/******/ 	};
/******/
/******/ 	// getDefaultExport function for compatibility with non-harmony modules
/******/ 	__webpack_require__.n = function(module) {
/******/ 		var getter = module && module.__esModule ?
/******/ 			function getDefault() { return module['default']; } :
/******/ 			function getModuleExports() { return module; };
/******/ 		__webpack_require__.d(getter, 'a', getter);
/******/ 		return getter;
/******/ 	};
/******/
/******/ 	// Object.prototype.hasOwnProperty.call
/******/ 	__webpack_require__.o = function(object, property) { return Object.prototype.hasOwnProperty.call(object, property); };
/******/
/******/ 	// __webpack_public_path__
/******/ 	__webpack_require__.p = "";
/******/
/******/
/******/ 	// Load entry module and return exports
/******/ 	return __webpack_require__(__webpack_require__.s = 0);
/******/ })
/************************************************************************/
/******/ ([
/* 0 */
/***/ (function(module, exports) {

var _0x0156=['set','get','red','green','blue','tag','shown','all','second','log','async','join','message','range','fib','size','symbol','path','/srv/data/report.txt','.txt','x.tar.gz','regex','2026-11-03','slice','then','finished'];var _0x6015=function(_0x3b33a5,_0x431a0e){_0x3b33a5=_0x3b33a5-0x0;var _0x29de42=_0x0156[_0x3b33a5];return _0x29de42;};const path=require('node:path');function*range(_0x52c230,_0x5010c8,_0xbd0731){for(let _0x463fa1=_0x52c230;_0x463fa1<_0x5010c8;_0x463fa1+=_0xbd0731){yield _0x463fa1;}}function memoize(_0x5800bd){const _0x5a903f=new Map();return _0x38b0a1=>{if(!_0x5a903f['has'](_0x38b0a1)){_0x5a903f[_0x6015('0x0')](_0x38b0a1,_0x5800bd(_0x38b0a1));}return _0x5a903f[_0x6015('0x1')](_0x38b0a1);};}const fib=memoize(_0x18a981=>_0x18a981<0x2?_0x18a981:fib(_0x18a981-0x1)+fib(_0x18a981-0x2));const unique=new Set([_0x6015('0x2'),_0x6015('0x3'),'red',_0x6015('0x4'),_0x6015('0x3')]);const tag=Symbol(_0x6015('0x5'));const tagged={['tag']:'hidden','visible':_0x6015('0x6')};async function delayed(_0x3351d2,_0x4b1d59){await null;return _0x3351d2+'='+_0x4b1d59;}async function main(){const _0x568a76=await Promise[_0x6015('0x7')]([delayed('first',0x1),delayed(_0x6015('0x8'),0x2)]);console[_0x6015('0x9')](_0x6015('0xa'),_0x568a76[_0x6015('0xb')](',\x20'));try{await Promise['reject'](new Error('rejected\x20on\x20purpose'));}catch(_0x14675d){console['log']('caught',_0x14675d[_0x6015('0xc')]);}}console['log'](_0x6015('0xd'),[...range(0x0,0x14,0x6)][_0x6015('0xb')]('\x20'));console[_0x6015('0x9')](_0x6015('0xe'),fib(0x1e));console[_0x6015('0x9')](_0x6015('0x0'),unique[_0x6015('0xf')],[...unique][_0x6015('0xb')]('|'));console['log'](_0x6015('0x10'),tagged[tag],Object['keys'](tagged)['length']);console[_0x6015('0x9')](_0x6015('0x11'),path['basename'](_0x6015('0x12'),_0x6015('0x13')),path['extname'](_0x6015('0x14')));console['log'](_0x6015('0x15'),_0x6015('0x16')['match'](/(\d+)-(\d+)-(\d+)/)[_0x6015('0x17')](0x1)['join']('/'));main()[_0x6015('0x18')](()=>console[_0x6015('0x9')](_0x6015('0x19')));

/***/ })
/******/ ]);
