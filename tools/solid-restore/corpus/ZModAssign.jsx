function Kid() { return <b>k</b>; }
Object.assign(Kid, { label: "lbl" });
export default function ZModAssign() { return <p><Kid />|{Kid.label}</p>; }
