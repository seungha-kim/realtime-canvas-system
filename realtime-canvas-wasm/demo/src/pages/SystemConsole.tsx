import * as React from "react";
import {
  DocumentMaterial,
  Fragment,
  SystemEvent,
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
};

class SystemConsoleInner extends React.Component<InnerProps, InnerState> {
  state: InnerState = {
    documentMaterial: null,
  };
  canvasRef = React.createRef<HTMLCanvasElement>();
  prevPos: { x: number; y: number } | null = null;

  componentDidMount() {
    const { systemFacade } = this.props;
    systemFacade.materializeDocument().then((d) => {
      this.setState({ documentMaterial: d });
      systemFacade.addInvalidationListener(
        d.id,
        this.handleDocumentMaterialUpdate
      );
    });

    systemFacade.addEventListener("system", this.systemEventHandler);
  }

  componentWillUnmount() {
    this.props.systemFacade.removeEventListener(
      "system",
      this.systemEventHandler
    );
    if (this.state.documentMaterial) {
      this.props.systemFacade.removeInvalidationListener(
        this.state.documentMaterial.id,
        this.handleDocumentMaterialUpdate
      );
    }
  }

  handleDocumentMaterialUpdate = async () => {
    this.setState({
      documentMaterial: await this.props.systemFacade.materializeDocument(),
    });
  };

  systemEventHandler = (e: any) => {
    const data = e.data as SystemEvent;
    if (data.SessionEvent?.Fragment) {
      this.draw(data.SessionEvent.Fragment);
    } else if (typeof data.SessionEvent?.SomeoneJoined !== "undefined") {
      this.props.toastController.showToast(
        "Someone joined: " + data.SessionEvent?.SomeoneJoined
      );
    } else if (typeof data.SessionEvent?.SomeoneLeft !== "undefined") {
      this.props.toastController.showToast(
        "Someone left: " + data.SessionEvent?.SomeoneLeft
      );
    }
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
      </div>
    );
  }
}

function SystemConsole(props: Props) {
  const system = useSystemFacade();
  const toastController = useToast();

  return (
    <SystemConsoleInner
      {...props}
      systemFacade={system}
      toastController={toastController}
    />
  );
}

export default SystemConsole;
