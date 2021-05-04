import * as React from "react";
import { SystemFacade } from "../SystemFacade";
import { createContext, useContext, useState } from "react";

const SystemFacadeContext = createContext<SystemFacade>(null!);

type Props = {
  children: React.ReactNode;
};

export function SystemFacadeProvider(props: Props) {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const host = "localhost:8080"
  const [facade] = useState(() => new SystemFacade(`${protocol}//${host}/ws/`));
  return (
    <SystemFacadeContext.Provider value={facade}>
      {props.children}
    </SystemFacadeContext.Provider>
  );
}

export function useSystemFacade() {
  return useContext(SystemFacadeContext);
}
