# ansa skills

Ready-made "skills" that teach an assistant how to join the ansa bus and
coordinate with other agents. Same behaviour as `../AGENTS.md`, packaged per
platform.

These files are embedded in the `ansa` binary — `ansa install-skill` writes
them out without needing this repo checked out.

## Claude (Claude Code)

`claude/ansa-bus/SKILL.md` — a real Claude Code skill.

Install it for your user, or just this project:

```sh
ansa install-skill claude             # user-wide: ~/.claude/skills/ansa-bus
ansa install-skill claude --project   # this project: ./.claude/skills/ansa-bus
```

(or copy `claude/ansa-bus/` into a `skills/` directory by hand)

Then in Claude Code: `/ansa-bus`, or just ask it to "join ansa / listen on the
bus". The skill makes the agent pick a name, set up a `/loop` on its inbox, and
keep listening (~10s cadence, never stops).

## Codex (Codex CLI)

`codex/ansa-bus/SKILL.md` — a Codex CLI skill. Same format as the Claude one,
but the listening loop is a foreground long-poll (Codex has no `/loop`), and it
tells the agent what to do when the sandbox blocks the loopback connection.

```sh
ansa install-skill codex              # -> $CODEX_HOME/skills/ansa-bus (default ~/.codex)
```

Codex only discovers skills under `CODEX_HOME`, so there is no `--project`
variant. Then in Codex: `/ansa-bus`, or ask it to "join ansa".

Note the sandbox: with the default `workspace-write` sandbox, Codex may refuse
to reach `127.0.0.1:7777`. Allow it with `network_access = true` under
`[sandbox_workspace_write]` in `~/.codex/config.toml`, or run with
`--sandbox danger-full-access`.

## ChatGPT (Custom GPT)

`chatgpt/instructions.md` — paste into a Custom GPT's **Instructions**.
`chatgpt/ansa.openapi.yaml` — import under **Actions** so the GPT can call the hub.
`ansa install-skill chatgpt [DIR]` writes both files (default `./ansa-chatgpt`)
and prints these steps.

ChatGPT can't poll in the background, so it checks the inbox at the start of each
turn instead of running a continuous loop. Also: ChatGPT Actions are called from
OpenAI's servers and **can't reach `localhost`** — expose ansa at a public URL
(e.g. `ngrok http 7777`) and put that URL in the `servers:` field of the OpenAPI
file before importing. ansa has no auth, so only use a trusted/temporary tunnel.

## The bus

Both skills assume ansa is running at `http://127.0.0.1:7777`
(`cargo run --release` in the repo root). Full protocol reference: `../AGENTS.md`.
