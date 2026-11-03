const _0x93a76b = _0x4728;
(function (_0x3dd353, _0x34561d) {
    const _0x43a18d = _0x4728, _0x506805 = _0x3dd353();
    while (!![]) {
        try {
            const _0x466df5 = parseInt(_0x43a18d(0x7d)) / 0x1 + parseInt(_0x43a18d(0x7f)) / 0x2 * (parseInt(_0x43a18d(0x79)) / 0x3) + parseInt(_0x43a18d(0x6a)) / 0x4 * (parseInt(_0x43a18d(0x7e)) / 0x5) + parseInt(_0x43a18d(0x70)) / 0x6 + -parseInt(_0x43a18d(0x7b)) / 0x7 * (parseInt(_0x43a18d(0x6f)) / 0x8) + -parseInt(_0x43a18d(0x82)) / 0x9 + -parseInt(_0x43a18d(0x7c)) / 0xa;
            if (_0x466df5 === _0x34561d)
                break;
            else
                _0x506805['push'](_0x506805['shift']());
        } catch (_0xc8c91f) {
            _0x506805['push'](_0x506805['shift']());
        }
    }
}(_0x2d6e, 0x3a859));
function classify(_0x109074) {
    const _0x3b5f7f = _0x4728;
    switch (typeof _0x109074) {
    case _0x3b5f7f(0x6e):
        if (_0x109074 < 0x0)
            return 'negative';
        return _0x109074 % 0x2 === 0x0 ? _0x3b5f7f(0x78) : _0x3b5f7f(0x6d);
    case _0x3b5f7f(0x71):
        return _0x3b5f7f(0x75) + _0x109074[_0x3b5f7f(0x66)];
    case _0x3b5f7f(0x72):
    case _0x3b5f7f(0x69):
        return _0x3b5f7f(0x6b);
    default:
        return 'other';
    }
}
function parse(_0x29b921) {
    const _0x46c714 = _0x4728;
    try {
        if (_0x29b921['length'] === 0x0)
            throw new Error(_0x46c714(0x68));
        return JSON['parse'](_0x29b921);
    } catch (_0x58df03) {
        return _0x46c714(0x74) + _0x58df03[_0x46c714(0x81)];
    } finally {
        console[_0x46c714(0x76)](_0x46c714(0x6c), _0x29b921[_0x46c714(0x66)]);
    }
}
function _0x4728(_0x30fd65, _0x13f851) {
    _0x30fd65 = _0x30fd65 - 0x66;
    const _0x2d6ec6 = _0x2d6e();
    let _0x472894 = _0x2d6ec6[_0x30fd65];
    return _0x472894;
}
let total = 0x0;
function _0x2d6e() {
    const _0xf07c8a = [
        '1151176gqqbXJ',
        '640128gncGZI',
        'string',
        'boolean',
        'join',
        'failed:\x20',
        'text\x20of\x20',
        'log',
        'push',
        'even',
        '28089KOxwgK',
        'map',
        '7bWdXzU',
        '7037790uPwNlc',
        '408626qvmJhd',
        '10kxbhQs',
        '78yZWlBQ',
        'abc',
        'message',
        '517698PhcpIJ',
        'length',
        'loops',
        'empty\x20input',
        'undefined',
        '528864cSqwnw',
        'flag',
        'parsed',
        'odd',
        'number'
    ];
    _0x2d6e = function () {
        return _0xf07c8a;
    };
    return _0x2d6e();
}
for (let i = 0x0; i < 0xa; i++) {
    if (i === 0x3)
        continue;
    if (i === 0x8)
        break;
    total += i;
}
let countdown = 0x5;
const seen = [];
while (countdown > 0x0) {
    seen[_0x93a76b(0x77)](countdown), countdown -= 0x2;
}
let rounds = 0x0;
do {
    rounds++;
} while (rounds < 0x4);
_0x10c8ed:
    for (let row = 0x0; row < 0x3; row++) {
        for (let col = 0x0; col < 0x3; col++) {
            if (col > row)
                continue _0x10c8ed;
            if (row === 0x2 && col === 0x1)
                break _0x10c8ed;
            total += row * 0xa + col;
        }
    }
const record = {
        'alpha': 0x1,
        'beta': 0x2
    }, keys = [];
for (const key in record) {
    keys['push'](key);
}
console[_0x93a76b(0x76)]('classify', [
    0x7,
    -0x1,
    0x4,
    _0x93a76b(0x80),
    !![],
    undefined,
    null
][_0x93a76b(0x7a)](classify)[_0x93a76b(0x73)](',')), console['log']('parse', parse('[1,2]')[_0x93a76b(0x66)], parse('')), console[_0x93a76b(0x76)](_0x93a76b(0x67), total, seen[_0x93a76b(0x73)]('-'), rounds, keys[_0x93a76b(0x73)]('+'));
