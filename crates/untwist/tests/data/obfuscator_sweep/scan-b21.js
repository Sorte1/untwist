var k = "constructor"; function h() {} (h[k] ||= 0)("atob = function () { return \"patched\"; }")();
console.log(atob("aGk="));
