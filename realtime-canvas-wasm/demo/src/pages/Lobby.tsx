import * as React from "react";
import { useSystemFacade } from "../contexts/SystemFacadeContext";
import { SystemFacade } from "../SystemFacade";

type Props = {
  onJoin: () => void;
};

type InnerProps = Props & {
  system: SystemFacade;
};

class LobbyInner extends React.Component<InnerProps> {
  inputRef = React.createRef<HTMLInputElement>();

  handleCreate = async () => {
    await this.props.system.createSession();
    this.props.onJoin();
  };

  handleJoin = async () => {
    const sessionId = parseInt(this.inputRef.current!.value, 10);
    try {
      await this.props.system.joinSession(sessionId);
      this.props.onJoin();
    } catch (e) {
      alert(e);
    }
  };

  render() {
    return (
      <div>
        <div>
          <button onClick={this.handleCreate}>Create a session</button>
        </div>
        <div>Or</div>
        <div>
          Join a session: <input ref={this.inputRef} type="text" />
          <button onClick={this.handleJoin}>Join!</button>
        </div>
      </div>
    );
  }
}

function Lobby(props: Props) {
  const system = useSystemFacade();
  if (system) {
    return <LobbyInner {...props} system={system} />;
  } else {
    return null;
  }
}

export default Lobby;
