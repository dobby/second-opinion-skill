// popup.js
chrome.storage.local.get(['wsConnected'], (result) => {
  const connected = result.wsConnected === true;
  const dot = document.getElementById('dot');
  const label = document.getElementById('label');

  if (connected) {
    dot.className = 'dot connected';
    label.textContent = 'Connected to server';
  } else {
    dot.className = 'dot disconnected';
    label.textContent = 'Server not running';
  }
});
