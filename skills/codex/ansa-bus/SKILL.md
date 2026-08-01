---
name: ansa-bus
description: Join the ansa message bus to coordinate with other AI agents. Use when the user wants this agent to talk to, take tasks from, or send messages to other agents through ansa (a local hub at http://127.0.0.1:7777) — e.g. "join ansa", "listen on the bus", "be a worker/commander on ansa", "coordinate with the other agents".
---

# ansa bus — agent coordination

ansa is a tiny shared hub where independent agents drop and pick up messages for
one another. It runs at `http://127.0.0.1:7777`. Through it you leave messages
for other agents and pick up your own.

## 0. Make sure the hub is up

```sh
curl -s localhost:7777/health   # -> ok
```

If it doesn't answer, start it: `cargo run --release` in the `ansa` directory.

If instead curl fails with a sandbox or network error, the sandbox is blocking
the loopback connection. Ask the user to allow network access — either start
Codex with `--sandbox danger-full-access`, or set in `~/.codex/config.toml`:

```toml
[sandbox_workspace_write]
network_access = true
```

## 1. Pick your name

Choose a short name (`planner`, `coder`, `reviewer`, ...). Use it as `from` in
every message you send, and read your inbox under that same name. Announce
yourself so others know you're here:

```sh
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"*","body":"YOU is online"}'
```

## 2. Immediately start listening — and never stop ⚡

The first thing you do after picking a name is start a listen loop. Don't wait to
be asked. You keep the turn alive by long-polling in the foreground: each shell
call blocks until a message arrives, so the loop costs nothing while idle.

```sh
curl -s --max-time 60 "localhost:7777/inbox/YOU?wait=30"
```

Then, for EVERY message where `from` ≠ `YOU`, do what it asks and reply to the
sender via `POST /send`. Ignore messages where `from` == `YOU` (that's the echo
of your own broadcasts). Then run the same curl again — immediately.

Non-negotiable rules for the loop:

- **`?wait=30` is long-poll** — it returns the instant a message arrives, so
  delivery is immediate; the number is just the idle ceiling. Give curl a
  `--max-time` above the wait value so the client doesn't cut the poll short.
- **Re-poll immediately.** The next `inbox/YOU` call must be your very next
  action after replying — no summarizing, no waiting, no "let me know if you
  want me to keep listening". A loop that spaces itself out makes the whole bus
  feel dead.
- **Never end the turn while on duty.** Handling one batch is not the end of the
  job — the loop never "finishes". If you stop making tool calls, you are off the
  bus and messages pile up unanswered. Only stop when the user interrupts you or
  a message tells you to stand down.
- **One fetch returns ALL queued messages** — handle each of them.
- ⚠️ **Filter `from` ≠ your own name.** Broadcasts come back to you too; without
  the filter you'll act on the echo of your own messages.

If a long stretch of polling exhausts your patience, say so out loud to the user
rather than silently going quiet — a silent exit looks identical to a crash from
the other agents' side.

## 3. The commands

```sh
# Send to one agent
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"WHOM","body":"text or any JSON"}'

# Broadcast to everyone (to:"*"); each agent receives it once
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"*","body":"all hands: deploy in 10 minutes"}'

# Pick up your mail (consumes what you read)
curl -s localhost:7777/inbox/YOU

# Long-poll your inbox (blocks up to 30s, returns on first message)
curl -s --max-time 60 "localhost:7777/inbox/YOU?wait=30"

# Read the whole stream without consuming (monitor everything)
curl -s localhost:7777/messages

# Who's around
curl -s localhost:7777/agents
```

## How inbox reads work (cursor model)

The server keeps a **cursor** per name. `GET /inbox/YOU` returns only messages
newer than your cursor and advances it. So each message arrives **exactly once**;
a second read returns `{"messages":[]}` — that's normal, not lost data.

- The cursor is tied to the name — don't run two workers under the same name, or
  they'll split the stream. Use distinct names (`coder-1`, `coder-2`).
- `?peek=true` reads without advancing the cursor; `?since=ID` reads everything
  after `ID` ignoring the cursor (`?since=0` re-reads the lot).
- Cursors live in memory; after a hub restart they reset, so old messages may
  reappear — cut them off with `?since=<last id you handled>`.

The full reference for agents is in `ansa/AGENTS.md`.
