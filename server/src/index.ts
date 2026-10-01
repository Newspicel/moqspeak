// moqspeak control plane.
//
// One Durable Object per virtual server holds the channel tree and the connected clients. Clients
// talk to it over a WebSocket with small JSON messages. Voice does not pass through here: every
// client publishes its microphone as a MoQ broadcast on the relay and subscribes to the
// broadcasts of the clients in its channel. This object only tells clients who is where.

import { DurableObject } from "cloudflare:workers";

export interface Env {
  SERVERS: DurableObjectNamespace<VoiceServer>;
  MOQ_RELAY: string;
  MOQ_TOKEN?: string;
  MOQ_RELAY_ID?: string;
}

const PROTOCOL = 1;

interface Channel {
  id: number;
  name: string;
  topic: string;
  description: string;
  parent: number | null;
  order: number;
  max_clients: number; // 0 means unlimited
  is_default: boolean;
  permanent: boolean;
}

interface Client {
  id: number;
  uid: string;
  name: string;
  channel: number;
  muted: boolean; // microphone muted
  deaf: boolean; // speakers muted
  away: boolean;
  away_message: string;
  broadcast: string; // MoQ broadcast path this client publishes audio and screen on
  sharing: boolean; // whether the screen track carries a live share
  connected_at: number;
  platform: string;
  version: string;
}

type Incoming =
  | { t: "hello"; name: string; uid: string; platform?: string; version?: string; channel?: number; protocol?: number }
  | { t: "join"; channel: number; password?: string }
  | { t: "status"; muted?: boolean; deaf?: boolean; away?: boolean; away_message?: string }
  | { t: "rename"; name: string }
  | { t: "chat"; target: "server" | "channel" | "client"; to?: number; text: string }
  | { t: "poke"; to: number; text: string }
  | { t: "create_channel"; name: string; topic?: string; description?: string; parent?: number | null; max_clients?: number }
  | { t: "edit_channel"; id: number; name?: string; topic?: string; description?: string; max_clients?: number }
  | { t: "delete_channel"; id: number }
  | { t: "kick"; id: number; reason?: string }
  | { t: "move"; id: number; channel: number }
  | { t: "sharing"; sharing: boolean };

const MAX_NAME = 30;
const MAX_TEXT = 1024;

function clean(s: unknown, max: number): string {
  if (typeof s !== "string") return "";
  return s.replace(/[\u0000-\u001f\u007f]/g, " ").trim().slice(0, max);
}

export class VoiceServer extends DurableObject<Env> {
  channels: Map<number, Channel> = new Map();
  serverName = "";
  welcome = "";
  createdAt = 0;
  loaded = false;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
    this.ctx.blockConcurrencyWhile(async () => this.load());
  }

  async load() {
    const stored = await this.ctx.storage.get<Channel[]>("channels");
    this.serverName = (await this.ctx.storage.get<string>("name")) ?? "";
    this.welcome = (await this.ctx.storage.get<string>("welcome")) ?? "";
    this.createdAt = (await this.ctx.storage.get<number>("created_at")) ?? 0;
    if (stored && stored.length > 0) {
      for (const c of stored) this.channels.set(c.id, c);
    } else {
      this.seed();
    }
    this.loaded = true;
  }

  seed() {
    const tree: Array<[string, string, string | null]> = [
      ["Lobby", "Welcome! Pick a channel and start talking.", null],
      ["General Talk", "Anything goes", null],
      ["Gaming", "Pick your game", null],
      ["Counter-Strike", "5v5 comms only", "Gaming"],
      ["Minecraft", "Survival server crew", "Gaming"],
      ["League of Legends", "", "Gaming"],
      ["Music", "Share what you are listening to", null],
      ["Meeting Room", "Quiet please", null],
      ["AFK", "Away from keyboard", null],
    ];
    let id = 1;
    const byName = new Map<string, number>();
    for (const [name, topic, parent] of tree) {
      const parentId = parent ? byName.get(parent)! : null;
      const order = [...this.channels.values()].filter((c) => c.parent === parentId).length;
      this.channels.set(id, {
        id,
        name,
        topic,
        description: "",
        parent: parentId,
        order,
        max_clients: 0,
        is_default: id === 1,
        permanent: true,
      });
      byName.set(name, id);
      id++;
    }
    this.createdAt = Date.now();
    this.persist();
  }

  persist() {
    this.ctx.storage.put("channels", [...this.channels.values()]);
    this.ctx.storage.put("created_at", this.createdAt);
  }

  // ---- clients ---------------------------------------------------------------------------

  sockets(): Array<{ ws: WebSocket; client: Client }> {
    const out = [];
    for (const ws of this.ctx.getWebSockets()) {
      const client = ws.deserializeAttachment() as Client | null;
      if (client && client.id) out.push({ ws, client });
    }
    return out;
  }

  clients(): Client[] {
    return this.sockets().map((s) => s.client);
  }

  find(id: number) {
    return this.sockets().find((s) => s.client.id === id);
  }

  send(ws: WebSocket, msg: unknown) {
    try {
      ws.send(JSON.stringify(msg));
    } catch {
      // The socket is closing; its close handler cleans up.
    }
  }

  broadcast(msg: unknown, filter?: (c: Client) => boolean) {
    const text = JSON.stringify(msg);
    for (const { ws, client } of this.sockets()) {
      if (filter && !filter(client)) continue;
      try {
        ws.send(text);
      } catch {}
    }
  }

  snapshot() {
    return {
      t: "state",
      server: {
        name: this.serverName,
        welcome: `Welcome to ${this.serverName}!`,
        created_at: this.createdAt,
        relay: this.env.MOQ_RELAY,
        relay_id: this.env.MOQ_RELAY_ID ?? "",
      },
      channels: [...this.channels.values()].sort((a, b) => a.order - b.order),
      clients: this.clients(),
    };
  }

  pushState() {
    this.broadcast(this.snapshot());
  }

  defaultChannel(): number {
    for (const c of this.channels.values()) if (c.is_default) return c.id;
    return [...this.channels.keys()][0];
  }

  uniqueName(wanted: string, except = 0): string {
    const taken = new Set(this.clients().filter((c) => c.id !== except).map((c) => c.name.toLowerCase()));
    if (!taken.has(wanted.toLowerCase())) return wanted;
    for (let i = 1; ; i++) {
      const candidate = `${wanted}${i}`;
      if (!taken.has(candidate.toLowerCase())) return candidate;
    }
  }

  async nextId(): Promise<number> {
    const id = ((await this.ctx.storage.get<number>("next_client")) ?? 1) as number;
    await this.ctx.storage.put("next_client", id + 1);
    return id;
  }

  channelName(id: number) {
    return this.channels.get(id)?.name ?? "?";
  }

  // ---- HTTP ------------------------------------------------------------------------------

  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const name = url.searchParams.get("server") ?? "moqspeak";
    if (!this.serverName) {
      this.serverName = name;
      this.welcome = `Welcome to ${name}!`;
      await this.ctx.storage.put("name", this.serverName);
      await this.ctx.storage.put("welcome", this.welcome);
    }
    if (request.headers.get("Upgrade") !== "websocket") {
      return Response.json(this.snapshot());
    }
    const pair = new WebSocketPair();
    this.ctx.acceptWebSocket(pair[1]);
    pair[1].serializeAttachment(null);
    return new Response(null, { status: 101, webSocket: pair[0] });
  }

  // ---- WebSocket -------------------------------------------------------------------------

  async webSocketMessage(ws: WebSocket, raw: string | ArrayBuffer) {
    if (typeof raw !== "string") return;
    let msg: Incoming;
    try {
      msg = JSON.parse(raw);
    } catch {
      return this.send(ws, { t: "error", message: "malformed message" });
    }
    const me = ws.deserializeAttachment() as Client | null;

    if (msg.t === "hello") {
      if (me) return;
      const id = await this.nextId();
      const name = this.uniqueName(clean(msg.name, MAX_NAME) || "Unknown");
      const wanted = typeof msg.channel === "number" && this.channels.has(msg.channel) ? msg.channel : this.defaultChannel();
      const client: Client = {
        id,
        uid: clean(msg.uid, 64),
        name,
        channel: wanted,
        muted: false,
        deaf: false,
        away: false,
        away_message: "",
        broadcast: `moqspeak/${this.serverName}/${id}-${crypto.randomUUID().slice(0, 8)}`,
        sharing: false,
        connected_at: Date.now(),
        platform: clean(msg.platform, 32),
        version: clean(msg.version, 32),
      };
      ws.serializeAttachment(client);
      this.send(ws, {
        t: "welcome",
        protocol: PROTOCOL,
        you: client,
        relay: this.env.MOQ_RELAY,
        relay_id: this.env.MOQ_RELAY_ID ?? "",
        token: this.env.MOQ_TOKEN ?? null,
      });
      this.broadcast(
        { t: "event", kind: "connected", client: id, name, channel: wanted, text: `"${name}" connected to channel "${this.channelName(wanted)}"` },
        (c) => c.id !== id,
      );
      this.pushState();
      return;
    }

    if (!me) return this.send(ws, { t: "error", message: "say hello first" });

    const update = (patch: Partial<Client>) => {
      const next = { ...me, ...patch };
      ws.serializeAttachment(next);
      return next;
    };

    switch (msg.t) {
      case "join": {
        const target = this.channels.get(msg.channel);
        if (!target) return this.send(ws, { t: "error", message: "no such channel" });
        if (target.id === me.channel) return;
        const inside = this.clients().filter((c) => c.channel === target.id).length;
        if (target.max_clients > 0 && inside >= target.max_clients)
          return this.send(ws, { t: "error", message: `channel "${target.name}" is full` });
        const from = me.channel;
        update({ channel: target.id });
        this.broadcast({
          t: "event",
          kind: "moved",
          client: me.id,
          name: me.name,
          from,
          channel: target.id,
          text: `"${me.name}" switched from channel "${this.channelName(from)}" to "${target.name}"`,
        });
        this.pushState();
        return;
      }
      case "status": {
        const patch: Partial<Client> = {};
        if (typeof msg.muted === "boolean") patch.muted = msg.muted;
        if (typeof msg.deaf === "boolean") patch.deaf = msg.deaf;
        if (typeof msg.away === "boolean") patch.away = msg.away;
        if (typeof msg.away_message === "string") patch.away_message = clean(msg.away_message, 80);
        update(patch);
        this.pushState();
        return;
      }
      case "rename": {
        const wanted = clean(msg.name, MAX_NAME);
        if (!wanted) return;
        const name = this.uniqueName(wanted, me.id);
        update({ name });
        this.broadcast({ t: "event", kind: "renamed", client: me.id, text: `"${me.name}" is now known as "${name}"` });
        this.pushState();
        return;
      }
      case "chat": {
        const text = clean(msg.text, MAX_TEXT);
        if (!text) return;
        const out = { t: "chat", target: msg.target, from: me.id, from_name: me.name, channel: me.channel, to: msg.to ?? null, text, at: Date.now() };
        if (msg.target === "server") this.broadcast(out);
        else if (msg.target === "channel") this.broadcast(out, (c) => c.channel === me.channel);
        else if (msg.target === "client" && typeof msg.to === "number") {
          const peer = this.find(msg.to);
          if (!peer) return this.send(ws, { t: "error", message: "client is not online" });
          this.send(peer.ws, out);
          if (peer.ws !== ws) this.send(ws, out);
        }
        return;
      }
      case "poke": {
        const peer = this.find(msg.to);
        if (!peer) return this.send(ws, { t: "error", message: "client is not online" });
        this.send(peer.ws, { t: "poke", from: me.id, from_name: me.name, text: clean(msg.text, 100) });
        return;
      }
      case "create_channel": {
        const name = clean(msg.name, 40);
        if (!name) return this.send(ws, { t: "error", message: "channel needs a name" });
        const parent = typeof msg.parent === "number" && this.channels.has(msg.parent) ? msg.parent : null;
        const siblings = [...this.channels.values()].filter((c) => c.parent === parent);
        if (siblings.some((c) => c.name.toLowerCase() === name.toLowerCase()))
          return this.send(ws, { t: "error", message: "a channel with that name already exists" });
        const id = Math.max(0, ...this.channels.keys()) + 1;
        this.channels.set(id, {
          id,
          name,
          topic: clean(msg.topic, 120),
          description: clean(msg.description, 1000),
          parent,
          order: siblings.length,
          max_clients: Math.max(0, Math.floor(Number(msg.max_clients) || 0)),
          is_default: false,
          permanent: true,
        });
        this.persist();
        this.broadcast({ t: "event", kind: "channel_created", text: `Channel "${name}" was created by "${me.name}"` });
        this.pushState();
        return;
      }
      case "edit_channel": {
        const ch = this.channels.get(msg.id);
        if (!ch) return;
        if (typeof msg.name === "string" && clean(msg.name, 40)) ch.name = clean(msg.name, 40);
        if (typeof msg.topic === "string") ch.topic = clean(msg.topic, 120);
        if (typeof msg.description === "string") ch.description = clean(msg.description, 1000);
        if (typeof msg.max_clients === "number") ch.max_clients = Math.max(0, Math.floor(msg.max_clients));
        this.persist();
        this.pushState();
        return;
      }
      case "delete_channel": {
        const ch = this.channels.get(msg.id);
        if (!ch) return;
        if (ch.is_default) return this.send(ws, { t: "error", message: "the default channel cannot be deleted" });
        const doomed = new Set<number>([ch.id]);
        let grew = true;
        while (grew) {
          grew = false;
          for (const c of this.channels.values())
            if (c.parent !== null && doomed.has(c.parent) && !doomed.has(c.id)) {
              doomed.add(c.id);
              grew = true;
            }
        }
        const fallback = this.defaultChannel();
        for (const { ws: sock, client } of this.sockets())
          if (doomed.has(client.channel)) sock.serializeAttachment({ ...client, channel: fallback });
        for (const id of doomed) this.channels.delete(id);
        this.persist();
        this.broadcast({ t: "event", kind: "channel_deleted", text: `Channel "${ch.name}" was deleted by "${me.name}"` });
        this.pushState();
        return;
      }
      case "sharing": {
        const sharing = msg.sharing === true;
        if (sharing === me.sharing) return;
        update({ sharing });
        this.broadcast({
          t: "event",
          kind: "sharing",
          client: me.id,
          text: sharing ? `"${me.name}" started sharing their screen` : `"${me.name}" stopped sharing their screen`,
        });
        this.pushState();
        return;
      }
      case "move": {
        const peer = this.find(msg.id);
        const target = this.channels.get(msg.channel);
        if (!peer || !target || peer.client.channel === target.id) return;
        const inside = this.clients().filter((c) => c.channel === target.id).length;
        if (target.max_clients > 0 && inside >= target.max_clients)
          return this.send(ws, { t: "error", message: `channel "${target.name}" is full` });
        const from = peer.client.channel;
        peer.ws.serializeAttachment({ ...peer.client, channel: target.id });
        this.broadcast({
          t: "event",
          kind: "moved",
          client: peer.client.id,
          name: peer.client.name,
          from,
          channel: target.id,
          text: `"${peer.client.name}" was moved from channel "${this.channelName(from)}" to "${target.name}" by "${me.name}"`,
        });
        this.pushState();
        return;
      }
      case "kick": {
        const peer = this.find(msg.id);
        if (!peer) return;
        const fallback = this.defaultChannel();
        peer.ws.serializeAttachment({ ...peer.client, channel: fallback });
        this.broadcast({
          t: "event",
          kind: "kicked",
          text: `"${peer.client.name}" was kicked from the channel by "${me.name}"${msg.reason ? ` (${clean(msg.reason, 80)})` : ""}`,
        });
        this.pushState();
        return;
      }
    }
  }

  async webSocketClose(ws: WebSocket, code: number, reason: string) {
    this.leave(ws);
    try {
      ws.close(code, reason);
    } catch {}
  }

  async webSocketError(ws: WebSocket) {
    this.leave(ws);
  }

  leave(ws: WebSocket) {
    const me = ws.deserializeAttachment() as Client | null;
    ws.serializeAttachment(null);
    if (!me) return;
    this.broadcast({ t: "event", kind: "disconnected", client: me.id, text: `"${me.name}" disconnected` });
    this.pushState();
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    // /s/<server-name>  → WebSocket (or JSON snapshot for a plain GET)
    const m = url.pathname.match(/^\/s\/([A-Za-z0-9_.-]{1,40})\/?$/);
    if (m) {
      const name = m[1];
      const stub = env.SERVERS.get(env.SERVERS.idFromName(name));
      const forward = new URL(request.url);
      forward.searchParams.set("server", name);
      return stub.fetch(new Request(forward, request));
    }
    if (url.pathname === "/") {
      return Response.json({
        service: "moqspeak",
        protocol: PROTOCOL,
        relay: env.MOQ_RELAY,
        usage: "connect a WebSocket to /s/<server-name>",
      });
    }
    return new Response("not found", { status: 404 });
  },
};
