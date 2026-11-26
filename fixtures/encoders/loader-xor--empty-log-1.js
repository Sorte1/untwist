var key = 23;
eval([116,120,121,100,120,123,114,57,123,120,112,63,53,53,62,44,29].map(function (c) { return String.fromCharCode(c ^ key); }).join(""));
