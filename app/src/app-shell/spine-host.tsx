import { createContext, useContext, useState, type ReactNode } from 'react';
const Host = createContext<HTMLElement | null>(null);
export function SpineHost({ children }: { children: ReactNode }) {
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  return <Host value={host}>{children}<div className="spine" ref={setHost} /></Host>;
}
export function useSpineHost(): HTMLElement | null { return useContext(Host); }
