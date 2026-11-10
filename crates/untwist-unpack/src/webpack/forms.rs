pub const FORMS: &[(&str, &[&str])] = &[
    (
        "d",
        &[
            r#"$R.d=($0,$1)=>{for(const $2 in $1){if($R.o($1,$2)&&!$R.o($0,$2)){Object.defineProperty($0,$2,{enumerable:true,get:$1[$2]})}}};"#,
            r#"$R.d=($0,$1)=>{for(var $2 in $1){if($R.o($1,$2)&&!$R.o($0,$2)){Object.defineProperty($0,$2,{enumerable:true,get:$1[$2]})}}};"#,
            r#"$R.d=function($0,$1,$2){if(!$R.o($0,$1)){Object.defineProperty($0,$1,{enumerable:true,get:$2})}};"#,
            r#"$R.d=function($0,$1,$2){if(!$R.o($0,$1)){const $3={enumerable:true,get:$2};Object.defineProperty($0,$1,$3)}};"#,
        ],
    ),
    (
        "n",
        &[
            r#"$R.n=$0=>{const $1=$0&&$0.__esModule?()=>$0.default:()=>$0;$R.d($1,{a:$1});return $1};"#,
            r#"$R.n=$0=>{var $1=$0&&$0.__esModule?()=>$0.default:()=>$0;$R.d($1,{a:$1});return $1};"#,
            r#"$R.n=function($0){const $1=$0&&$0.__esModule?function $2(){return $0.default}:function $3(){return $0};$R.d($1,`a`,$1);return $1};"#,
            r#"$R.n=function($0){const $1=$0&&$0.__esModule?function(){return $0.default}:function(){return $0};$R.d($1,`a`,$1);return $1};"#,
            r#"$R.n=function($0){var $1=$0&&$0.__esModule?function $2(){return $0[`default`]}:function $3(){return $0};$R.d($1,`a`,$1);return $1};"#,
            r#"$R.n=function($0){var $1=$0&&$0.__esModule?function(){return $0[`default`]}:function(){return $0};$R.d($1,`a`,$1);return $1};"#,
        ],
    ),
    (
        "o",
        &[
            r#"$R.o=($0,$1)=>Object.prototype.hasOwnProperty.call($0,$1);"#,
            r#"$R.o=function($0,$1){return Object.prototype.hasOwnProperty.call($0,$1)};"#,
        ],
    ),
    (
        "r",
        &[
            r#"$R.r=$0=>{if(typeof Symbol!==`undefined`&&Symbol.toStringTag){Object.defineProperty($0,Symbol.toStringTag,{value:`Module`})}Object.defineProperty($0,"__esModule",{value:true})};"#,
            r#"$R.r=function($0){if(typeof Symbol!==`undefined`&&Symbol.toStringTag){Object.defineProperty($0,Symbol.toStringTag,{value:`Module`})}Object.defineProperty($0,"__esModule",{value:true})};"#,
        ],
    ),
    (
        "t",
        &[
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}const $2=Object.create(null);$R.r($2);Object.defineProperty($2,"default",{enumerable:true,value:$0});if($1&2&&typeof $0!==`string`){for(const $3 in $0){$R.d($2,$3,function($1){return $0[$1]}.bind(null,$3))}}return $2};"#,
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}const $2=Object.create(null);$R.r($2);Object.defineProperty($2,"default",{enumerable:true,value:$0});if($1&2&&typeof $0!==`string`){for(const $3 in $0){$R.d($2,$3,function($3){return $0[$3]}.bind(null,$3))}}return $2};"#,
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}const $2=Object.create(null);$R.r($2);Object.defineProperty($2,"default",{enumerable:true,value:$0});if($1&2&&typeof $0!==`string`){for(const $3 in $0){$R.d($2,$3,function($4){return $0[$4]}.bind(null,$3))}}return $2};"#,
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}const $2=Object.create(null);$R.r($2);const $3={enumerable:true,value:$0};Object.defineProperty($2,"default",$3);if($1&2&&typeof $0!==`string`){for(const $4 in $0){$R.d($2,$4,function($1){return $0[$1]}.bind(null,$4))}}return $2};"#,
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}var $2=Object.create(null);$R.r($2);Object.defineProperty($2,"default",{enumerable:true,value:$0});if($1&2&&typeof $0!==`string`){for(var $3 in $0)$R.d($2,$3,function($3){return $0[$3]}.bind(null,$3))}return $2};"#,
            r#"$R.t=function($0,$1){if($1&1){$0=$R($0)}if($1&8){return $0}if($1&4&&typeof $0===`object`&&$0&&$0.__esModule){return $0}var $2=Object.create(null);$R.r($2);Object.defineProperty($2,"default",{enumerable:true,value:$0});if($1&2&&typeof $0!==`string`){for(var $3 in $0){$R.d($2,$3,function($1){return $0[$1]}.bind(null,$3))}}return $2};"#,
            r#"(()=>{var $0,$1;$1=Object.getPrototypeOf?$0=>Object.getPrototypeOf($0):$0=>$0.__proto__;$R.t=function($2,$3){if($3&1){$2=this($2)}if($3&8){return $2}if(typeof $2===`object`&&$2){if($3&4&&$2.__esModule){return $2}if($3&16&&typeof $2.then===`function`){return $2}}const $4=Object.create(null);$R.r($4);var $5={};$0=$0||[null,$1({}),$1([]),$1($1)];for(let $6=$3&2&&$2;typeof $6===`object`&&!~$0.indexOf($6);$6=$1($6)){Object.getOwnPropertyNames($6).forEach($0=>$5[$0]=()=>$2[$0])}$5.default=()=>$2;$R.d($4,$5);return $4}})();"#,
            r#"(()=>{var $0=Object.getPrototypeOf?$1=>Object.getPrototypeOf($1):$1=>$1.__proto__;var $2;$R.t=function($3,$4){if($4&1){$3=this($3)}if($4&8){return $3}if(typeof $3===`object`&&$3){if($4&4&&$3.__esModule){return $3}if($4&16&&typeof $3.then===`function`){return $3}}const $5=Object.create(null);$R.r($5);var $6={};$2=$2||[null,$0({}),$0([]),$0($0)];for(let $7=$4&2&&$3;typeof $7===`object`&&!~$2.indexOf($7);$7=$0($7)){Object.getOwnPropertyNames($7).forEach($8=>$6[$8]=()=>$3[$8])}$6.default=()=>$3;$R.d($5,$6);return $5}})();"#,
            r#"(()=>{var $0=Object.getPrototypeOf?$1=>Object.getPrototypeOf($1):$1=>$1.__proto__;var $2;$R.t=function($3,$4){if($4&1){$3=this($3)}if($4&8){return $3}if(typeof $3===`object`&&$3){if($4&4&&$3.__esModule){return $3}if($4&16&&typeof $3.then===`function`){return $3}}var $5=Object.create(null);$R.r($5);var $6={};$2=$2||[null,$0({}),$0([]),$0($0)];for(var $7=$4&2&&$3;typeof $7===`object`&&!~$2.indexOf($7);$7=$0($7)){Object.getOwnPropertyNames($7).forEach($8=>$6[$8]=()=>$3[$8])}$6[`default`]=()=>$3;$R.d($5,$6);return $5}})();"#,
        ],
    ),
];
