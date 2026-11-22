function a() {
    var d = ['/w', 'yMv0yq', 'z2fTBwe', 'ywXWAge'];
    a = function () { return d; };
    return a();
}
function b(c, d) {
    c = c - 0xda;
    var e = a();
    var f = e[c];
    if (b['wmXyAk'] === undefined) {
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
        b['bdrnch'] = g, b['FNMOaF'] = {}, b['wmXyAk'] = !![];
    }
    var h = e[0x0];
    b['RcluFm'] !== h && (b['FNMOaF'] = {}, b['RcluFm'] = h);
    var k = c & 1;
    var i = b['FNMOaF'][k];
    
    return i === undefined ? (f = b['bdrnch'](f), b['FNMOaF'][k] = f) : f = i, f;
}
function p1() { try { console.log(b(0xda, 1)); } catch (x) { console.log("caught", String(x)); } }
function p2() { try { console.log(b(0xdc, 1)); } catch (x) { console.log("caught", String(x)); } }
function p3() { try { console.log(b(0xdb, 1)); } catch (x) { console.log("caught", String(x)); } }
p2(); p1(); p3(); p1(); p2();
