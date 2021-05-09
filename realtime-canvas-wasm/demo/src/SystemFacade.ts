// @ts-ignore
import mod from "../../pkg/realtime_canvas_wasm_bg.wasm";
import init, { CanvasSystem } from "../../pkg/realtime_canvas_wasm.js";

export interface Fragment {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

type ConnectionId = number;
type SessionId = number;
type CommandId = number;

export type SystemEvent = {
  JoinedSession?: { session_id: SessionId };
  LeftSession?: null;
  SessionEvent?: {
    Fragment?: Fragment;
    SomeoneJoined: ConnectionId;
    SomeoneLeft: ConnectionId;
  };
};

type SystemCommand = {
  CreateSession?: null;
  JoinSession?: { session_id: SessionId };
  LeaveSession?: null;
  SessionCommand?: { Fragment: Fragment };
};

interface IdentifiableEvent {
  ByMyself?: {
    command_id: CommandId;
    result: {
      SystemEvent?: SystemEvent;
      Error?: any;
    };
  };
  BySystem?: {
    system_event: SystemEvent;
  };
}

type CommandResolver = {
  resolve: (value: SystemEvent) => void;
  reject: (error: any) => void;
};

export type DocumentMaterial = {
  id: string;
  title: string;
};

export type DocumentCommand = {
  UpdateDocumentTitle?: { title: string };
};

type InvalidationListener = (objectId: string) => void;
type SessionSnapshotListener = (sessionSnapshot: SessionSnapshot) => void;
export type SessionSnapshot = {
  connections: number[];
};

export class SystemFacade {
  private system: Promise<CanvasSystem>;
  private ws: WebSocket;
  private commandResolverRegistry: Map<CommandId, CommandResolver> = new Map();
  private invalidationListeners: Map<
    string,
    Set<InvalidationListener>
  > = new Map();
  private sessionSnapshotChangeListeners: Set<SessionSnapshotListener> = new Set();

  constructor(url: string) {
    this.system = init(mod).then(() => {
      const system = new CanvasSystem();
      (window as any).system = system;
      return system;
    });
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

      (await this.system).handle_event_from_server(buf);
      await this.notifyObjectInvalidation();
      await this.notifySessionSnapshotInvalidation();
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

  async materializeDocument(): Promise<DocumentMaterial> {
    return JSON.parse((await this.system).materialize_document());
  }

  async materializeSession(): Promise<SessionSnapshot> {
    return JSON.parse((await this.system).materialize_session());
  }

  async pushDocumentCommand(command: DocumentCommand) {
    (await this.system).push_document_command(JSON.stringify(command));
    await this.notifyObjectInvalidation();
    while (true) {
      const pendingCommand = (
        await this.system
      ).consume_pending_identifiable_command();
      if (pendingCommand) {
        this.ws.send(pendingCommand);
      } else {
        break;
      }
    }
  }

  addInvalidationListener(objectId: string, listener: InvalidationListener) {
    const listeners = this.invalidationListeners.get(objectId) ?? new Set();
    listeners.add(listener);
    this.invalidationListeners.set(objectId, listeners);
  }

  removeInvalidationListener(objectId: string, listener: InvalidationListener) {
    this.invalidationListeners.get(objectId)?.delete(listener);
  }

  addSessionSnapshotChangeListener(listener: SessionSnapshotListener) {
    this.sessionSnapshotChangeListeners.add(listener);
  }

  removeSessionSnapshotChangeListener(listener: SessionSnapshotListener) {
    this.sessionSnapshotChangeListeners.delete(listener);
  }

  private async notifyObjectInvalidation() {
    const invalidatedObjectIds = await this.consumeInvalidatedObjectIds();
    for (const objectId of invalidatedObjectIds) {
      const listeners = this.invalidationListeners.get(objectId);
      if (listeners) {
        for (const listener of listeners.values()) {
          listener(objectId);
        }
      }
    }
  }

  private async notifySessionSnapshotInvalidation() {
    const snapshotJson = (await this.system).consume_latest_session_snapshot();
    if (snapshotJson) {
      const parsed = JSON.parse(snapshotJson);
      for (const listener of this.sessionSnapshotChangeListeners) {
        listener(parsed);
      }
    }
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
    const systemEvent =
      event.BySystem?.system_event ??
      event.ByMyself?.result?.SystemEvent ??
      null;

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

  private static formatJson(obj: any) {
    return JSON.stringify(obj, null, 2);
  }

  private async consumeInvalidatedObjectIds(): Promise<string[]> {
    return JSON.parse((await this.system).consume_invalidated_object_ids());
  }
}
