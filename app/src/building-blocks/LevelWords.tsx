import './LevelWords.css';
/** Keep anatomical names in the mono face without changing their surrounding words. */
export function LevelWords({children}:{children:string}) {
  return children.split(/\b(C[1-7]|T(?:1[0-3]|[1-9])|L[1-6]|S1)\b/g).map((part,i)=>/^(?:C[1-7]|T(?:1[0-3]|[1-9])|L[1-6]|S1)$/.test(part)?<span className="level-name" key={i}>{part}</span>:part);
}
