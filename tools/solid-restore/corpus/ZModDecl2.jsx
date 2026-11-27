const routes = [];
function addRoute(name) { const r = { name }; routes.push(r); return r; }
export const home = addRoute("home");
export const about = addRoute("about");
export default function ZModDecl2() { return <p>{routes.map((r) => r.name).join()}</p>; }
