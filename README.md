# Second Opinion Skill

A Claude Code skill that lets AI agents get a second opinion from Grok AI — back-and-forth conversation through the user's existing browser session.

## How It Works

```
Agent (via SKILL.md)
    │
    ▼
second-opinion CLI (Rust binary)
    │ WebSocket (localhost)
    ▼
WS Server (daemon)
    │ WebSocket
    ▼
Chrome Extension (background.js)
    │ chrome.tabs.sendMessage
    ▼
content.js (on grok.com)
    │ TipTap DOM manipulation
    ▼
grok.com → Grok AI → response
```

## Installation

### Install via skills.sh

```bash
npx skills add dobby/second-opinion-skill
```

### Manual Installation

### 1. Install the Chrome Extension

1. Open Chrome → `chrome://extensions`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select the `extension/` directory from this skill

### 2. Open grok.com

Log in to [grok.com](https://grok.com) in Chrome and keep the tab open.

## Usage (as an Agent)

Start the server:
```bash
./scripts/second-opinion start
./scripts/second-opinion status
# {"running": true, "port": 7878, "extension_connected": true}
```

Ask a question:
```bash
./scripts/second-opinion ask "Is this database schema well-normalized? [paste schema]"
```

Follow-up:
```bash
./scripts/second-opinion ask "What about adding an index on the email column?"
```

Stop when done:
```bash
./scripts/second-opinion stop
```

## Configuration

Optional — create `.agents/second-opinion/second-opinion.toml`:
```toml
port = 7878
timeout_secs = 60
```

## Error Codes

| Exit Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Server not running |
| 2 | Extension not connected (open grok.com) |
| 3 | Timeout waiting for response |

## Releasing

Tag a version to trigger the GitHub Actions workflow, which builds binaries for all platforms and commits them back to the repo:

```bash
git tag v0.1.0
git push origin v0.1.0
```
