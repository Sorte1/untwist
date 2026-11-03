const _0x3411de = _0x4991;
(function (_0x269942, _0x48ef41) {
    const _0x5e82f2 = _0x4991, _0x2fbe92 = _0x269942();
    while (!![]) {
        try {
            const _0x1a1a45 = parseInt(_0x5e82f2(0x208)) / 0x1 + parseInt(_0x5e82f2(0x1f4)) / 0x2 + -parseInt(_0x5e82f2(0x20a)) / 0x3 * (-parseInt(_0x5e82f2(0x1f8)) / 0x4) + -parseInt(_0x5e82f2(0x205)) / 0x5 + -parseInt(_0x5e82f2(0x207)) / 0x6 * (parseInt(_0x5e82f2(0x1fb)) / 0x7) + parseInt(_0x5e82f2(0x209)) / 0x8 + -parseInt(_0x5e82f2(0x1fc)) / 0x9;
            if (_0x1a1a45 === _0x48ef41)
                break;
            else
                _0x2fbe92['push'](_0x2fbe92['shift']());
        } catch (_0x3fb2c7) {
            _0x2fbe92['push'](_0x2fbe92['shift']());
        }
    }
}(_0x450f, 0xcb658));
class Animal {
    constructor(_0x4b141e, _0x2e7261) {
        const _0x881910 = _0x4991;
        this[_0x881910(0x1f5)] = _0x4b141e, this[_0x881910(0x1f9)] = _0x2e7261;
    }
    ['describe']() {
        const _0x2addbb = _0x4991;
        return this[_0x2addbb(0x1f5)] + '\x20walks\x20on\x20' + this['legs'] + _0x2addbb(0x1f1);
    }
    static [_0x3411de(0x201)](_0x4bb0f7, _0x19c04c) {
        return _0x4bb0f7['legs'] - _0x19c04c['legs'];
    }
    get ['kind']() {
        const _0x134ac7 = _0x3411de;
        return _0x134ac7(0x204);
    }
}
class Bird extends Animal {
    constructor(_0x25df08) {
        const _0x2cf8b9 = _0x3411de;
        super(_0x25df08, 0x2), this[_0x2cf8b9(0x202)] = 0x2;
    }
    [_0x3411de(0x206)]() {
        const _0x4837b8 = _0x3411de;
        return super[_0x4837b8(0x206)]() + '\x20and\x20flies\x20with\x20' + this[_0x4837b8(0x202)] + _0x4837b8(0x203);
    }
    get ['kind']() {
        const _0x2b1a46 = _0x3411de;
        return _0x2b1a46(0x1fa);
    }
}
class Tally {
    [_0x3411de(0x20b)] = 0x0;
    ['add'](_0x28c3ce) {
        const _0xee9528 = _0x3411de;
        return this[_0xee9528(0x20b)] += _0x28c3ce, this;
    }
}
function _0x4991(_0x127c30, _0x4340ac) {
    _0x127c30 = _0x127c30 - 0x1f1;
    const _0x450f50 = _0x450f();
    let _0x4991a5 = _0x450f50[_0x127c30];
    return _0x4991a5;
}
function _0x450f() {
    const _0x2be7fe = [
        'legs',
        'bird',
        '21DlVLHt',
        '25762977JZDQJz',
        'log',
        'kind',
        'sort',
        'crow',
        'compare',
        'wings',
        '\x20wings',
        'animal',
        '1977600TZfrKE',
        'describe',
        '44628oREuxL',
        '937344qPuUAn',
        '7985400bxOEnH',
        '3iRthfO',
        'count',
        '\x20legs',
        'add',
        'cat',
        '2295564jRbUxM',
        'name',
        'tally',
        'is\x20bird',
        '4120792buOdxZ'
    ];
    _0x450f = function () {
        return _0x2be7fe;
    };
    return _0x450f();
}
const zoo = [
    new Animal(_0x3411de(0x1f3), 0x4),
    new Bird(_0x3411de(0x200)),
    new Animal('snail', 0x0)
];
zoo[_0x3411de(0x1ff)](Animal[_0x3411de(0x201)]);
for (const creature of zoo) {
    console[_0x3411de(0x1fd)](creature[_0x3411de(0x206)](), creature[_0x3411de(0x1fe)]);
}
console[_0x3411de(0x1fd)](_0x3411de(0x1f7), zoo[0x1] instanceof Bird, zoo[0x1] instanceof Animal), console[_0x3411de(0x1fd)](_0x3411de(0x1f6), new Tally()[_0x3411de(0x1f2)](0x3)[_0x3411de(0x1f2)](0x4)[_0x3411de(0x20b)]);
