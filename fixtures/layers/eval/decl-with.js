eval("var shadow = 'global'; with ({ shadow: 'object' }) { console.log('decl-with', shadow); } console.log(shadow);");
