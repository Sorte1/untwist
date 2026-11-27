const k = "label";
function Kid() { return <b>k</b>; }
Kid[k] = "lbl";
export default function ZModComputed() { return <p><Kid />|{Kid.label}</p>; }
