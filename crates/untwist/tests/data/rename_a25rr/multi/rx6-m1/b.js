var s = Reflect.get(Reflect, "set"); s(globalThis, "f", function () { return 99; }); show();
