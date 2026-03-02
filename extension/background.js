// background.js — Service Worker
let ws = null;
let extensionConnected = false;
const RECONNECT_DELAY_MS = 3000;

async function getPort() {
  return new Promise((resolve) => {
    chrome.storage.local.get(['port'], (result) => {
      resolve(result.port || 7878);
    });
  });
}

function setStatus(connected) {
  extensionConnected = connected;
  chrome.storage.local.set({ wsConnected: connected });
}

async function connectWS() {
  const port = await getPort();
  const url = `ws://localhost:${port}`;

  try {
    ws = new WebSocket(url);
  } catch (e) {
    scheduleReconnect();
    return;
  }

  ws.onopen = () => {
    setStatus(true);
    ws.send(JSON.stringify({ type: 'extension_ready' }));
  };

  ws.onclose = () => {
    setStatus(false);
    ws = null;
    scheduleReconnect();
  };

  ws.onerror = () => {
    // onclose will fire after onerror
  };

  ws.onmessage = async (event) => {
    let msg;
    try {
      msg = JSON.parse(event.data);
    } catch (e) {
      return;
    }

    if (msg.type === 'ask') {
      await handleAsk(msg);
    }
  };
}

function scheduleReconnect() {
  setTimeout(connectWS, RECONNECT_DELAY_MS);
}

async function handleAsk(msg) {
  // Find a grok.com tab
  const tabs = await chrome.tabs.query({ url: 'https://grok.com/*' });
  if (!tabs || tabs.length === 0) {
    sendError(msg.id, 'No grok.com tab found. Please open grok.com in Chrome.');
    return;
  }

  const tab = tabs[0];

  // Send to content script and await response
  try {
    const response = await chrome.tabs.sendMessage(tab.id, msg);
    if (response && response.type === 'response') {
      ws.send(JSON.stringify({ type: 'response', id: msg.id, text: response.text }));
    } else if (response && response.type === 'error') {
      sendError(msg.id, response.error);
    } else {
      sendError(msg.id, 'Unexpected response from content script');
    }
  } catch (e) {
    sendError(msg.id, `Content script error: ${e.message}`);
  }
}

function sendError(id, error) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify({ type: 'error', id, error }));
  }
}

// Start connecting
connectWS();
