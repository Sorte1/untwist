function a() { var h = ['sgvSBg8Gv29YBgqH', 'y29UC29Szq', 'zgvVyMz1C2nHDgu', 'Bg9N']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
if (b['init'] === undefined) {
var atobPoly = function (input) {
var chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';
var output = '', temp = '';
for (var bc = 0, bs, buffer, idx = 0; (buffer = input['charAt'](idx++)); ~buffer && ((bs = bc % 4 ? bs * 64 + buffer : buffer), bc++ % 4) ? (output += String['fromCharCode'](255 & (bs >> ((-2 * bc) & 6)))) : 0) {
buffer = chars['indexOf'](buffer);
}
for (var k = 0, len = output['length']; k < len; k++) {
temp += '%' + ('00' + output['charCodeAt'](k)['toString'](16))['slice'](-2);
}
return decodeURIComponent(temp);
};
b['decoder'] = atobPoly;
b['cache'] = {};
b['init'] = true;
}
var first = e[0], key = f + first, hit = b['cache'][key];
return !hit ? ((i = b['decoder'](i)), (b['cache'][key] = i)) : (i = hit), i;
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(0) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
(function () { var t = { v: 0 }; return b(t.v); })();
(function () { var t = { v: 1 }; return b(t.v); })();
(function () { var t = { v: 2 }; return b(t.v); })();
(function () { var t = { v: 3 }; return b(t.v); })();
