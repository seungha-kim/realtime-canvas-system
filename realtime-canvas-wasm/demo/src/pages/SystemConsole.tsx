import * as React from "react";
import {
  DocumentMaterial,
  Fragment,
  SessionSnapshot,
  SystemFacade,
} from "../SystemFacade";
import { useSystemFacade } from "../contexts/SystemFacadeContext";
import { ToastController, useToast } from "../contexts/ToastContext";

function getLocalPos(e: any): { x: number; y: number } {
  const rect = e.target.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  return { x, y };
}

type Props = {
  onLeave: () => void;
};

type InnerProps = Props & {
  systemFacade: SystemFacade;
  toastController: ToastController;
};

type InnerState = {
  documentMaterial: DocumentMaterial | null;
  sessionSnapshot: SessionSnapshot | null;
};

class SystemConsoleInner extends React.Component<InnerProps, InnerState> {
  state: InnerState = {
    documentMaterial: null,
    sessionSnapshot: null,
  };
  canvasRef = React.createRef<HTMLCanvasElement>();
  prevPos: { x: number; y: number } | null = null;

  componentDidMount() {
    const { systemFacade } = this.props;
    const documentMaterial = systemFacade.materializeDocument();
    const sessionSnapshot = systemFacade.materializeSession();
    this.setState({ documentMaterial, sessionSnapshot });
    systemFacade.addInvalidationListener(
      documentMaterial.id,
      this.handleDocumentMaterialUpdate
    );
    systemFacade.addSessionSnapshotChangeListener(
      this.handleSessionSnapshotUpdate
    );
  }

  componentWillUnmount() {
    if (this.state.documentMaterial) {
      this.props.systemFacade.removeInvalidationListener(
        this.state.documentMaterial.id,
        this.handleDocumentMaterialUpdate
      );
    }
    this.props.systemFacade.removeSessionSnapshotChangeListener(
      this.handleSessionSnapshotUpdate
    );
  }

  handleDocumentMaterialUpdate = async () => {
    this.setState({
      documentMaterial: await this.props.systemFacade.materializeDocument(),
    });
  };

  handleSessionSnapshotUpdate = (sessionSnapshot: SessionSnapshot) => {
    this.setState({
      sessionSnapshot,
    });
  };

  handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (this.prevPos) {
      const { x, y } = getLocalPos(e);
      this.sendFragment(x, y);
    }
  };

  handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = getLocalPos(e);
    this.prevPos = { x, y };
  };

  handleMouseUp = () => {
    this.prevPos = null;
  };

  handleLeave = async () => {
    await this.props.systemFacade.leaveSession();
    this.props.onLeave();
  };

  handleTitleClick = () => {
    const title = prompt("New title?");
    if (title) {
      this.props.systemFacade.pushDocumentCommand({
        UpdateDocumentTitle: { title },
      });
    }
  };

  sendFragment = (x: number, y: number) => {
    this.props.systemFacade.sendFragment({
      x1: this.prevPos!.x,
      y1: this.prevPos!.y,
      x2: x,
      y2: y,
    });
  };

  draw = (fragment: Fragment) => {
    const { x1, y1, x2, y2 } = fragment;
    const ctx = this.canvasRef.current!.getContext("2d")!;
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
    this.prevPos = { x: x2, y: y2 };
  };

  render() {
    const { documentMaterial } = this.state;
    return (
      <div>
        <h1 onClick={this.handleTitleClick}>{documentMaterial?.title}</h1>
        <canvas
          ref={this.canvasRef}
          width={100}
          height={100}
          style={{ width: 100, height: 100, border: "1px solid silver" }}
          onMouseDown={this.handleMouseDown}
          onMouseMove={this.handleMouseMove}
          onMouseUp={this.handleMouseUp}
        />
        <button onClick={this.handleLeave}>Leave</button>
        <div>
          {this.state.sessionSnapshot?.connections.map((connectionId) => {
            return (
              <div key={connectionId} style={{ border: "1px solid red" }}>
                {connectionId}
              </div>
            );
          })}
        </div>
      </div>
    );
  }
}

function SystemConsole(props: Props) {
  const system = useSystemFacade();
  const toastController = useToast();

  if (system) {
    return (
      <SystemConsoleInner
        {...props}
        systemFacade={system}
        toastController={toastController}
      />
    );
  } else {
    return null;
  }
}

export default SystemConsole;
