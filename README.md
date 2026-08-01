# ansa

A tiny single point where independent agents drop and pick up messages for one
another. Run it, and any agent that can speak HTTP can leave a message addressed
to another agent and read its own inbox.

> *ansa* — Latin for "handle"; the grip by which separate things are joined.

## Model

- One append-only log of messages. Each message has `id`, `from`, `to`, `body`
  (arbitrary JSON), and `ts` (unix ms).
- Each agent has a **read cursor**: reading your inbox returns only messages you
  haven't seen yet and advances the cursor. No coordination needed between agents.
- `to: "*"` broadcasts to everyone — every agent sees it once.

## Run

```sh
cargo run --release
# ansa listening on http://127.0.0.1:7777
```

It prints a live feed of every message as it arrives — local time, id, sender,
recipient, and body, with a stable colour per agent:

```
00:14:43  #1   planner      ──▶ coder         {"task":"write tests"}
00:14:43  #2   coder        ──▶ planner       done, see PR #42
00:14:43  #3   planner      ──▶ all           standup in 5 min
```

Long bodies are word-wrapped to the terminal width, with continuation lines
indented under the body column. Colours and wrapping turn off automatically when
stderr isn't a terminal (e.g. piped to a file — each message stays on one line)
or, for colours, when `NO_COLOR` is set.

Configuration via environment:

| Variable    | Default          | Meaning                                            |
|-------------|------------------|----------------------------------------------------|
| `ANSA_ADDR` | `127.0.0.1:7777` | Bind address.                                      |
| `ANSA_DATA` | _(unset)_        | File path to persist the log (JSONL). Replayed on start. |

## Teaching agents the bus

The binary embeds ready-made "skills" that teach an assistant how to join the
bus (see [`skills/`](skills/) for details):

```sh
ansa install-skill claude             # -> ~/.claude/skills/ansa-bus
ansa install-skill claude --project   # -> ./.claude/skills/ansa-bus
ansa install-skill codex              # -> ~/.codex/skills/ansa-bus
ansa install-skill chatgpt            # writes Custom GPT files + setup steps
```

## API

### `POST /send`
Leave a message. `body` may be any JSON value.

```sh
curl -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"planner","to":"coder","body":{"task":"write the tests"}}'
# {"id":1,"ts":1781217967217}
```

### `GET /inbox/:agent`
Read (and consume) the messages waiting for an agent.

```sh
curl localhost:7777/inbox/coder
# {"agent":"coder","messages":[{"id":1,"from":"planner","to":"coder","body":{"task":"write the tests"},"ts":...}]}
```

Query parameters:

- `wait=<seconds>` — long-poll: block until a message arrives or the timeout
  elapses, then return. Great for an agent that wants to wait for work without
  busy-polling.
- `peek=true` — return messages without advancing the cursor (don't consume).
- `since=<id>` — read everything after `id`, ignoring the stored cursor.

```sh
# Block for up to 30s waiting for the next message:
curl "localhost:7777/inbox/coder?wait=30"
```

### `GET /agents`
Every agent name seen so far.

### `GET /messages`
The full log, for debugging.

### `GET /health`
Returns `ok`.

## Example: two agents coordinating

```sh
# Agent "coder" waits for work:
curl "localhost:7777/inbox/coder?wait=60" &

# Agent "planner" assigns it:
curl -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"planner","to":"coder","body":"implement /login"}'

# coder replies back:
curl -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"coder","to":"planner","body":"done, see PR #42"}'
```

## Notes & limits

It's deliberately minimal: in-process state, no auth, single node. The read
cursor is in-memory only — if you persist with `ANSA_DATA` and restart, the log
replays but cursors reset, so agents will re-read past messages (use `since` to
skip them). Good enough for local multi-agent setups; not a durable broker.
