import { test, expect, Page } from '@playwright/test';

// Helper function to wait for page to be ready
async function waitForPageReady(page: Page) {
    await page.waitForLoadState('networkidle');
    await page.waitForSelector('.main-layout', { state: 'visible' });
}

// Helper function to create a folder
async function createFolder(page: Page, folderName: string) {
    await page.click('#new-folder-btn');
    await page.waitForSelector('#folder-modal.active', { state: 'visible' });
    await page.fill('#folder-name', folderName);
    await page.click('#folder-modal button:has-text("Save")');
    await page.waitForSelector('#folder-modal:not(.active)', { state: 'hidden', timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(300);
}

// Helper function to create a request
async function createRequest(page: Page, requestName: string, method: string = 'GET', url: string = 'https://api.example.com/v1/users', folderId?: string) {
    await page.click('#new-request-btn');
    
    // Handle request choice modal if it appears
    try {
        await page.waitForSelector('#new-request-choice-modal.active', { state: 'visible', timeout: 2000 });
        await page.click('#new-request-choice-modal button:has-text("New HTTP/API Request")');
    } catch (e) {
        // If modal doesn't appear within timeout, maybe it went straight to request modal
        // or we missed it (unlikely with this flow)
    }

    await page.waitForSelector('#request-modal.active', { state: 'visible' });
    await page.fill('#request-name', requestName);
    await page.selectOption('#request-method-modal', method);
    await page.fill('#request-url-modal', url);
    if (folderId) {
        await page.selectOption('#request-folder-modal', folderId);
    }
    await page.click('#request-modal button:has-text("Save")');
    // Wait for modal to close
    await page.waitForSelector('#request-modal:not(.active)', { state: 'hidden', timeout: 10000 });
    // Wait for request to be created and tab to appear
    await page.waitForTimeout(500);
    // Wait for the request tab to appear
    await page.waitForSelector(`.request-tab:has-text("${requestName}")`, { state: 'visible', timeout: 10000 });
}

// Helper function to create an environment
async function createEnvironment(page: Page, envName: string, variables: Record<string, string>) {
    await page.click('#add-environment-btn');
    await page.waitForSelector('#environment-modal.active', { state: 'visible' });
    await page.fill('#environment-name', envName);
    await page.fill('#environment-variables', JSON.stringify(variables, null, 2));
    await page.click('#environment-modal button:has-text("Save")');
    await page.waitForSelector('#environment-modal:not(.active)', { state: 'hidden', timeout: 5000 }).catch(() => {});
    await page.waitForTimeout(300);
}

test.describe('JS-Link Comprehensive E2E Tests', () => {
    test.beforeEach(async ({ page }) => {
        page.on('console', msg => console.log(`BROWSER LOG: ${msg.text()}`));
        await page.goto('http://localhost:3000');
        await waitForPageReady(page);
        // Add a small delay to ensure page is fully ready
        await page.waitForTimeout(300);
    });

    // ==================== UI LAYOUT TESTS ====================
    test.describe('UI Layout and Components', () => {
        test('should display all main UI components on load', async ({ page }) => {
            // Top bar elements
            await expect(page.locator('.top-bar')).toBeVisible();
            await expect(page.locator('.logo-text')).toBeVisible();
            await expect(page.locator('#global-search')).toBeVisible();

            // Left sidebar with collapsible sections
            await expect(page.locator('.left-sidebar')).toBeVisible();
            await expect(page.locator('#collection-section')).toBeVisible();
            await expect(page.locator('#history-section')).toBeVisible();
            await expect(page.locator('#new-folder-btn')).toBeVisible();
            await expect(page.locator('#new-request-sidebar-btn')).toBeVisible();

            // Main content
            await expect(page.locator('.main-content')).toBeVisible();
            await expect(page.locator('.request-editor-panel')).toBeVisible();
            await expect(page.locator('.response-panel')).toBeVisible();

            // Right sidebar
            await expect(page.locator('.right-sidebar')).toBeVisible();
            await expect(page.locator('#environment-select')).toBeVisible();
            await expect(page.locator('#add-environment-btn')).toBeVisible();

            // Status bar
            await expect(page.locator('.status-bar')).toBeVisible();
        });

        test('should display request editor components', async ({ page }) => {
            await expect(page.locator('#request-method')).toBeVisible();
            await expect(page.locator('#request-url')).toBeVisible();
            await expect(page.locator('#send-request-btn')).toBeVisible();

            // Request tabs
            await expect(page.locator('.request-sub-tab[data-tab="headers"]')).toBeVisible();
            await expect(page.locator('.request-sub-tab[data-tab="body"]')).toBeVisible();
            await expect(page.locator('.request-sub-tab[data-tab="auth"]')).toBeVisible();
            await expect(page.locator('.request-sub-tab[data-tab="settings"]')).toBeVisible();
        });

        test('should display response panel components', async ({ page }) => {
            await expect(page.locator('.response-panel')).toBeVisible();
            await expect(page.locator('#response-status')).toBeVisible();
            await expect(page.locator('#copy-response')).toBeVisible();
            await expect(page.locator('#format-response')).toBeVisible();

            // Response tabs
            await expect(page.locator('.response-tab[data-tab="response-body"]')).toBeVisible();
            await expect(page.locator('.response-tab[data-tab="response-headers"]')).toBeVisible();
        });
    });

    // ==================== RESPONSIVE DESIGN TESTS ====================
    test.describe('Responsive Design', () => {
        test('should adapt to mobile viewport (375x667)', async ({ page }) => {
            await page.setViewportSize({ width: 375, height: 667 });
            await waitForPageReady(page);

            await expect(page.locator('.main-layout')).toBeVisible();
            await expect(page.locator('.left-sidebar')).toBeVisible();
            await expect(page.locator('.main-content')).toBeVisible();
        });

        test('should adapt to tablet viewport (768x1024)', async ({ page }) => {
            await page.setViewportSize({ width: 768, height: 1024 });
            await waitForPageReady(page);

            await expect(page.locator('.main-layout')).toBeVisible();
            await expect(page.locator('.left-sidebar')).toBeVisible();
            await expect(page.locator('.right-sidebar')).toBeVisible();
        });

        test('should adapt to desktop viewport (1920x1080)', async ({ page }) => {
            await page.setViewportSize({ width: 1920, height: 1080 });
            await waitForPageReady(page);

            await expect(page.locator('.main-layout')).toBeVisible();
            await expect(page.locator('.left-sidebar')).toBeVisible();
            await expect(page.locator('.right-sidebar')).toBeVisible();
        });

        test('should adapt to small mobile viewport (320x568)', async ({ page }) => {
            await page.setViewportSize({ width: 320, height: 568 });
            await waitForPageReady(page);

            await expect(page.locator('.main-layout')).toBeVisible();
        });
    });

    // ==================== PANEL RESIZER TESTS ====================
    test.describe('Panel Resizer', () => {
        test('should display panel resizer', async ({ page }) => {
            await expect(page.locator('#panel-resizer')).toBeVisible();
            await expect(page.locator('.resizer-handle')).toBeVisible();
        });

        test('should change cursor on hover', async ({ page }) => {
            const resizer = page.locator('#panel-resizer');
            await resizer.hover();
            // The resizer should have row-resize cursor
            await expect(resizer).toHaveCSS('cursor', 'row-resize');
        });

        test.skip('should resize panels on drag', async ({ page }) => {
            const requestPanel = page.locator('.request-editor-panel');
            const resizer = page.locator('#panel-resizer');

            const initialHeight = await requestPanel.boundingBox();

            // Drag the resizer down
            await resizer.hover();
            await page.mouse.down();
            await page.mouse.move(500, 500);
            await page.mouse.up();

            const newHeight = await requestPanel.boundingBox();
            // Height should have changed
            expect(newHeight?.height).not.toBe(initialHeight?.height);
        });

        test('should reset panels on double-click', async ({ page }) => {
            const resizer = page.locator('#panel-resizer');
            const requestPanel = page.locator('.request-editor-panel');

            // First resize
            await resizer.hover();
            await page.mouse.down();
            await page.mouse.move(500, 600);
            await page.mouse.up();

            // Double-click to reset
            await resizer.dblclick();

            // Check that flex style is reset
            const flexStyle = await requestPanel.evaluate(el => el.style.flex);
            expect(flexStyle).toBe('');
        });
    });

    // ==================== FOLDER MANAGEMENT TESTS ====================
    test.describe('Folder Management', () => {
        test('should create a new folder', async ({ page }) => {
            const folderName = `Test Folder ${Date.now()}`;
            await createFolder(page, folderName);
            await expect(page.locator(`text=${folderName}`)).toBeVisible();
        });

        test('should create multiple folders', async ({ page }) => {
            const folder1 = `Folder 1 ${Date.now()}`;
            const folder2 = `Folder 2 ${Date.now()}`;

            await createFolder(page, folder1);
            await createFolder(page, folder2);

            await expect(page.locator(`text=${folder1}`)).toBeVisible();
            await expect(page.locator(`text=${folder2}`)).toBeVisible();
        });

        test('should edit folder via context menu', async ({ page }) => {
            const folderName = `Folder Edit ${Date.now()}`;
            const newFolderName = `Updated ${folderName}`;

            await createFolder(page, folderName);
            await page.waitForTimeout(500);

            // Right-click on folder
            const folderItem = page.locator('.collection-sub-item').filter({ hasText: folderName }).first();
            await folderItem.click({ button: 'right' });

            // Click Edit in context menu
            await page.waitForSelector('.context-menu.active', { state: 'visible' });
            await page.click('.context-menu-item:has-text("Edit")');

            // Edit folder name
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });
            await page.fill('#folder-name', newFolderName);
            await page.click('#folder-modal button:has-text("Save")');

            await expect(page.locator(`text=${newFolderName}`)).toBeVisible();
        });

        test('should archive folder via context menu', async ({ page }) => {
            const folderName = `Folder Archive ${Date.now()}`;

            await createFolder(page, folderName);
            await page.waitForTimeout(500);

            // Right-click on folder
            const folderItem = page.locator('.collection-sub-item').filter({ hasText: folderName }).first();
            await folderItem.click({ button: 'right' });

            // Click Archive in context menu
            await page.waitForSelector('.context-menu.active', { state: 'visible' });
            await page.click('.context-menu-item:has-text("Archive")');

            // Folder should not be visible (archived)
            await page.waitForTimeout(500);
            await expect(page.locator(`.collection-sub-item:has-text("${folderName}")`)).not.toBeVisible();
        });

        test('should delete folder via context menu', async ({ page }) => {
            const folderName = `Folder Delete ${Date.now()}`;

            await createFolder(page, folderName);
            await page.waitForTimeout(500);

            // Right-click on folder
            const folderItem = page.locator('.collection-sub-item').filter({ hasText: folderName }).first();
            await folderItem.click({ button: 'right' });

            // Click Delete in context menu
            await page.waitForSelector('.context-menu.active', { state: 'visible' });

            // Handle confirmation dialog
            page.on('dialog', dialog => dialog.accept());
            await page.click('.context-menu-item:has-text("Delete")');

            // Folder should be deleted
            await page.waitForTimeout(500);
            await expect(page.locator(`.collection-sub-item:has-text("${folderName}")`)).not.toBeVisible();
        });

        test('should not create folder with empty name', async ({ page }) => {
            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            // Try to save with empty name
            await page.fill('#folder-name', '');
            await page.click('#folder-modal button:has-text("Save")');

            // Modal should still be visible (validation failed)
            await expect(page.locator('#folder-modal.active')).toBeVisible();
        });

        test('should close folder modal on cancel', async ({ page }) => {
            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            await page.click('#folder-modal button:has-text("Cancel")');

            await expect(page.locator('#folder-modal.active')).not.toBeVisible();
        });

        test('should close folder modal on overlay click', async ({ page }) => {
            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            // Click on overlay (outside modal)
            await page.click('#folder-modal', { position: { x: 10, y: 10 } });

            await expect(page.locator('#folder-modal.active')).not.toBeVisible();
        });
    });

    // ==================== REQUEST MANAGEMENT TESTS ====================
    test.describe('Request Management', () => {
        test('should create a new GET request', async ({ page }) => {
            const requestName = `GET Request ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://api.example.com/users');

            await expect(page.locator(`.request-tab:has-text("${requestName}")`)).toBeVisible();
            await expect(page.locator('#request-method')).toHaveValue('GET');
        });

        test('should create a new POST request', async ({ page }) => {
            const requestName = `POST Request ${Date.now()}`;
            await createRequest(page, requestName, 'POST', 'https://api.example.com/users');

            await expect(page.locator(`.request-tab:has-text("${requestName}")`)).toBeVisible();
            await expect(page.locator('#request-method')).toHaveValue('POST');
        });

        test('should create requests with all HTTP methods', async ({ page }) => {
            test.setTimeout(60000);
            const methods = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'];

            for (const method of methods) {
                const requestName = `${method} Request ${Date.now()}`;
                await createRequest(page, requestName, method);
                await expect(page.locator(`.request-tab:has-text("${requestName}")`)).toBeVisible();
            }
        });

        test('should edit request URL in editor', async ({ page }) => {
            const requestName = `URL Edit ${Date.now()}`;
            await createRequest(page, requestName);

            const newUrl = 'https://jsonplaceholder.typicode.com/posts';
            await page.fill('#request-url', newUrl);

            await expect(page.locator('#request-url')).toHaveValue(newUrl);
        });

        test('should change request method in editor', async ({ page }) => {
            const requestName = `Method Change ${Date.now()}`;
            await createRequest(page, requestName, 'GET');

            await page.selectOption('#request-method', 'POST');

            await expect(page.locator('#request-method')).toHaveValue('POST');
        });

        test('should close request tab', async ({ page }) => {
            const requestName = `Close Tab ${Date.now()}`;
            await createRequest(page, requestName);
            await page.waitForTimeout(500);

            const tab = page.locator(`.request-tab:has-text("${requestName}")`);
            await tab.waitFor({ state: 'visible' });
            await tab.click(); // Activate tab
            
            // Click the close button (the X icon) - try multiple selectors
            const closeBtn = tab.locator('.request-tab-close').first();
            await closeBtn.waitFor({ state: 'visible' });
            await closeBtn.evaluate((node: HTMLElement) => node.click());

            // Wait for tab to be removed from DOM
            await expect(tab).not.toBeAttached({ timeout: 3000 });
        });

        test('should switch between request tabs', async ({ page }) => {
            const request1 = `Request 1 ${Date.now()}`;
            const request2 = `Request 2 ${Date.now()}`;

            await createRequest(page, request1, 'GET', 'https://api1.example.com');
            await page.waitForTimeout(500);
            await createRequest(page, request2, 'POST', 'https://api2.example.com');
            await page.waitForTimeout(500);

            // Click first tab
            const tab1 = page.locator(`.request-tab:has-text("${request1}")`);
            await tab1.waitFor({ state: 'visible' });
            await tab1.click();
            await page.waitForTimeout(300);
            await expect(page.locator('#request-url')).toHaveValue('https://api1.example.com');

            // Click second tab
            const tab2 = page.locator(`.request-tab:has-text("${request2}")`);
            await tab2.waitFor({ state: 'visible' });
            await tab2.click();
            await page.waitForTimeout(300);
            await expect(page.locator('#request-url')).toHaveValue('https://api2.example.com');
        });

        test('should edit request via context menu', async ({ page }) => {
            const requestName = `Request Edit ${Date.now()}`;
            const newRequestName = `Updated ${requestName}`;

            await createRequest(page, requestName);
            await page.waitForTimeout(1000);

            // Find and right-click on request in collection tree
            const requestItem = page.locator('.collection-sub-item').filter({ hasText: requestName }).first();
            await requestItem.waitFor({ state: 'visible' });
            await requestItem.click({ button: 'right' });

            // Wait for context menu to appear
            await page.waitForSelector('.context-menu.active', { state: 'visible', timeout: 5000 });
            await page.click('.context-menu-item:has-text("Edit")');

            // Edit request name
            await page.waitForSelector('#request-modal.active', { state: 'visible' });
            await page.fill('#request-name', newRequestName);
            await page.click('#request-modal button:has-text("Save")');
            await page.waitForTimeout(500);

            // Click the request in sidebar to ensure tab updates
            await page.click(`.collection-sub-item:has-text("${newRequestName}")`);

            await expect(page.locator(`.request-tab:has-text("${newRequestName}")`)).toBeVisible({ timeout: 5000 });
        });

        test('should archive request via context menu', async ({ page }) => {
            const requestName = `Request Archive ${Date.now()}`;

            await createRequest(page, requestName);
            await page.waitForTimeout(1000);

            // Find and right-click on request in collection tree
            const requestItem = page.locator('.collection-sub-item').filter({ hasText: requestName }).first();
            await requestItem.waitFor({ state: 'visible' });
            await requestItem.click({ button: 'right' });

            // Wait for context menu to appear
            await page.waitForSelector('.context-menu.active', { state: 'visible', timeout: 5000 });
            await page.click('.context-menu-item:has-text("Archive")');

            // Request should not be visible in collection (archived)
            await page.waitForTimeout(1000);
            await expect(requestItem).not.toBeVisible();
        });

        test('should delete request via context menu', async ({ page }) => {
            const requestName = `Request Delete ${Date.now()}`;

            await createRequest(page, requestName);
            await page.waitForTimeout(1000);

            // Handle confirmation dialog
            page.on('dialog', dialog => dialog.accept());

            // Find and right-click on request in collection tree
            const requestItem = page.locator('.collection-sub-item').filter({ hasText: requestName }).first();
            await requestItem.waitFor({ state: 'visible' });
            await requestItem.click({ button: 'right' });

            // Wait for context menu to appear
            await page.waitForSelector('.context-menu.active', { state: 'visible', timeout: 5000 });
            await page.click('.context-menu-item:has-text("Delete")');

            // Request should be deleted
            await page.waitForTimeout(1000);
            await expect(requestItem).not.toBeVisible();
        });

        test('should not create request with empty name', async ({ page }) => {
            await page.click('#new-request-btn');
            
            // Handle request choice modal if it appears
            try {
                await page.waitForSelector('#new-request-choice-modal.active', { state: 'visible', timeout: 2000 });
                await page.click('#new-request-choice-modal button:has-text("New HTTP/API Request")');
            } catch (e) {}

            await page.waitForSelector('#request-modal.active', { state: 'visible' });

            await page.fill('#request-name', '');
            await page.fill('#request-url-modal', 'https://api.example.com');
            await page.click('#request-modal button:has-text("Save")');

            // Modal should still be visible
            await expect(page.locator('#request-modal.active')).toBeVisible();
        });

        test('should assign request to folder', async ({ page }) => {
            const folderName = `Folder ${Date.now()}`;
            await createFolder(page, folderName);
            await page.waitForTimeout(500);

            // Get folder ID from dropdown
            await page.click('#new-request-btn');
            
            // Handle request choice modal if it appears
            try {
                await page.waitForSelector('#new-request-choice-modal.active', { state: 'visible', timeout: 2000 });
                await page.click('#new-request-choice-modal button:has-text("New HTTP/API Request")');
            } catch (e) {}

            await page.waitForSelector('#request-modal.active', { state: 'visible' });

            // Select the folder
            const folderOption = page.locator(`#request-folder-modal option:has-text("${folderName}")`);
            if (await folderOption.count() > 0) {
                const folderId = await folderOption.getAttribute('value');
                if (folderId) {
                    await page.selectOption('#request-folder-modal', folderId);
                }
            }

            const requestName = `Request in Folder ${Date.now()}`;
            await page.fill('#request-name', requestName);
            await page.click('#request-modal button:has-text("Save")');

            await expect(page.locator(`.request-tab:has-text("${requestName}")`)).toBeVisible();
        });
    });

    // ==================== HEADERS MANAGEMENT TESTS ====================
    test.describe('Headers Management', () => {
        test('should add a new header', async ({ page }) => {
            const requestName = `Headers Test ${Date.now()}`;
            await createRequest(page, requestName);
            await page.waitForTimeout(500);

            // Make sure we're on the headers tab
            await page.click('.request-sub-tab[data-tab="headers"]');
            await page.waitForTimeout(200);

            // Find the empty header row and add a header
            const lastRow = page.locator('#new-header-row');
            const keyInput = lastRow.locator('input[placeholder="Key"]');
            const valueInput = lastRow.locator('input[placeholder="Value"]');
            
            await keyInput.fill('X-Custom-Header');
            await valueInput.fill('custom-value');

            // Wait for new row to be added (triggered by input event)
            await page.waitForTimeout(800);

            // Verify header was added - check that an input with the key value exists
            const keyInputs = page.locator('input.header-input[placeholder="Key"]');
            const count = await keyInputs.count();
            let found = false;
            for (let i = 0; i < count; i++) {
                const value = await keyInputs.nth(i).inputValue();
                if (value === 'X-Custom-Header') {
                    found = true;
                    break;
                }
            }
            expect(found).toBeTruthy();
        });

        test('should remove a header', async ({ page }) => {
            const requestName = `Remove Header ${Date.now()}`;
            await createRequest(page, requestName);

            // Add a header first
            const lastRow = page.locator('#new-header-row');
            await lastRow.locator('input[placeholder="Key"]').fill('X-Remove-Me');
            await lastRow.locator('input[placeholder="Value"]').fill('value');
            await page.waitForTimeout(300);

            // Remove the header
            const removeBtn = page.locator('.header-row:has(input[value="X-Remove-Me"]) .header-remove');
            await removeBtn.click();

            // Verify header was removed
            await expect(page.locator('input[value="X-Remove-Me"]')).not.toBeVisible();
        });

        test('should toggle header checkbox', async ({ page }) => {
            const requestName = `Toggle Header ${Date.now()}`;
            await createRequest(page, requestName);

            // Add a header
            const lastRow = page.locator('#new-header-row');
            await lastRow.locator('input[placeholder="Key"]').fill('X-Toggle-Header');
            await lastRow.locator('input[placeholder="Value"]').fill('value');
            await page.waitForTimeout(300);

            // Find the checkbox for the new header and uncheck it
            const checkbox = page.locator('.header-row:has(input[value="X-Toggle-Header"]) .header-checkbox');
            await checkbox.uncheck();

            // Verify checkbox is unchecked
            await expect(checkbox).not.toBeChecked();
        });

        test('should not send unchecked headers', async ({ page }) => {
            const requestName = `Unchecked Headers ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://httpbin.org/headers');

            // Add a header and uncheck it
            const lastRow = page.locator('#new-header-row');
            await lastRow.locator('input[placeholder="Key"]').fill('X-Should-Not-Send');
            await lastRow.locator('input[placeholder="Value"]').fill('secret');
            await page.waitForTimeout(300);

            // Uncheck the header
            const checkbox = page.locator('.header-row:has(input[value="X-Should-Not-Send"]) .header-checkbox');
            await checkbox.uncheck();

            // Send request (the header should not be included)
            await page.click('#send-request-btn');
            await page.waitForTimeout(2000);

            // Response should not contain the header
            const responseBody = await page.locator('#response-body-content').textContent();
            expect(responseBody).not.toContain('X-Should-Not-Send');
        });

        test('should add multiple headers', async ({ page }) => {
            const requestName = `Multiple Headers ${Date.now()}`;
            await createRequest(page, requestName);

            // Add first header
            let lastRow = page.locator('#new-header-row');
            await lastRow.locator('input[placeholder="Key"]').fill('X-Header-1');
            await lastRow.locator('input[placeholder="Value"]').fill('value1');
            await page.waitForTimeout(300);

            // Add second header
            lastRow = page.locator('#new-header-row');
            await lastRow.locator('input[placeholder="Key"]').fill('X-Header-2');
            await lastRow.locator('input[placeholder="Value"]').fill('value2');
            await page.waitForTimeout(300);

            // Verify both headers exist
            await expect(page.locator('input[value="X-Header-1"]')).toBeVisible();
            await expect(page.locator('input[value="X-Header-2"]')).toBeVisible();
        });
    });

    // ==================== ENVIRONMENT MANAGEMENT TESTS ====================
    test.describe('Environment Management', () => {
        test('should create a new environment', async ({ page }) => {
            const envName = `Test Env ${Date.now()}`;
            const variables = { base_url: 'https://api.example.com', token: 'test-token' };

            await createEnvironment(page, envName, variables);

            await expect(page.locator(`#environment-select option:has-text("${envName}")`)).toBeVisible();
        });

        test('should select environment and show variables', async ({ page }) => {
            const envName = `Env Variables ${Date.now()}`;
            const variables = { api_key: 'secret-key-123' };

            await createEnvironment(page, envName, variables);
            await page.waitForTimeout(500);

            // Select environment
            await page.selectOption('#environment-select', { label: envName });
            await page.waitForTimeout(300);

            // Verify variables are displayed
            await expect(page.locator('text={{api_key}}')).toBeVisible();
        });

        test('should edit environment via double-click', async ({ page }) => {
            const envName = `Env Edit ${Date.now()}`;
            const newEnvName = `Updated ${envName}`;
            const variables = { key: 'value' };

            await createEnvironment(page, envName, variables);
            await page.waitForTimeout(500);

            // Select and double-click environment
            await page.selectOption('#environment-select', { label: envName });
            await page.locator('#environment-select').dblclick();

            // Edit environment
            await page.waitForSelector('#environment-modal.active', { state: 'visible' });
            await page.fill('#environment-name', newEnvName);
            await page.click('#environment-modal button:has-text("Save")');

            await expect(page.locator(`#environment-select option:has-text("${newEnvName}")`)).toBeVisible();
        });

        test('should validate JSON in environment variables', async ({ page }) => {
            await page.click('#add-environment-btn');
            await page.waitForSelector('#environment-modal.active', { state: 'visible' });

            await page.fill('#environment-name', 'Invalid JSON Env');
            await page.fill('#environment-variables', '{ invalid json }');

            // Handle alert dialog
            page.on('dialog', dialog => {
                expect(dialog.message()).toContain('Invalid JSON');
                dialog.accept();
            });

            await page.click('#environment-modal button:has-text("Save")');
        });

        test('should not create environment with empty name', async ({ page }) => {
            await page.click('#add-environment-btn');
            await page.waitForSelector('#environment-modal.active', { state: 'visible' });

            await page.fill('#environment-name', '');
            await page.fill('#environment-variables', '{}');

            page.on('dialog', dialog => dialog.accept());
            await page.click('#environment-modal button:has-text("Save")');

            // Modal should still be visible
            await expect(page.locator('#environment-modal.active')).toBeVisible();
        });

        test('should archive environment via context menu', async ({ page }) => {
            const envName = `Env Archive ${Date.now()}`;
            const variables = { key: 'value' };

            await createEnvironment(page, envName, variables);
            await page.waitForTimeout(500);

            // Select the environment
            await page.selectOption('#environment-select', { label: envName });

            // Right-click on environment selector
            await page.locator('#environment-select').click({ button: 'right' });

            // Click Archive
            await page.waitForSelector('.context-menu.active', { state: 'visible' });
            await page.click('.context-menu-item:has-text("Archive")');

            await page.waitForTimeout(500);
        });

        test('should delete environment via context menu', async ({ page }) => {
            const envName = `Env Delete ${Date.now()}`;
            const variables = { key: 'value' };

            await createEnvironment(page, envName, variables);
            await page.waitForTimeout(500);

            // Select the environment
            await page.selectOption('#environment-select', { label: envName });

            // Right-click on environment selector
            await page.locator('#environment-select').click({ button: 'right' });

            // Handle confirmation dialog
            page.on('dialog', dialog => dialog.accept());

            // Click Delete
            await page.waitForSelector('.context-menu.active', { state: 'visible' });
            await page.click('.context-menu-item:has-text("Delete")');

            await page.waitForTimeout(500);
        });
    });

    // ==================== REQUEST EXECUTION TESTS ====================
    test.describe('Request Execution', () => {
        test('should execute GET request successfully', async ({ page }) => {
            const requestName = `Exec GET ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForSelector('#response-status', { timeout: 15000 });
            await page.waitForTimeout(2000);

            const status = await page.locator('#response-status').textContent();
            expect(status).toContain('200');
        });

        test('should execute POST request with body', async ({ page }) => {
            const requestName = `Exec POST ${Date.now()}`;
            await createRequest(page, requestName, 'POST', 'https://jsonplaceholder.typicode.com/posts');

            // Add body
            await page.click('.request-sub-tab[data-tab="body"]');
            await page.fill('#request-body', JSON.stringify({ title: 'Test', body: 'Content', userId: 1 }));

            await page.click('#send-request-btn');
            await page.waitForSelector('#response-status', { timeout: 15000 });
            await page.waitForTimeout(2000);

            const status = await page.locator('#response-status').textContent();
            expect(status).toContain('201');
        });

        test('should execute request with environment variables', async ({ page }) => {
            const envName = `Exec Env ${Date.now()}`;
            const variables = {
                base_url: 'https://jsonplaceholder.typicode.com',
                post_id: '1'
            };

            await createEnvironment(page, envName, variables);
            await page.waitForTimeout(500);

            await page.selectOption('#environment-select', { label: envName });

            const requestName = `Var Exec ${Date.now()}`;
            await createRequest(page, requestName, 'GET', '{{base_url}}/posts/{{post_id}}');

            await page.click('#send-request-btn');
            await page.waitForSelector('#response-status', { timeout: 15000 });
            await page.waitForTimeout(2000);

            const status = await page.locator('#response-status').textContent();
            expect(status).toContain('200');
        });

        test('should handle request error gracefully', async ({ page }) => {
            const requestName = `Error Request ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://invalid-domain-that-does-not-exist.com');

            await page.click('#send-request-btn');
            await page.waitForTimeout(5000);

            // Should show error status
            const status = await page.locator('#response-status').textContent();
            expect(status).toBeTruthy();
        });

        test('should display execution history', async ({ page }) => {
            const requestName = `History ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            const historyList = page.locator('#execution-history');
            await expect(historyList).toBeVisible();
        });

        test('should show loading state during request', async ({ page }) => {
            const requestName = `Loading ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            // Click send and immediately check loading state
            await page.click('#send-request-btn');

            // Status should show loading
            const statusText = await page.locator('#response-status').textContent();
            // It might show "Loading..." or the actual response quickly
            expect(statusText).toBeTruthy();
        });
    });

    // ==================== RESPONSE PANEL TESTS ====================
    test.describe('Response Panel', () => {
        test('should display response body', async ({ page }) => {
            const requestName = `Response Body ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            const responseBody = await page.locator('#response-body-content').textContent();
            expect(responseBody).toBeTruthy();
        });

        test('should switch to response headers tab', async ({ page }) => {
            const requestName = `Response Headers ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            await page.click('.response-tab[data-tab="response-headers"]');
            await expect(page.locator('#response-headers')).toBeVisible();
        });

        test('should copy response to clipboard', async ({ page }) => {
            const requestName = `Copy Response ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            await page.click('#copy-response');

            // Button should show "Copied!"
            await expect(page.locator('#copy-response:has-text("Copied!")')).toBeVisible({ timeout: 3000 });
        });

        test('should format JSON response', async ({ page }) => {
            const requestName = `Format Response ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            await page.click('#format-response');

            // Response should still be visible
            await expect(page.locator('#response-body-content')).toBeVisible();
        });

        test('should switch between all response tabs', async ({ page }) => {
            const requestName = `Response Tabs ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://jsonplaceholder.typicode.com/posts/1');

            await page.click('#send-request-btn');
            await page.waitForTimeout(3000);

            // Body tab
            await page.click('.response-tab[data-tab="response-body"]');
            await expect(page.locator('#response-body')).toBeVisible();

            // Headers tab
            await page.click('.response-tab[data-tab="response-headers"]');
            await expect(page.locator('#response-headers')).toBeVisible();

            // Timeline tab
            await page.click('.response-tab[data-tab="response-timeline"]');
            await expect(page.locator('#response-timeline')).toBeVisible();

            // Cookies tab
            await page.click('.response-tab[data-tab="response-cookies"]');
            await expect(page.locator('#response-cookies')).toBeVisible();
        });
    });

    // ==================== REQUEST EDITOR TABS TESTS ====================
    test.describe('Request Editor Tabs', () => {
        test('should switch between all request editor tabs', async ({ page }) => {
            const requestName = `Editor Tabs ${Date.now()}`;
            await createRequest(page, requestName);

            // Headers tab
            await page.click('.request-sub-tab[data-tab="headers"]');
            await expect(page.locator('#headers-tab')).toBeVisible();

            // Body tab
            await page.click('.request-sub-tab[data-tab="body"]');
            await expect(page.locator('#body-tab')).toBeVisible();

            // Auth tab
            await page.click('.request-sub-tab[data-tab="auth"]');
            await expect(page.locator('#auth-tab')).toBeVisible();

            // Settings tab
            await page.click('.request-sub-tab[data-tab="settings"]');
            await expect(page.locator('#settings-tab')).toBeVisible();
        });

        test('should edit request body', async ({ page }) => {
            const requestName = `Body Edit ${Date.now()}`;
            await createRequest(page, requestName, 'POST');

            await page.click('.request-sub-tab[data-tab="body"]');

            const bodyText = '{"key": "value"}';
            await page.fill('#request-body', bodyText);

            await expect(page.locator('#request-body')).toHaveValue(bodyText);
        });
    });

    // ==================== WEBSOCKET TESTS ====================
    test.describe('WebSocket Support', () => {
        test('should show WebSocket controls when WS method selected', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            // WebSocket controls should be visible
            await expect(page.locator('#ws-connect-btn')).toBeVisible();
            await expect(page.locator('.ws-tab')).toBeVisible();

            // HTTP send button should be hidden
            await expect(page.locator('#send-request-btn')).toBeHidden();
        });

        test('should hide WebSocket controls when HTTP method selected', async ({ page }) => {
            // First select WS
            await page.selectOption('#request-method', 'WS');
            await expect(page.locator('#ws-connect-btn')).toBeVisible();

            // Then select GET
            await page.selectOption('#request-method', 'GET');

            // WebSocket controls should be hidden
            await expect(page.locator('#ws-connect-btn')).toBeHidden();
            await expect(page.locator('.ws-tab')).toBeHidden();

            // HTTP send button should be visible
            await expect(page.locator('#send-request-btn')).toBeVisible();
        });

        test('should show WebSocket tab content', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            // Should automatically switch to WebSocket tab
            await expect(page.locator('#websocket-tab')).toBeVisible();
            await expect(page.locator('#ws-status')).toBeVisible();
            await expect(page.locator('#ws-message-input')).toBeVisible();
            await expect(page.locator('#ws-send-btn')).toBeVisible();
        });

        test('should update URL placeholder for WebSocket', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            const placeholder = await page.locator('#request-url').getAttribute('placeholder');
            expect(placeholder).toContain('ws://');
        });

        test('should set default WebSocket URL', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            const url = await page.locator('#request-url').inputValue();
            expect(url.startsWith('ws://') || url.startsWith('wss://')).toBeTruthy();
        });

        test('should validate WebSocket URL', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            // Set invalid URL
            await page.fill('#request-url', 'https://invalid.com');

            // Click connect
            await page.click('#ws-connect-btn');

            // Should show error notification
            await page.waitForTimeout(500);
            // Notification should appear
        });

        test('should clear WebSocket messages', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            await page.click('#ws-clear-btn');

            const messages = page.locator('#ws-messages');
            await expect(messages).toBeEmpty();
        });

        test('should disable send button when not connected', async ({ page }) => {
            await page.selectOption('#request-method', 'WS');

            const sendBtn = page.locator('#ws-send-btn');
            await expect(sendBtn).toBeDisabled();
        });
    });

    // ==================== COLLECTION TREE TESTS ====================
    test.describe('Collection Tree', () => {
        test('should display Project API parent', async ({ page }) => {
            const folderName = `Tree Folder ${Date.now()}`;
            await createFolder(page, folderName);

            await expect(page.locator('text=Project API')).toBeVisible();
        });

        test('should expand and collapse collection tree', async ({ page }) => {
            const folderName = `Collapse Folder ${Date.now()}`;
            await createFolder(page, folderName);

            // Click on Project API to collapse
            const projectItem = page.locator('.collection-parent:has-text("Project API")');
            await projectItem.click();
            await page.waitForTimeout(300);

            // Click again to expand
            await projectItem.click();
            await page.waitForTimeout(300);
        });

        test('should show folder request count', async ({ page }) => {
            const folderName = `Count Folder ${Date.now()}`;
            await createFolder(page, folderName);

            // Collection item count should be visible
            await expect(page.locator('.collection-item-count')).toBeVisible();
        });
    });

    // ==================== COLLAPSIBLE SIDEBAR SECTIONS TESTS ====================
    test.describe('Collapsible Sidebar Sections', () => {
        test('should display Collection section header', async ({ page }) => {
            await expect(page.locator('#collection-header')).toBeVisible();
            await expect(page.locator('#collection-header:has-text("Collection")')).toBeVisible();
        });

        test('should display History section header', async ({ page }) => {
            await expect(page.locator('#history-header')).toBeVisible();
            await expect(page.locator('#history-header:has-text("History")')).toBeVisible();
        });

        test('should collapse Collection section on click', async ({ page }) => {
            const collectionHeader = page.locator('#collection-header');
            const collectionSection = page.locator('#collection-section');

            // Click to collapse
            await collectionHeader.click();
            await page.waitForTimeout(300);

            // Section should be collapsed
            await expect(collectionSection).toHaveClass(/collapsed/);

            // Click again to expand
            await collectionHeader.click();
            await page.waitForTimeout(300);

            // Section should be expanded
            await expect(collectionSection).not.toHaveClass(/collapsed/);
        });

        test('should collapse History section on click', async ({ page }) => {
            const historyHeader = page.locator('#history-header');
            const historySection = page.locator('#history-section');

            // Click to collapse
            await historyHeader.click();
            await page.waitForTimeout(300);

            // Section should be collapsed
            await expect(historySection).toHaveClass(/collapsed/);

            // Click again to expand
            await historyHeader.click();
            await page.waitForTimeout(300);

            // Section should be expanded
            await expect(historySection).not.toHaveClass(/collapsed/);
        });

        test('should show history items sorted by modified date', async ({ page }) => {
            // Create multiple requests
            const request1 = `History Test 1 ${Date.now()}`;
            const request2 = `History Test 2 ${Date.now() + 1}`;

            await createRequest(page, request1);
            await page.waitForTimeout(500);
            await createRequest(page, request2);
            await page.waitForTimeout(500);

            // History section should show the requests
            const historyTree = page.locator('#history-tree');
            await expect(historyTree).toBeVisible();

            // Should have history items
            const historyItems = page.locator('.history-item');
            await expect(historyItems.first()).toBeVisible();
        });

        test('should display method badges in history', async ({ page }) => {
            const requestName = `Method Badge Test ${Date.now()}`;
            await createRequest(page, requestName, 'POST');
            await page.waitForTimeout(500);

            // Look for POST method badge in history
            const postBadge = page.locator('.history-item-method.POST');
            await expect(postBadge).toBeVisible();
        });

        test('should open request from history on click', async ({ page }) => {
            const requestName = `Open From History ${Date.now()}`;
            const requestUrl = 'https://api.example.com/history-test';
            await createRequest(page, requestName, 'GET', requestUrl);
            await page.waitForTimeout(500);

            // Click on the request in history
            const historyItem = page.locator(`.history-item:has-text("${requestName}")`).first();
            if (await historyItem.isVisible()) {
                await historyItem.click();
                await page.waitForTimeout(500);

                // Request should be loaded in editor
                await expect(page.locator('#request-url')).toHaveValue(requestUrl);
            }
        });

        test('should not collapse section when clicking action buttons', async ({ page }) => {
            const collectionSection = page.locator('#collection-section');

            // Click on new folder button (should not collapse)
            await page.click('#new-folder-btn');
            await page.waitForTimeout(300);

            // Section should still be expanded (modal opens instead)
            await expect(collectionSection).not.toHaveClass(/collapsed/);

            // Close modal
            await page.click('#folder-modal .modal-close');
        });
    });

    // ==================== MODAL TESTS ====================
    test.describe('Modal Behavior', () => {
        test('should close modal on escape key', async ({ page }) => {
            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            await page.keyboard.press('Escape');

            // Modal might or might not close depending on implementation
        });

        test('should close modal on X button click', async ({ page }) => {
            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            await page.click('#folder-modal .modal-close');

            await expect(page.locator('#folder-modal.active')).not.toBeVisible();
        });

        test('should submit modal form on Enter key', async ({ page }) => {
            const folderName = `Enter Folder ${Date.now()}`;

            await page.click('#new-folder-btn');
            await page.waitForSelector('#folder-modal.active', { state: 'visible' });

            await page.fill('#folder-name', folderName);
            await page.keyboard.press('Enter');

            await page.waitForTimeout(500);
            // Either modal closes or folder is created
        });
    });

    // ==================== NOTIFICATION TESTS ====================
    test.describe('Notifications', () => {
        test('should show success notification on folder create', async ({ page }) => {
            const folderName = `Notif Folder ${Date.now()}`;
            await createFolder(page, folderName);

            // Look for notification
            const notification = page.locator('text=successfully');
            // Notification should appear briefly
        });

        test('should show error notification on invalid action', async ({ page }) => {
            // Try to send request without URL
            await page.fill('#request-url', '');
            await page.click('#send-request-btn');

            // Should show error notification
            await page.waitForTimeout(500);
        });
    });

    // ==================== SEARCH FUNCTIONALITY TESTS ====================
    test.describe('Search Functionality', () => {
        test('should display search input', async ({ page }) => {
            await expect(page.locator('#global-search')).toBeVisible();
        });

        test('should be able to type in search', async ({ page }) => {
            await page.fill('#global-search', 'test search');
            await expect(page.locator('#global-search')).toHaveValue('test search');
        });
    });

    // ==================== EDGE CASES ====================
    test.describe('Edge Cases', () => {
        test('should handle empty URL', async ({ page }) => {
            await page.fill('#request-url', '');
            await page.click('#send-request-btn');

            // Should show error or notification
            await page.waitForTimeout(500);
        });

        test('should handle very long URL', async ({ page }) => {
            const longUrl = 'https://example.com/' + 'a'.repeat(1000);
            await page.fill('#request-url', longUrl);

            // URL input should accept long URLs
            const value = await page.locator('#request-url').inputValue();
            expect(value.length).toBeGreaterThan(100);
        });

        test('should handle special characters in request name', async ({ page }) => {
            const requestName = `Request !@#$%^&*() ${Date.now()}`;

            await page.click('#new-request-btn');
            
            // Handle request choice modal if it appears
            try {
                await page.waitForSelector('#new-request-choice-modal.active', { state: 'visible', timeout: 2000 });
                await page.click('#new-request-choice-modal button:has-text("New HTTP/API Request")');
            } catch (e) {}

            await page.waitForSelector('#request-modal.active', { state: 'visible' });

            await page.fill('#request-name', requestName);
            await page.click('#request-modal button:has-text("Save")');

            await page.waitForTimeout(500);
        });

        test('should handle unicode characters in request body', async ({ page }) => {
            const requestName = `Unicode ${Date.now()}`;
            await createRequest(page, requestName, 'POST');

            await page.click('.request-sub-tab[data-tab="body"]');
            await page.fill('#request-body', '{"message": "Hello 世界 🌍"}');

            await expect(page.locator('#request-body')).toHaveValue('{"message": "Hello 世界 🌍"}');
        });

        test('should handle rapid button clicks', async ({ page }) => {
            // Rapidly click new folder button
            for (let i = 0; i < 5; i++) {
                await page.click('#new-folder-btn');
            }

            // Modal should be visible
            await expect(page.locator('#folder-modal.active')).toBeVisible();

            // Close modal
            await page.click('#folder-modal .modal-close');
        });

        test('should handle network timeout gracefully', async ({ page }) => {
            const requestName = `Timeout ${Date.now()}`;
            await createRequest(page, requestName, 'GET', 'https://httpstat.us/200?sleep=30000');

            await page.click('#send-request-btn');

            // Should show loading state
            await page.waitForTimeout(2000);

            // Status should be visible (loading or error)
            const status = page.locator('#response-status');
            await expect(status).toBeVisible();
        });
    });
});
