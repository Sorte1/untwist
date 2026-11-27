export default function ZFdd0() {
  const v = "V";
  const mark = "﷐";
  return <div title={`a
b ${v}`}><pre>{`x
  y ${v}
z`}</pre><p>{mark.length}{mark.charCodeAt(0)}</p></div>;
}
