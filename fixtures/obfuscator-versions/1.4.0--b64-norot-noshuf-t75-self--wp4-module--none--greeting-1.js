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

var _0x2ff3=['CMv0DxjUic8IicSGDgHPCYaRiciV','y29UC3rYDwn0B3i=','xIHBxIbDkYGGk1TEif0RksSPk1TEif19','DgvZDa==','AgvSBg8Sia==','ywrH','z3jHy2u=','BgvUz3rO','zg9Uzsb3AxrOia==','igD1zxn0CW=='];var _0x3571=function(_0x2ff35d,_0x35718e){_0x2ff35d=_0x2ff35d-0x0;var _0x59f361=_0x2ff3[_0x2ff35d];if(_0x3571['HLCvdT']===undefined){var _0x4365e1=function(_0x5307dc){var _0x2eb820='abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';var _0x7dfd49=String(_0x5307dc)['replace'](/=+$/,'');var _0x413d8c='';for(var _0x3e9bd3=0x0,_0x5314ab,_0x533866,_0x56a0b3=0x0;_0x533866=_0x7dfd49['charAt'](_0x56a0b3++);~_0x533866&&(_0x5314ab=_0x3e9bd3%0x4?_0x5314ab*0x40+_0x533866:_0x533866,_0x3e9bd3++%0x4)?_0x413d8c+=String['fromCharCode'](0xff&_0x5314ab>>(-0x2*_0x3e9bd3&0x6)):0x0){_0x533866=_0x2eb820['indexOf'](_0x533866);}return _0x413d8c;};_0x3571['WngqvG']=function(_0x548d3b){var _0x46f640=_0x4365e1(_0x548d3b);var _0x2af9c4=[];for(var _0x137362=0x0,_0x11cadb=_0x46f640['length'];_0x137362<_0x11cadb;_0x137362++){_0x2af9c4+='%'+('00'+_0x46f640['charCodeAt'](_0x137362)['toString'](0x10))['slice'](-0x2);}return decodeURIComponent(_0x2af9c4);};_0x3571['fPpmaG']={};_0x3571['HLCvdT']=!![];}var _0x39c4e6=_0x3571['fPpmaG'][_0x2ff35d];if(_0x39c4e6===undefined){var _0x1ac59d=function(_0x228b1d){this['WbjosP']=_0x228b1d;this['nTsvod']=[0x1,0x0,0x0];this['eiLtEL']=function(){return'newState';};this['hYyphL']='\x5cw+\x20*\x5c(\x5c)\x20*{\x5cw+\x20*';this['PWFOwt']='[\x27|\x22].+[\x27|\x22];?\x20*}';};_0x1ac59d['prototype']['Zuwjwe']=function(){var _0x23fe5b=new RegExp(this['hYyphL']+this['PWFOwt']);var _0x4dacb0=_0x23fe5b['test'](this['eiLtEL']['toString']())?--this['nTsvod'][0x1]:--this['nTsvod'][0x0];return this['qjaiIe'](_0x4dacb0);};_0x1ac59d['prototype']['qjaiIe']=function(_0x366f18){if(!Boolean(~_0x366f18)){return _0x366f18;}return this['ylYgnE'](this['WbjosP']);};_0x1ac59d['prototype']['ylYgnE']=function(_0x5c00b9){for(var _0x5b7dda=0x0,_0x72e562=this['nTsvod']['length'];_0x5b7dda<_0x72e562;_0x5b7dda++){this['nTsvod']['push'](Math['round'](Math['random']()));_0x72e562=this['nTsvod']['length'];}return _0x5c00b9(this['nTsvod'][0x0]);};new _0x1ac59d(_0x3571)['Zuwjwe']();_0x59f361=_0x3571['WngqvG'](_0x59f361);_0x3571['fPpmaG'][_0x2ff35d]=_0x59f361;}else{_0x59f361=_0x39c4e6;}return _0x59f361;};function greet(_0x293bb6){var _0x2d3b3b=function(){var _0x5b5799=!![];return function(_0x37deda,_0x5cb1f1){var _0x171490=_0x5b5799?function(){if(_0x5cb1f1){var _0x5e3f2a=_0x5cb1f1['apply'](_0x37deda,arguments);return _0x5cb1f1=null,_0x5e3f2a;}}:function(){};_0x5b5799=![];return _0x171490;};}(),_0x806a95=_0x2d3b3b(this,function(){var _0x4f1a5c=function(){var _0x17fcbc=_0x4f1a5c['constructor'](_0x3571('0x0'))()[_0x3571('0x1')](_0x3571('0x2'));return!_0x17fcbc[_0x3571('0x3')](_0x806a95);};return _0x4f1a5c();});_0x806a95();return _0x3571('0x4')+_0x293bb6;}var guests=[_0x3571('0x5'),_0x3571('0x6'),'linus'];for(var i=0x0;i<guests[_0x3571('0x7')];i++){console['log'](greet(guests[i]));}console['log'](_0x3571('0x8')+guests[_0x3571('0x7')]+_0x3571('0x9'));

/***/ })
/******/ ]);
