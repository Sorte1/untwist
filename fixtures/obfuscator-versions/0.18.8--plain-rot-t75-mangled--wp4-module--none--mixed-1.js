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

var a=['slice','then','finished','node:path','has','set','get','red','green','blue','tag','hidden','all','log','async','join','reject','rejected\x20on\x20purpose','caught','message','range','fib','size','symbol','keys','length','path','.txt','extname','x.tar.gz','2026-11-03','match'];(function(c,d){var e=function(f){while(--f){c['push'](c['shift']());}};e(++d);}(a,0x1c3));var b=function(c,d){c=c-0x0;var e=a[c];return e;};const path=require(b('0x0'));function*range(c,d,e){for(let f=c;f<d;f+=e){yield f;}}function memoize(g){const h=new Map();return i=>{if(!h[b('0x1')](i)){h[b('0x2')](i,g(i));}return h[b('0x3')](i);};}const fib=memoize(j=>j<0x2?j:fib(j-0x1)+fib(j-0x2));const unique=new Set([b('0x4'),b('0x5'),b('0x4'),b('0x6'),b('0x5')]);const tag=Symbol(b('0x7'));const tagged={[tag]:b('0x8'),'visible':'shown'};async function delayed(k,l){await null;return k+'='+l;}async function main(){const m=await Promise[b('0x9')]([delayed('first',0x1),delayed('second',0x2)]);console[b('0xa')](b('0xb'),m[b('0xc')](',\x20'));try{await Promise[b('0xd')](new Error(b('0xe')));}catch(n){console[b('0xa')](b('0xf'),n[b('0x10')]);}}console['log'](b('0x11'),[...range(0x0,0x14,0x6)]['join']('\x20'));console['log'](b('0x12'),fib(0x1e));console[b('0xa')](b('0x2'),unique[b('0x13')],[...unique]['join']('|'));console['log'](b('0x14'),tagged[tag],Object[b('0x15')](tagged)[b('0x16')]);console[b('0xa')](b('0x17'),path['basename']('/srv/data/report.txt',b('0x18')),path[b('0x19')](b('0x1a')));console['log']('regex',b('0x1b')[b('0x1c')](/(\d+)-(\d+)-(\d+)/)[b('0x1d')](0x1)[b('0xc')]('/'));main()[b('0x1e')](()=>console[b('0xa')](b('0x1f')));

/***/ })
/******/ ]);
