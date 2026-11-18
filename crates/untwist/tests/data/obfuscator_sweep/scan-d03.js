"use strict"; class A { static m() { return (super.constructor ||= 0); } } A.m()("atob = function () { return \"patched\"; }")();
console.log(atob("aGk="));
