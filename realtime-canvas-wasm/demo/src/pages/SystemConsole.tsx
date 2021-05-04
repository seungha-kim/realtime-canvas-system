import * as React from "react";
import {useCallback, useEffect, useRef, useState} from "react";
import {DocumentMaterial, Fragment, SystemEvent} from "../SystemFacade";
import { useSystemFacade } from "../contexts/SystemFacadeContext";
import { useToast } from "../contexts/ToastContext";

function getLocalPos(e: any): { x: number; y: number } {
  const rect = e.target.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  return { x, y };
}

type Props = {
  onLeave: () => void;
};

function SystemConsole(props: Props) {
  const system = useSystemFacade();
  const [documentMaterial, setDocumentMaterial] = useState<DocumentMaterial | null>(null)
  const prevPosRef = useRef<{ x: number; y: number } | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const toastControl = useToast();

  useEffect(() => {
    (window as any).system = system;

    (async () => {
      setDocumentMaterial(await system.materializeDocument())
    })()

    const handler = (e: any) => {
      const data = e.data as SystemEvent;
      if (data.SessionEvent?.Fragment) {
        draw(data.SessionEvent.Fragment);
      } else if (typeof data.SessionEvent?.SomeoneJoined !== "undefined") {
        toastControl.toast(
          "Someone joined: " + data.SessionEvent?.SomeoneJoined
        );
      } else if (typeof data.SessionEvent?.SomeoneLeft !== "undefined") {
        toastControl.toast("Someone left: " + data.SessionEvent?.SomeoneLeft);
      }
    };

    system.addEventListener("system", handler);

    return () => {
      system.removeEventListener("system", handler);
    };
  }, [system, toastControl]);

  const handleMouseDown = useCallback((e) => {
    const { x, y } = getLocalPos(e);
    prevPosRef.current = { x, y };
  }, []);

  const handleMouseUp = useCallback((e) => {
    prevPosRef.current = null;
  }, []);

  const sendFragment = useCallback(
    (x: number, y: number) => {
      system.sendFragment({
        x1: prevPosRef.current!.x,
        y1: prevPosRef.current!.y,
        x2: x,
        y2: y,
      });
    },
    [system]
  );

  const draw = useCallback((fragment: Fragment) => {
    const { x1, y1, x2, y2 } = fragment;
    const ctx = canvasRef.current!.getContext("2d")!;
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
    prevPosRef.current = { x: x2, y: y2 };
  }, []);

  const handleMouseMove = useCallback(
    (e) => {
      if (system && prevPosRef.current) {
        const { x, y } = getLocalPos(e);
        sendFragment(x, y);
      }
    },
    [system]
  );

  const handleLeave = useCallback(async () => {
    await system.leaveSession();
    props.onLeave();
  }, [system]);

  if (!system) {
    return <div>Loading</div>;
  } else {
    return (
      <div>
        <h1>{documentMaterial?.title}</h1>
        <canvas
          ref={canvasRef}
          width={100}
          height={100}
          style={{ width: 100, height: 100, border: "1px solid silver" }}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
        />
        <button onClick={handleLeave}>Leave</button>
      </div>
    );
  }
}

export default SystemConsole;
