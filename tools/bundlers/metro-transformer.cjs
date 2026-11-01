const babel = require("@babel/core");

module.exports.transform = ({ filename, src, plugins }) => {
  const ast = babel.parseSync(src, { filename, babelrc: false, configFile: false, sourceType: "unambiguous" });
  const result = babel.transformFromAstSync(ast, src, {
    filename, babelrc: false, configFile: false, ast: true, code: false, plugins: plugins || [], sourceType: "unambiguous",
  });
  return { ast: result.ast, metadata: result.metadata };
};
