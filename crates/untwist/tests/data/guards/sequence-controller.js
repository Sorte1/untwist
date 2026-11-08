var T = ['apply', 'return /" + this + "/', '^([^ ]+( +[^ ]+)+)+[^ ]}', 'constructor', 'test'];
var d = function (a, b) { return T[a - b]; };
console.log('start');
var j,A=(j=!0x0,function(K,L){var M=j?function(){var N=function(P,Q){return d(P- -1,Q);};if(L){var O=L[N(0,1)](K,arguments);return L=null,O;}}:function(){};return j=!0x1,M;})(this,function(){var Q=function(){return!Q[d(3,0)](d(1,0))()[d(3,0)](d(2,0))[d(4,0)](A);};return Q();});A();
console.log('end', T.length);
