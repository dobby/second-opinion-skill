---
name: second-opinion
description: Get a second opinion from Grok AI on plans, code, architecture, or any question. Back-and-forth conversation with Grok through the user's existing browser session. Requires Chrome extension installed and grok.com open in Chrome.
---

## Prerequisites
- Chrome extension installed (load unpacked from `extension/` in this skill directory)
- grok.com must be open in Chrome with an active logged-in session

## Usage

### Start the server (always do this first)
```bash
./scripts/second-opinion start
```
Check that it's running:
```bash
./scripts/second-opinion status
```
If `extension_connected` is false, the user needs to open grok.com in Chrome.

### Ask a question
```bash
./scripts/second-opinion ask "Your question here. Include full context (code, plans, errors)."
```
Grok's response is printed to stdout.

### Continue the conversation
Call `ask` again with follow-up questions. Grok continues in the same browser conversation thread unless the user starts a new chat.

### Stop when done
```bash
./scripts/second-opinion stop
```
Always stop the server when you no longer need it.

## Configuration
Optional: create `.agents/second-opinion/second-opinion.toml` in your project directory:
```toml
port = 7878
timeout_secs = 60
```

## Error codes
- Exit 1: Server not running (call `start` first)
- Exit 2: Extension not connected (open grok.com in Chrome)
- Exit 3: Timeout waiting for Grok response
