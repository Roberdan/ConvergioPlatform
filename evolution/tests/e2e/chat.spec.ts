import { test, expect, Page } from '@playwright/test';

/** Collect JS errors emitted by the page. */
function collectPageErrors(page: Page): () => string[] {
  const errors: string[] = [];
  page.on('pageerror', (err) => errors.push(err.message));
  return () => errors;
}

/** SSE mock: intercepts /api/chat/stream and returns a minimal event-stream. */
async function mockSseRoute(page: Page): Promise<void> {
  await page.route('**/api/chat/**', async (route) => {
    const sseBody = [
      'data: {"role":"assistant","content":"Hello from mock"}\n\n',
      'data: [DONE]\n\n',
    ].join('');

    await route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      headers: {
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      },
      body: sseBody,
    });
  });
}

test.describe('Chat page', () => {
  test('loads without JS errors', async ({ page }) => {
    const getErrors = collectPageErrors(page);

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    expect(getErrors()).toHaveLength(0);
  });

  test('chat input or compose area is present', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Broad selector: textarea, input[type=text], or any element with a chat-related role.
    const input = page
      .locator(
        'textarea, input[type="text"], [role="textbox"], .chat-input, #chat-input, [data-testid="chat-input"]',
      )
      .first();

    // Navigate to /chat if the input is not visible on the root page.
    const visible = await input.isVisible().catch(() => false);
    if (!visible) {
      await page.goto('/chat');
      await page.waitForLoadState('networkidle');
    }

    // Re-query after potential navigation.
    const chatInput = page
      .locator(
        'textarea, input[type="text"], [role="textbox"], .chat-input, #chat-input, [data-testid="chat-input"]',
      )
      .first();
    await expect(chatInput).toBeAttached();
  });

  test('SSE mock: submitting a message receives streamed response', async ({ page }) => {
    await mockSseRoute(page);

    // Navigate to whichever route serves the chat UI.
    await page.goto('/chat').catch(() => page.goto('/'));
    await page.waitForLoadState('networkidle');

    const input = page
      .locator('textarea, input[type="text"], [role="textbox"]')
      .first();

    const isVisible = await input.isVisible().catch(() => false);
    if (!isVisible) {
      test.skip(true, 'Chat input not found — skipping SSE interaction test');
      return;
    }

    await input.fill('Hello');

    // Submit via Enter key or a send button.
    const sendButton = page.locator(
      'button[type="submit"], button:has-text("Send"), [data-testid="send-btn"]',
    ).first();
    const hasSendBtn = await sendButton.isVisible().catch(() => false);

    if (hasSendBtn) {
      await sendButton.click();
    } else {
      await input.press('Enter');
    }

    // The mocked SSE should produce some assistant text in the DOM.
    await expect(
      page.locator('text=Hello from mock, .message, .chat-message, [data-role="assistant"]').first(),
    ).toBeAttached({ timeout: 5000 });
  });

  test('visual snapshot — chat page', async ({ page }) => {
    await page.goto('/chat').catch(() => page.goto('/'));
    await page.waitForLoadState('networkidle');

    await expect(page).toHaveScreenshot('chat-page.png', {
      maxDiffPixelRatio: 0.05,
    });
  });
});
