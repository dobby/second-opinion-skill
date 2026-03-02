// popup.js
function render(connected) {
  const dot = document.getElementById('dot');
  const label = document.getElementById('label');

  if (connected) {
    dot.className = 'dot connected';
    label.textContent = 'Connected to server';
  } else {
    dot.className = 'dot disconnected';
    label.textContent = 'Server not running';
  }
}

// Initial read
chrome.storage.local.get(['wsConnected'], (result) => {
  render(result.wsConnected === true);
});

// Live updates while popup is open
chrome.storage.onChanged.addListener((changes) => {
  if ('wsConnected' in changes) {
    render(changes.wsConnected.newValue === true);
  }
});

// Wake the background service worker so it attempts a reconnect immediately
chrome.runtime.sendMessage({ type: 'wake' }).catch(() => {});
