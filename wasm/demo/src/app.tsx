import * as React from "react";
import * as ReactDOM from "react-dom";
import SystemConsole from "./pages/SystemConsole";
import { SystemFacadeProvider } from "./contexts/SystemFacadeContext";
import { useState } from "react";
import Lobby from "./pages/Lobby";

function App() {
  const [route, setRoute] = useState("lobby");
  return (
    <SystemFacadeProvider>
      {route === "lobby" ? (
        <Lobby onJoin={() => setRoute("session")} />
      ) : route === "session" ? (
        <SystemConsole onLeave={() => setRoute("lobby")} />
      ) : null}
    </SystemFacadeProvider>
  );
}

ReactDOM.render(<App />, document.getElementById("root"));
