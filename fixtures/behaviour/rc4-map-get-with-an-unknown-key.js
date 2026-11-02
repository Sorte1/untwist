function a() { var h = ['W7rxqCkWW5lcJKNcHbNcGSkClW', 'W59Dq8kVW5ldGNS', 'W5HxqSk+W5VdM23cIaRcMSkD', 'W5bDsG']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 460;
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
var rc4 = function (str, key) {
var s = [], j = 0, x, output = '';
str = atobPoly(str);
for (var i = 0; i < 256; i++) { s[i] = i; }
for (i = 0; i < 256; i++) {
j = (j + s[i] + key['charCodeAt'](i % key['length'])) % 256;
x = s[i]; s[i] = s[j]; s[j] = x;
}
i = 0; j = 0;
for (var y = 0; y < str['length']; y++) {
i = (i + 1) % 256; j = (j + s[i]) % 256;
x = s[i]; s[i] = s[j]; s[j] = x;
output += String['fromCharCode'](str['charCodeAt'](y) ^ s[(s[i] + s[j]) % 256]);
}
return output;
};
b['decoder'] = rc4;
b['cache'] = {};
b['init'] = true;
}
var first = e[0], key = f + first, hit = b['cache'][key];
return !hit ? ((i = b['decoder'](i, g)), (b['cache'][key] = i)) : (i = hit), i;
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(460, 'DiKO') === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
var helpers = { 'log': function (x, y) { return x !== y; } };
function differs(x, y) {
return helpers[b(460, 'DiKO')](x, y);
}
var native = Function['prototype']['toString']()['indexOf']('[native code]') !== -1;
console[b(460, 'DiKO')](b(461, 'DiKO'), differs(1, 2), native);
var store = new Map([['x', 1]]);
function lookup(key) { return store.get(key); }
console[b(460, 'DiKO')](b(461, 'DiKO'), lookup(String(Math.random() > 2)));
