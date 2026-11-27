const list = [];
const alias = list;
alias.push(1, 2);
export default function ZModAlias() { return <p>{list.join()}</p>; }
