eval("var seen = 1; var d = Object.getOwnPropertyDescriptor(globalThis, 'seen'); console.log('decl-descriptor', d.configurable, d.writable, d.enumerable);");
