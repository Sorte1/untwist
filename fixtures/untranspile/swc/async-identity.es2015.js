function asyncGeneratorStep(gen, resolve, reject, _next, _throw, key, arg) {
    try {
        var info = gen[key](arg);
        var value = info.value;
    } catch (error) {
        reject(error);
        return;
    }
    if (info.done) resolve(value);
    else Promise.resolve(value).then(_next, _throw);
}
function _async_to_generator(fn) {
    return function() {
        var self = this, args = arguments;
        return new Promise(function(resolve, reject) {
            var gen = fn.apply(self, args);
            function _next(value) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "next", value);
            }
            function _throw(err) {
                asyncGeneratorStep(gen, resolve, reject, _next, _throw, "throw", err);
            }
            _next(undefined);
        });
    };
}
function one(x) {
    return _async_to_generator(function*() {
        return x;
    })();
}
console.log(one.length, Object.getPrototypeOf(one) === Function.prototype, typeof one.prototype);
const order = [];
function a1() {
    return _async_to_generator(function*() {
        order.push(1);
        yield undefined;
        order.push(4);
        yield undefined;
        order.push(6);
    })();
}
a1();
order.push(2);
Promise.resolve().then(()=>order.push(5)).then(()=>order.push(7)).then(()=>order.push(8));
order.push(3);
setTimeout(()=>console.log(order.join(",")), 0);
function thenable() {
    return _async_to_generator(function*() {
        return {
            then (resolve) {
                resolve("thenable");
            }
        };
    })();
}
thenable().then((v)=>console.log(v));
