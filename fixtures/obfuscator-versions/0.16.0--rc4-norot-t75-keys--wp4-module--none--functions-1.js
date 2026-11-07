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

var _0x53f0=['QMOgDRnDtls=','H8OIwpvChg==','w6o5CWs=','SMOsdsKn','w7diBw==','wqrDvXfChcK/UVHCvQ==','RRt1TA/DqH8=','eVteUA==','RwTCui4u','wpgNwqU=','ACNy','wowKw4k8wrY=','wooIwrjCqsOh'];var _0x28eb=function(_0x2f10bd,_0x2e30cb){_0x2f10bd=_0x2f10bd-0x0;var _0x3a2907=_0x53f0[_0x2f10bd];if(_0x28eb['MhSVYP']===undefined){(function(){var _0x2d675f=function(){var _0xf857d5;try{_0xf857d5=Function('return\x20(function()\x20'+'{}.constructor(\x22return\x20this\x22)(\x20)'+');')();}catch(_0x245717){_0xf857d5=window;}return _0xf857d5;};var _0x5a5a28=_0x2d675f();var _0x14149c='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=';_0x5a5a28['atob']||(_0x5a5a28['atob']=function(_0x372ef4){var _0x3f1474=String(_0x372ef4)['replace'](/=+$/,'');for(var _0x48238e=0x0,_0x26cc8a,_0x2d0302,_0x408bf7=0x0,_0x138aff='';_0x2d0302=_0x3f1474['charAt'](_0x408bf7++);~_0x2d0302&&(_0x26cc8a=_0x48238e%0x4?_0x26cc8a*0x40+_0x2d0302:_0x2d0302,_0x48238e++%0x4)?_0x138aff+=String['fromCharCode'](0xff&_0x26cc8a>>(-0x2*_0x48238e&0x6)):0x0){_0x2d0302=_0x14149c['indexOf'](_0x2d0302);}return _0x138aff;});}());var _0x496c9d=function(_0x3c2ef0,_0x1a3008){var _0x1cbb9b=[],_0x1d3c35=0x0,_0x26e839,_0xc3e06a='',_0x2e9dfb='';_0x3c2ef0=atob(_0x3c2ef0);for(var _0x176e6f=0x0,_0x29fdc5=_0x3c2ef0['length'];_0x176e6f<_0x29fdc5;_0x176e6f++){_0x2e9dfb+='%'+('00'+_0x3c2ef0['charCodeAt'](_0x176e6f)['toString'](0x10))['slice'](-0x2);}_0x3c2ef0=decodeURIComponent(_0x2e9dfb);for(var _0x47b020=0x0;_0x47b020<0x100;_0x47b020++){_0x1cbb9b[_0x47b020]=_0x47b020;}for(_0x47b020=0x0;_0x47b020<0x100;_0x47b020++){_0x1d3c35=(_0x1d3c35+_0x1cbb9b[_0x47b020]+_0x1a3008['charCodeAt'](_0x47b020%_0x1a3008['length']))%0x100;_0x26e839=_0x1cbb9b[_0x47b020];_0x1cbb9b[_0x47b020]=_0x1cbb9b[_0x1d3c35];_0x1cbb9b[_0x1d3c35]=_0x26e839;}_0x47b020=0x0;_0x1d3c35=0x0;for(var _0x1b5033=0x0;_0x1b5033<_0x3c2ef0['length'];_0x1b5033++){_0x47b020=(_0x47b020+0x1)%0x100;_0x1d3c35=(_0x1d3c35+_0x1cbb9b[_0x47b020])%0x100;_0x26e839=_0x1cbb9b[_0x47b020];_0x1cbb9b[_0x47b020]=_0x1cbb9b[_0x1d3c35];_0x1cbb9b[_0x1d3c35]=_0x26e839;_0xc3e06a+=String['fromCharCode'](_0x3c2ef0['charCodeAt'](_0x1b5033)^_0x1cbb9b[(_0x1cbb9b[_0x47b020]+_0x1cbb9b[_0x1d3c35])%0x100]);}return _0xc3e06a;};_0x28eb['cKOgjl']=_0x496c9d;_0x28eb['gVlZIZ']={};_0x28eb['MhSVYP']=!![];}var _0x430c19=_0x28eb['gVlZIZ'][_0x2f10bd];if(_0x430c19===undefined){if(_0x28eb['vAZIAP']===undefined){_0x28eb['vAZIAP']=!![];}_0x3a2907=_0x28eb['cKOgjl'](_0x3a2907,_0x2e30cb);_0x28eb['gVlZIZ'][_0x2f10bd]=_0x3a2907;}else{_0x3a2907=_0x430c19;}return _0x3a2907;};function factorial(_0x3a796f){return _0x3a796f<=0x1?0x1:_0x3a796f*factorial(_0x3a796f-0x1);}function compose(_0xc91a6b,_0x53a8b9){return function(_0x2dad0a){return _0x53a8b9(_0xc91a6b(_0x2dad0a));};}function makeCounter(_0x140ac6){var _0x2ed9fa=_0x140ac6;return{'next':function(){_0x2ed9fa+=0x1;return _0x2ed9fa;},'reset':function(){_0x2ed9fa=_0x140ac6;return _0x2ed9fa;}};}const double=_0x88f643=>_0x88f643*0x2;const increment=_0x434ba6=>_0x434ba6+0x1;const pipeline=compose(double,increment);function sum(..._0x17eb82){return _0x17eb82[_0x28eb('0x0','C5D5')]((_0xf95ab,_0x24e0da)=>_0xf95ab+_0x24e0da,0x0);}function greetAll(_0xbe9f5f='hi',..._0x26202d){return _0x26202d['map'](_0x2e5d50=>_0xbe9f5f+'\x20'+_0x2e5d50)[_0x28eb('0x1','bHGp')](';\x20');}const counter=makeCounter(0xa);counter[_0x28eb('0x2','DaCn')]();counter[_0x28eb('0x3','FTjY')]();console['log']('factorial',factorial(0x7));console[_0x28eb('0x4','sWpd')](_0x28eb('0x5','Yt9W'),pipeline(0x14));console['log'](_0x28eb('0x6','txk1'),counter[_0x28eb('0x7','n(zI')](),counter[_0x28eb('0x8','O2]s')]());console['log'](_0x28eb('0x9','lP)a'),sum(0x4,0x8,0xf,0x10,0x17,0x2a));console[_0x28eb('0xa','OCsk')]('greet',greetAll(undefined,'north','south'));console['log'](_0x28eb('0xb','ubc$'),Math['max'][_0x28eb('0xc','lP)a')](null,[0x3,0x9,0x1]));

/***/ })
/******/ ]);
