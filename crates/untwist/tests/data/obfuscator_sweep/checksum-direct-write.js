function f(){var arr0=['hello','3','4','world'];for(;;){try{if(11===parseInt(dec0(0))+parseInt(dec0(1))*2){arr0=arr0.concat('x');break}arr0.push(arr0.shift())}catch(e){arr0.push(arr0.shift())}}function dec0(a,b){a=a-0;var c=arr0[a];return c}return dec0(2)+dec0(3)+arr0.length}
console.log(f());
