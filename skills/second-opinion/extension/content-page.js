// content-page.js — MAIN world
// Has access to page's JavaScript context (TipTap el.editor, etc.)

const BRIDGE_TO_PAGE = 'SECOND_OPINION_TO_PAGE';
const PAGE_TO_BRIDGE = 'SECOND_OPINION_TO_BRIDGE';
const RESPONSE_TIMEOUT_MS = 60000;

window.addEventListener('message', async (event) => {
  if (event.source !== window) return;
  if (!event.data || event.data.source !== BRIDGE_TO_PAGE) return;

  const msg = event.data.payload;
  if (msg.type !== 'ask') return;

  try {
    const text = await askGrok(msg.message);
    window.postMessage({
      source: PAGE_TO_BRIDGE,
      payload: { type: 'response', id: msg.id, text }
    }, '*');
  } catch (e) {
    window.postMessage({
      source: PAGE_TO_BRIDGE,
      payload: { type: 'error', id: msg.id, error: e.message }
    }, '*');
  }
});

async function askGrok(message) {
  // Step 1: Find TipTap editor element
  const el = document.querySelector('div.tiptap.ProseMirror[contenteditable="true"]')
    || document.querySelector('.ProseMirror[contenteditable="true"]');

  if (!el) {
    throw new Error('Could not find TipTap editor on grok.com. Make sure grok.com is loaded.');
  }

  const editor = el.editor;
  if (!editor) {
    throw new Error('TipTap editor instance not found on element (el.editor is undefined). Must run in MAIN world.');
  }

  // Step 2: Record baseline response count before submitting
  const baseline = document.querySelectorAll('[id^="response-"]').length;

  // Step 3: Insert text via TipTap API
  editor.commands.focus();
  editor.commands.clearContent();
  editor.commands.insertContent(message);

  // Small delay to ensure React state updates
  await sleep(100);

  // Step 4: Submit the form
  const form = el.closest('form');
  if (!form) {
    throw new Error('Could not find form element wrapping the TipTap editor.');
  }

  try {
    form.requestSubmit();
  } catch (e) {
    // Fallback: find submit button and click
    const btn = form.querySelector('button[aria-label="Submit"][type="submit"]')
      || form.querySelector('button[type="submit"]');
    if (btn) {
      btn.removeAttribute('disabled');
      btn.click();
    } else {
      throw new Error('Could not submit the form: requestSubmit failed and no submit button found.');
    }
  }

  // Step 5: Wait for response using MutationObserver
  return await waitForResponse(baseline);
}

function waitForResponse(baseline) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      observer.disconnect();
      reject(new Error('timeout'));
    }, RESPONSE_TIMEOUT_MS);

    const observer = new MutationObserver(() => {
      const responses = document.querySelectorAll('[id^="response-"]');
      if (responses.length <= baseline) return;

      const latest = responses[responses.length - 1];
      const actionButtons = latest.querySelector('.action-buttons');

      // Streaming is complete when .action-buttons appears with timing text
      if (actionButtons && actionButtons.innerText && actionButtons.innerText.trim()) {
        observer.disconnect();
        clearTimeout(timeout);

        const messageBubble = latest.querySelector('.message-bubble');
        const text = messageBubble ? messageBubble.innerText.trim() : '';

        if (!text) {
          reject(new Error('Response container found but .message-bubble text is empty.'));
        } else {
          resolve(text);
        }
      }
    });

    observer.observe(document.body, { childList: true, subtree: true });
  });
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}
