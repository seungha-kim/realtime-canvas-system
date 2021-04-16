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
    SomeoneJoined: number; // connection id
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
    result: {
      SystemEvent?: SystemEvent;
      Error?: any;
    };
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

type CommandResolver = {
  resolve: (value: SystemEvent) => void;
  reject: (error: any) => void;
};

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
      this.handleIdentifiableEvent(parsed);
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
    SystemFacade.logCommand(command);
    const commandBuf = (await this.system).create_command(
      JSON.stringify(command)
    );
    this.ws.send(commandBuf);
    if (registerCommandResolver) {
      const commandId = (await this.system).last_command_id();
      return new Promise((resolve, reject) => {
        this.registerCommandResolver(commandId, { resolve, reject });
      });
    }
  }

  private registerCommandResolver(commandId: number, resolve: CommandResolver) {
    // TODO: timeout
    this.commandResolverRegistry.set(commandId, resolve);
  }

  private handleIdentifiableEvent(event: IdentifiableEvent) {
    SystemFacade.logEvent(event);
    const systemEvent =
      event.BySystem?.system_event ??
      event.ByMyself?.result?.SystemEvent ??
      null;
    if (systemEvent) {
      this.dispatchEvent(new SystemFacadeEvent("system", systemEvent));
    }

    if (
      event.ByMyself &&
      this.commandResolverRegistry.has(event.ByMyself.command_id)
    ) {
      const commandId = event.ByMyself.command_id;
      const resolver = this.commandResolverRegistry.get(commandId)!;
      this.commandResolverRegistry.delete(commandId);
      if (systemEvent) {
        resolver.resolve(systemEvent);
      } else {
        resolver.reject(event.ByMyself.result.Error);
      }
    }
  }

  private static logCommand(command: SystemCommand) {
    if (process.env.NODE_ENV == "production") {
      return;
    }
    if (command.SessionCommand?.Fragment) {
      console.debug(this.formatJson(command));
    } else {
      console.info(this.formatJson(command));
    }
  }

  private static logEvent(event: IdentifiableEvent) {
    if (process.env.NODE_ENV == "production") {
      return;
    }
    const error = event.ByMyself?.result.Error;
    const systemEvent =
      event.ByMyself?.result?.SystemEvent ?? event.BySystem?.system_event;
    if (error) {
      console.error(this.formatJson(event));
    }
    if (systemEvent?.SessionEvent?.Fragment) {
      console.debug(this.formatJson(event));
    } else {
      console.info(this.formatJson(event));
    }
  }

  private static formatJson(obj: any) {
    return JSON.stringify(obj, null, 2);
  }
}
