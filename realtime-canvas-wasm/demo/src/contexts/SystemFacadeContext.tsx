import * as React from "react";
import { SystemFacade } from "../SystemFacade";

const SystemFacadeContext = React.createContext<SystemFacade | null>(null!);

type Props = {
  children: React.ReactNode;
};

export function SystemFacadeProvider(props: Props) {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  const host = "localhost:8080";
  const [facade, setFacade] = React.useState<SystemFacade | null>(null);
  React.useEffect(() => {
    (async () => {
      const facade = await SystemFacade.create(`${protocol}//${host}/ws/`);
      setFacade(facade);
    })();
  }, []);

  return (
    <SystemFacadeContext.Provider value={facade}>
      {props.children}
    </SystemFacadeContext.Provider>
  );
}

export function useSystemFacade() {
  return React.useContext(SystemFacadeContext);
}
