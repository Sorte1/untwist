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

var _0x543a=['cmVzZXQ=','c3Vt','Z3JlZXQ=','bm9ydGg=','c291dGg=','YXBwbHk=','bWF4','bGFxV2c=','RWRxelE=','cFVMUWY=','V1dLRlk=','TWZDZVA=','aGdRWUE=','cmVkdWNl','bWFw','am9pbg==','bmV4dA==','bG9n','ZmFjdG9yaWFs','cGlwZWxpbmU=','Y291bnRlcg=='];(function(_0x478872,_0x3ca841){var _0x3b5572=function(_0x92e416){while(--_0x92e416){_0x478872['push'](_0x478872['shift']());}};_0x3b5572(++_0x3ca841);}(_0x543a,0x196));var _0x1fd1=function(_0xb440b5,_0x47e30c){_0xb440b5=_0xb440b5-0x0;var _0x24ca11=_0x543a[_0xb440b5];if(_0x1fd1['KChXbk']===undefined){(function(){var _0x244b91;try{var _0x468464=Function('return\x20(function()\x20'+'{}.constructor(\x22return\x20this\x22)(\x20)'+');');_0x244b91=_0x468464();}catch(_0x268c7c){_0x244b91=window;}var _0x4be3a9='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=';_0x244b91['atob']||(_0x244b91['atob']=function(_0x297e70){var _0x1d3fa8=String(_0x297e70)['replace'](/=+$/,'');for(var _0x5c44a7=0x0,_0x14ae56,_0x42cdaf,_0x26ad04=0x0,_0x442260='';_0x42cdaf=_0x1d3fa8['charAt'](_0x26ad04++);~_0x42cdaf&&(_0x14ae56=_0x5c44a7%0x4?_0x14ae56*0x40+_0x42cdaf:_0x42cdaf,_0x5c44a7++%0x4)?_0x442260+=String['fromCharCode'](0xff&_0x14ae56>>(-0x2*_0x5c44a7&0x6)):0x0){_0x42cdaf=_0x4be3a9['indexOf'](_0x42cdaf);}return _0x442260;});}());_0x1fd1['ALtPZo']=function(_0x5df5d7){var _0x5a4db4=atob(_0x5df5d7);var _0x5345da=[];for(var _0x1afe0d=0x0,_0xe072a1=_0x5a4db4['length'];_0x1afe0d<_0xe072a1;_0x1afe0d++){_0x5345da+='%'+('00'+_0x5a4db4['charCodeAt'](_0x1afe0d)['toString'](0x10))['slice'](-0x2);}return decodeURIComponent(_0x5345da);};_0x1fd1['AiSRFc']={};_0x1fd1['KChXbk']=!![];}var _0x40d27d=_0x1fd1['AiSRFc'][_0xb440b5];if(_0x40d27d===undefined){_0x24ca11=_0x1fd1['ALtPZo'](_0x24ca11);_0x1fd1['AiSRFc'][_0xb440b5]=_0x24ca11;}else{_0x24ca11=_0x40d27d;}return _0x24ca11;};function factorial(_0x3f49f4){var _0x344da9={'laqWg':function(_0x5565a9,_0x56f355){return _0x5565a9<=_0x56f355;},'EdqzQ':function(_0x597172,_0x17bd92){return _0x597172*_0x17bd92;},'pULQf':function(_0x52a6b0,_0x21ce06){return _0x52a6b0(_0x21ce06);},'WWKFY':function(_0xe753ac,_0x49c0df){return _0xe753ac-_0x49c0df;}};return _0x344da9[_0x1fd1('0x0')](_0x3f49f4,0x1)?0x1:_0x344da9[_0x1fd1('0x1')](_0x3f49f4,_0x344da9[_0x1fd1('0x2')](factorial,_0x344da9[_0x1fd1('0x3')](_0x3f49f4,0x1)));}function compose(_0x291194,_0x53e9df){var _0x4b38e2={'MfCeP':function(_0x5e5361,_0x15e7b6){return _0x5e5361(_0x15e7b6);},'hgQYA':function(_0x3b7014,_0x179de8){return _0x3b7014(_0x179de8);}};return function(_0x2908c5){return _0x4b38e2[_0x1fd1('0x4')](_0x53e9df,_0x4b38e2[_0x1fd1('0x5')](_0x291194,_0x2908c5));};}function makeCounter(_0x1b1a98){var _0x5bdcf4=_0x1b1a98;return{'next':function(){_0x5bdcf4+=0x1;return _0x5bdcf4;},'reset':function(){_0x5bdcf4=_0x1b1a98;return _0x5bdcf4;}};}const double=_0x216db7=>_0x216db7*0x2;const increment=_0x392158=>_0x392158+0x1;const pipeline=compose(double,increment);function sum(..._0x172fee){return _0x172fee[_0x1fd1('0x6')]((_0x4b2d56,_0x6f2c1f)=>_0x4b2d56+_0x6f2c1f,0x0);}function greetAll(_0x2889bf='hi',..._0x22f97b){return _0x22f97b[_0x1fd1('0x7')](_0x83c90b=>_0x2889bf+'\x20'+_0x83c90b)[_0x1fd1('0x8')](';\x20');}const counter=makeCounter(0xa);counter[_0x1fd1('0x9')]();counter[_0x1fd1('0x9')]();console[_0x1fd1('0xa')](_0x1fd1('0xb'),factorial(0x7));console[_0x1fd1('0xa')](_0x1fd1('0xc'),pipeline(0x14));console[_0x1fd1('0xa')](_0x1fd1('0xd'),counter[_0x1fd1('0x9')](),counter[_0x1fd1('0xe')]());console[_0x1fd1('0xa')](_0x1fd1('0xf'),sum(0x4,0x8,0xf,0x10,0x17,0x2a));console[_0x1fd1('0xa')](_0x1fd1('0x10'),greetAll(undefined,_0x1fd1('0x11'),_0x1fd1('0x12')));console[_0x1fd1('0xa')](_0x1fd1('0x13'),Math[_0x1fd1('0x14')][_0x1fd1('0x13')](null,[0x3,0x9,0x1]));

/***/ })
/******/ ]);
