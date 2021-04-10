// @ts-ignore
import mod from "../../pkg/realtime_canvas_bg.wasm";
import init, { CanvasSystem } from "../../pkg/realtime_canvas.js";

export interface Fragment {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export interface SystemEventData {
  JoinedSession?: { session_id: number };
  LeftSession?: null;
  SessionEvent?: {
    Fragment?: Fragment;
  };
}

export class SystemEvent extends Event {
  data: SystemEventData;

  constructor(type: string, data: SystemEventData) {
    super(type);
    this.data = data;
  }
}

export class SystemFacade extends EventTarget {
  private system: Promise<CanvasSystem>;
  private ws: WebSocket;

  constructor(url: string) {
    super();
    this.system = init(mod).then(() => new CanvasSystem());
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    this.setupWebSocketEventHandlers();
  }

  private setupWebSocketEventHandlers() {
    // this.ws.onopen = this.ws.onmessage = this.ws.onerror = this.ws.onclose = console.log
    this.ws.addEventListener("message", async (e) => {
      const buf = new Uint8Array(e.data);
      const json = (await this.system).translate_event_to_json(buf);
      const data: SystemEventData = JSON.parse(json);
      this.dispatchEvent(new SystemEvent("system", data));
    });
  }

  async createSession() {
    // TODO: check status
    return this.ws.send(
      (await this.system).translate_command_from_json(
        JSON.stringify({
          CreateSession: null,
        })
      )
    );
  }

  async joinSession(sessionId: number) {
    // TODO: check status
    return this.ws.send(
      (await this.system).translate_command_from_json(
        JSON.stringify({
          JoinSession: { session_id: sessionId },
        })
      )
    );
  }

  async leaveSession() {
    // TODO: check status
    return this.ws.send(
      (await this.system).translate_command_from_json(
        JSON.stringify({
          LeaveSession: null,
        })
      )
    );
  }

  async sendFragment(fragment: Fragment) {
    return this.ws.send(
      (await this.system).translate_command_from_json(
        JSON.stringify({
          SessionCommand: { Fragment: fragment },
        })
      )
    );
  }
}
