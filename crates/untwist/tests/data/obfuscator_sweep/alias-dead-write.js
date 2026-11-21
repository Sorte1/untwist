var f = b;
function greet(c) {
    var d = b;
    return d(0xc3) + c;
}
function shout(c) {
    var e = b;
    if (e(0xc6) === e(0xc7)) {
        e = b[e(0xc5)](c);
    }
    return greet(c)[e(0xc4)]() + '!';
}
function a() {
    var g = [
        'AgvSBg8Sia',
        'Dg9vChbLCKnHC2u',
        'Bg9N',
        'ywrH',
        'z3jHy2u'
    ];
    a = function () {
        return g;
    };
    return a();
}
function b(c, d) {
    c = c - 0xc3;
    var e = a();
    var f = e[c];
    if (b['OUztfQ'] === undefined) {
        var g = function (j) {
            var l = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';
            var m = '', n = '';
            for (var o = 0x0, p, q, r = 0x0; q = j['charAt'](r++); ~q && (p = o % 0x4 ? p * 0x40 + q : q, o++ % 0x4) ? m += String['fromCharCode'](0xff & p >> (-0x2 * o & 0x6)) : 0x0) {
                q = l['indexOf'](q);
            }
            for (var s = 0x0, t = m['length']; s < t; s++) {
                n += '%' + ('00' + m['charCodeAt'](s)['toString'](0x10))['slice'](-0x2);
            }
            return decodeURIComponent(n);
        };
        b['KhyNzH'] = g, b['oPbphN'] = {}, b['OUztfQ'] = !![];
    }
    var h = e[0x0];
    b['wvoazE'] !== h && (b['oPbphN'] = {}, b['wvoazE'] = h);
    var i = b['oPbphN'][c];
    return i === undefined ? (f = b['KhyNzH'](f), b['oPbphN'][c] = f) : f = i, f;
}
console[f(0xc5)](greet(f(0xc6))), console[f(0xc5)](shout(f(0xc7)));