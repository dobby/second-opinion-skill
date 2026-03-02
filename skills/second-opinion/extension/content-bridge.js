// content-bridge.js — ISOLATED world
// Receives messages from background.js (chrome.runtime)
// Relays to page world via window.postMessage
// Receives responses from page world via window.addEventListener('message')

const BRIDGE_TO_PAGE = 'SECOND_OPINION_TO_PAGE';
const PAGE_TO_BRIDGE = 'SECOND_OPINION_TO_BRIDGE';

// Listen for messages from background service worker
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type !== 'ask') return false;

  // Forward to MAIN world
  window.postMessage({ source: BRIDGE_TO_PAGE, payload: msg }, '*');

  // Wait for response from MAIN world
  const handler = (event) => {
    if (event.source !== window) return;
    if (!event.data || event.data.source !== PAGE_TO_BRIDGE) return;
    if (event.data.payload.id !== msg.id) return;

    window.removeEventListener('message', handler);
    sendResponse(event.data.payload);
  };

  window.addEventListener('message', handler);

  // Return true to indicate async response
  return true;
});
