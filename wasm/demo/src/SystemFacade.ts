// @ts-ignore
import mod from "../../pkg/realtime_canvas_bg.wasm";
import init, { CanvasSystem } from "../../pkg/realtime_canvas.js";

export interface Fragment {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

export type SystemEvent = {
  JoinedSession?: { session_id: number };
  LeftSession?: null;
  SessionEvent?: {
    Fragment?: Fragment;
  };
};

type SystemCommand = {
  CreateSession?: null;
  JoinSession?: { session_id: number };
  LeaveSession?: null;
  SessionCommand?: { Fragment: Fragment };
};

interface IdentifiableEvent {
  ByMyself?: {
    command_id: number;
    system_event: SystemEvent;
  };
  BySystem?: {
    system_event: SystemEvent;
  };
}

class SystemFacadeEvent extends Event {
  data: SystemEvent;

  constructor(type: string, data: SystemEvent) {
    super(type);
    this.data = data;
  }
}

type CommandResolver = (value: SystemEvent) => void;

export class SystemFacade extends EventTarget {
  private system: Promise<CanvasSystem>;
  private ws: WebSocket;
  private commandResolverRegistry: Map<number, CommandResolver> = new Map();

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
      const json = (await this.system).convert_event_to_json(buf);
      const parsed: IdentifiableEvent = JSON.parse(json);
      const systemEvent =
        parsed.BySystem?.system_event ?? parsed.ByMyself?.system_event!;
      this.dispatchEvent(new SystemFacadeEvent("system", systemEvent));
      if (
        parsed.ByMyself &&
        this.commandResolverRegistry.has(parsed.ByMyself.command_id)
      ) {
        const commandId = parsed.ByMyself.command_id;
        const resolver = this.commandResolverRegistry.get(commandId)!;
        this.commandResolverRegistry.delete(commandId);
        resolver(systemEvent);
      }
    });
  }

  async createSession(): Promise<SystemEvent> {
    return this.sendCommand({
      CreateSession: null,
    });
  }

  async joinSession(sessionId: number): Promise<SystemEvent> {
    return this.sendCommand({
      JoinSession: { session_id: sessionId },
    });
  }

  async leaveSession(): Promise<SystemEvent> {
    return this.sendCommand({
      LeaveSession: null,
    });
  }

  async sendFragment(fragment: Fragment): Promise<void> {
    return this.sendCommand(
      {
        SessionCommand: { Fragment: fragment },
      },
      false
    );
  }

  private async sendCommand(command: SystemCommand): Promise<SystemEvent>;
  private async sendCommand(
    command: SystemCommand,
    registerCommandResolver: false
  ): Promise<void>;
  private async sendCommand(
    command: SystemCommand,
    registerCommandResolver = true
  ): Promise<SystemEvent | void> {
    const commandBuf = (await this.system).create_command(
      JSON.stringify(command)
    );
    this.ws.send(commandBuf);
    if (registerCommandResolver) {
      const commandId = (await this.system).last_command_id();
      return new Promise((resolve) => {
        this.registerCommandResolver(commandId, resolve);
      });
    }
  }

  private registerCommandResolver(commandId: number, resolve: CommandResolver) {
    // TODO: timeout
    this.commandResolverRegistry.set(commandId, resolve);
  }
}
