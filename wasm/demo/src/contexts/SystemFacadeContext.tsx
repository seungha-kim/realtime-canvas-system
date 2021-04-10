import * as React from "react";
import { SystemFacade } from "../SystemFacade";
import { createContext, useContext, useState } from "react";

const SystemFacadeContext = createContext<SystemFacade>(null!);

type Props = {
  children: React.ReactNode;
};

export function SystemFacadeProvider(props: Props) {
  const [facade] = useState(() => new SystemFacade("ws://localhost:8080/ws/"));
  return (
    <SystemFacadeContext.Provider value={facade}>
      {props.children}
    </SystemFacadeContext.Provider>
  );
}

export function useSystemFacade() {
  return useContext(SystemFacadeContext);
}
