import * as React from "react";
import { useCallback, useRef } from "react";
import { useSystemFacade } from "../contexts/SystemFacadeContext";

type Props = {
  onJoin: () => void;
};

function Lobby(props: Props) {
  const system = useSystemFacade();
  const inputRef = useRef<HTMLInputElement>(null);

  const handleCreate = useCallback(async () => {
    console.log(await system.createSession());
    props.onJoin();
  }, [system]);

  const handleJoin = useCallback(async () => {
    const sessionId = parseInt(inputRef.current!.value, 10);
    console.log(await system.joinSession(sessionId));
    props.onJoin();
  }, [system]);

  return (
    <div>
      <div>
        <button onClick={handleCreate}>Create a session</button>
      </div>
      <div>Or</div>
      <div>
        Join a session: <input ref={inputRef} type="text" />
        <button onClick={handleJoin}>Join!</button>
      </div>
    </div>
  );
}

export default Lobby;
