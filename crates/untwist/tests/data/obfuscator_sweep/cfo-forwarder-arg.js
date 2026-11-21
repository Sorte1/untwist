function a() {
    var g = [
        'ExldHvPKtM8',
        'swZcIq1cnvO2WPVdJmoi',
        'uwZcUW',
        'hSo0W5K',
        'iSk7WPa',
        'gSkcW4vfEG'
    ];
    a = function () {
        return g;
    };
    return a();
}
var f = b;
function greet(c) {
    var d = b;
    return d(0xc3, ']VXJ') + c;
}
function shout(c) {
    var v = {
        'gmDBv': function (f, k, m) {
            return f(k, m);
        },
        'ZFwSq': function (k, m) {
            return k === m;
        },
        'ScrZV': 'iEpNTx'
    };
    if (v['ZFwSq'](v['gmDBv'](W, 0xc7, 'dixq'), v['gmDBv'](W, 0xc9, 'suIV'))) {
        v[v['ScrZV']] = !![];
    }
    return greet(c)[v['gmDBv'](W, 0xc5, 'P^Qk')]() + '!';
}
function W(l, a) {
    return b(l - 0x1, a);
}
function b(c, d) {
    c = c - 0xc3;
    var e = a();
    var f = e[c];
    if (b['oPbphN'] === undefined) {
        var g = function (l) {
            var m = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=';
            var n = '', o = '';
            for (var p = 0x0, q, r, s = 0x0; r = l['charAt'](s++); ~r && (q = p % 0x4 ? q * 0x40 + r : r, p++ % 0x4) ? n += String['fromCharCode'](0xff & q >> (-0x2 * p & 0x6)) : 0x0) {
                r = m['indexOf'](r);
            }
            for (var t = 0x0, u = n['length']; t < u; t++) {
                o += '%' + ('00' + n['charCodeAt'](t)['toString'](0x10))['slice'](-0x2);
            }
            return decodeURIComponent(o);
        };
        var j = function (k, l) {
            var m = [], n = 0x0, o, p = '';
            k = g(k);
            var q;
            for (q = 0x0; q < 0x100; q++) {
                m[q] = q;
            }
            for (q = 0x0; q < 0x100; q++) {
                n = (n + m[q] + l['charCodeAt'](q % l['length'])) % 0x100, o = m[q], m[q] = m[n], m[n] = o;
            }
            q = 0x0, n = 0x0;
            for (var r = 0x0; r < k['length']; r++) {
                q = (q + 0x1) % 0x100, n = (n + m[q]) % 0x100, o = m[q], m[q] = m[n], m[n] = o, p += String['fromCharCode'](k['charCodeAt'](r) ^ m[(m[q] + m[n]) % 0x100]);
            }
            return p;
        };
        b['wvoazE'] = j, b['PnbyfA'] = {}, b['oPbphN'] = !![];
    }
    var h = e[0x0];
    b['LQNXRW'] !== h && (b['PnbyfA'] = {}, b['LQNXRW'] = h);
    var i = b['PnbyfA'][c];
    return i === undefined ? (b['VqgHIW'] === undefined && (b['VqgHIW'] = !![]), f = b['wvoazE'](f, d), b['PnbyfA'][c] = f) : f = i, f;
}
console[f(0xc5, 'P^Qk')](greet(f(0xc6, 'dixq'))), console[f(0xc7, 'ffux')](shout(f(0xc8, 'suIV')));