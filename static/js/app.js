// Global state
let currentRequestId = null;
let currentEnvironmentId = null;
let folders = [];
let requests = [];
let environments = [];
let executionHistory = [];
let showArchived = false;
let wsConnection = null;
let wsConnected = false;
let pendingImportFile = null;

// Initialize app
document.addEventListener('DOMContentLoaded', () => {
    loadFolders();
    loadRequests();
    loadEnvironments();
    loadExecutionHistory();
    setupEventListeners();
    setupCollapsibleSections();
    setupTheme();
    setupEnvironmentToggle();
    setupLayoutToggle();
    // Initialize variable preview with defaults
    setTimeout(() => {
        updateVariablePreview();
        buildHistoryTree();
    }, 500);
});

// Setup theme
function setupTheme() {
    const themeToggleBtn = document.getElementById('theme-toggle-btn');
    const body = document.body;
    const icon = themeToggleBtn.querySelector('i');

    const applyTheme = (theme) => {
        if (theme === 'light') {
            body.classList.add('light-mode');
            icon.classList.remove('fa-sun');
            icon.classList.add('fa-moon');
        } else {
            body.classList.remove('light-mode');
            icon.classList.remove('fa-moon');
            icon.classList.add('fa-sun');
        }
    };

    let currentTheme = localStorage.getItem('theme') || 'dark';
    applyTheme(currentTheme);

    themeToggleBtn.addEventListener('click', () => {
        currentTheme = body.classList.contains('light-mode') ? 'dark' : 'light';
        applyTheme(currentTheme);
        localStorage.setItem('theme', currentTheme);
    });
}

// Setup environment panel toggle
function setupEnvironmentToggle() {
    const envToggleBtn = document.getElementById('environment-toggle-btn');
    const rightSidebar = document.querySelector('.right-sidebar');
    const icon = envToggleBtn.querySelector('i');

    // Load saved state from localStorage
    let envVisible = localStorage.getItem('envPanelVisible') !== 'false';

    const applyState = (visible) => {
        if (visible) {
            rightSidebar.classList.remove('hidden');
            icon.classList.remove('fa-cog');
            icon.classList.add('fa-cog');
        } else {
            rightSidebar.classList.add('hidden');
            icon.classList.remove('fa-cog');
            icon.classList.add('fa-cog');
        }
    };

    applyState(envVisible);

    envToggleBtn.addEventListener('click', () => {
        envVisible = !envVisible;
        applyState(envVisible);
        localStorage.setItem('envPanelVisible', envVisible);
    });
}

// Setup layout toggle (horizontal/vertical)
function setupLayoutToggle() {
    const layoutToggleBtn = document.getElementById('layout-toggle-btn');
    const mainLayout = document.querySelector('.main-layout');
    const icon = layoutToggleBtn.querySelector('i');

    // Load saved state from localStorage
    let isVertical = localStorage.getItem('layoutMode') === 'vertical';

    const applyLayout = (vertical) => {
        if (vertical) {
            mainLayout.classList.add('vertical-layout');
            icon.classList.remove('fa-columns');
            icon.classList.add('fa-bars');
        } else {
            mainLayout.classList.remove('vertical-layout');
            icon.classList.remove('fa-bars');
            icon.classList.add('fa-columns');
        }
    };

    applyLayout(isVertical);

    layoutToggleBtn.addEventListener('click', () => {
        isVertical = !isVertical;
        applyLayout(isVertical);
        localStorage.setItem('layoutMode', isVertical ? 'vertical' : 'horizontal');

        // Reset panel sizes when switching layout
        const requestPanel = document.querySelector('.request-editor-panel');
        const responsePanel = document.querySelector('.response-panel');
        if (requestPanel && responsePanel) {
            requestPanel.style.flex = '1';
            requestPanel.style.width = '';
            requestPanel.style.height = '';
            responsePanel.style.flex = '1';
            responsePanel.style.width = '';
            responsePanel.style.height = '';
        }
    });
}


// Event Listeners
function setupEventListeners() {
    // Request sub-tabs
    document.querySelectorAll('.request-sub-tab').forEach(tab => {
        tab.addEventListener('click', () => switchRequestTab(tab.dataset.tab));
    });

    // Panel resizer
    setupPanelResizer();

    // Response tabs
    document.querySelectorAll('.response-tab').forEach(tab => {
        tab.addEventListener('click', () => switchResponseTab(tab.dataset.tab));
    });

    // Send request
    document.getElementById('send-request-btn').addEventListener('click', sendRequest);

    // Method change handler for WebSocket mode
    document.getElementById('request-method').addEventListener('change', handleMethodChange);

    // WebSocket buttons
    document.getElementById('ws-connect-btn').addEventListener('click', connectWebSocket);
    document.getElementById('ws-disconnect-btn').addEventListener('click', disconnectWebSocket);
    document.getElementById('ws-send-btn').addEventListener('click', sendWebSocketMessage);
    document.getElementById('ws-clear-btn').addEventListener('click', clearWebSocketMessages);

    // Environment selector
    const envSelect = document.getElementById('environment-select');
    envSelect.addEventListener('change', (e) => {
        currentEnvironmentId = e.target.value ? parseInt(e.target.value) : null;
        updateVariablePreview();

        // Show/hide environment action buttons
        const envActions = document.getElementById('environment-actions');
        if (envActions) {
            if (currentEnvironmentId) {
                envActions.style.display = 'flex';
                // Hide archive button for now
                const archiveBtn = document.getElementById('archive-environment-btn');
                if (archiveBtn) archiveBtn.style.display = 'none';
            } else {
                envActions.style.display = 'none';
            }
        }
    });

    // Add context menu for environment selector
    envSelect.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        const envId = parseInt(e.target.value);
        if (!envId) return;

        const env = environments.find(e => e.id === envId);
        if (!env) return;

        const isArchived = env.archived_at;
        showContextMenu(e, [
            {
                label: 'Edit',
                icon: 'fas fa-edit',
                action: () => editEnvironment(envId)
            },
            /*
            {
                label: isArchived ? 'Unarchive' : 'Archive',
                icon: isArchived ? 'fas fa-archive' : 'fas fa-archive',
                action: () => isArchived ? unarchiveEnvironment(envId) : archiveEnvironment(envId)
            },
            */
            {
                label: 'Delete',
                icon: 'fas fa-trash',
                action: () => deleteEnvironment(envId),
                danger: true
            }
        ]);
    });

    // Header management
    setupHeaderManagement();

    // Copy response
    document.getElementById('copy-response').addEventListener('click', copyResponse);

    // Format response
    document.getElementById('format-response').addEventListener('click', formatResponse);

    // New folder button
    const newFolderBtn = document.getElementById('new-folder-btn');
    if (newFolderBtn) {
        newFolderBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            e.preventDefault();
            openFolderModal();
        });
    } else {
        console.error('New folder button not found');
    }

    // New request button in sidebar
    const newRequestSidebarBtn = document.getElementById('new-request-sidebar-btn');
    if (newRequestSidebarBtn) {
        newRequestSidebarBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            e.preventDefault();
            openRequestModal();
        });
    }

    // Import button
    setupImportHandler();
    
    // Confirm Import button
    const confirmImportBtn = document.getElementById('confirm-import-btn');
    if (confirmImportBtn) {
        confirmImportBtn.addEventListener('click', confirmImport);
    }

    // Close modals on overlay click
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            if (e.target === overlay) {
                closeModal(overlay.id);
            }
        });
    });

    // Add environment button (if exists)
    const addEnvBtn = document.getElementById('add-environment-btn');
    if (addEnvBtn) {
        addEnvBtn.addEventListener('click', () => openEnvironmentModal());
    }

    // Edit environment button
    const editEnvBtn = document.getElementById('edit-environment-btn');
    if (editEnvBtn) {
        editEnvBtn.addEventListener('click', () => {
            if (currentEnvironmentId) {
                openEnvironmentModal(currentEnvironmentId);
            }
        });
    }

    // Archive environment button
    const archiveEnvBtn = document.getElementById('archive-environment-btn');
    if (archiveEnvBtn) {
        archiveEnvBtn.addEventListener('click', () => {
            if (currentEnvironmentId) {
                archiveEnvironment(currentEnvironmentId);
            }
        });
    }

    // Delete environment button
    const deleteEnvBtn = document.getElementById('delete-environment-btn');
    if (deleteEnvBtn) {
        deleteEnvBtn.addEventListener('click', () => {
            if (currentEnvironmentId) {
                deleteEnvironment(currentEnvironmentId);
            }
        });
    }

    // Allow Enter key to submit modals
    document.querySelectorAll('.modal form').forEach(form => {
        form.addEventListener('submit', (e) => {
            e.preventDefault();
            const modal = form.closest('.modal-overlay');
            if (modal.id === 'folder-modal') {
                saveFolder();
            } else if (modal.id === 'request-modal') {
                saveRequestFromModal();
            } else if (modal.id === 'environment-modal') {
                saveEnvironment();
            }
        });
    });

    // Type selector (API/WS) change handler
    const typeSelect = document.getElementById('request-type');
    if (typeSelect) {
        typeSelect.addEventListener('change', handleRequestTypeChange);
    }

    // Auth type selector change handler
    const authTypeSelect = document.getElementById('auth-type-select');
    if (authTypeSelect) {
        authTypeSelect.addEventListener('change', handleAuthTypeChange);
    }

    // Body type selector change handler
    const bodyTypeSelect = document.getElementById('body-type-select');
    if (bodyTypeSelect) {
        bodyTypeSelect.addEventListener('change', handleBodyTypeChange);
    }

    // Format body button
    const formatBodyBtn = document.getElementById('format-body-btn');
    if (formatBodyBtn) {
        formatBodyBtn.addEventListener('click', formatRequestBody);
    }

    // Preview response button
    const previewResponseBtn = document.getElementById('preview-response');
    if (previewResponseBtn) {
        previewResponseBtn.addEventListener('click', toggleHtmlPreview);
    }

    // Environment variables textarea - hide sample on input
    const envVariablesTextarea = document.getElementById('environment-variables');
    if (envVariablesTextarea) {
        envVariablesTextarea.addEventListener('input', handleEnvVariablesInput);
        envVariablesTextarea.addEventListener('focus', handleEnvVariablesFocus);
        envVariablesTextarea.addEventListener('blur', handleEnvVariablesBlur);
    }

    // Close modals on Escape key
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            const activeModal = document.querySelector('.modal-overlay.active');
            if (activeModal) {
                closeModal(activeModal.id);
            }
        }
    });
}

// Load folders and build collection tree
async function loadFolders() {
    try {
        const response = await fetch('/api/folders');
        if (!response.ok) {
            throw new Error(`Failed to load folders: ${response.status} ${response.statusText}`);
        }
        folders = await response.json();
        buildCollectionTree();
    } catch (error) {
        console.error('Error loading folders:', error);
        showNotification('Failed to load folders', 'error');
    }
}

// Load requests
async function loadRequests(folderId = null, includeArchived = false) {
    try {
        const params = new URLSearchParams();
        if (folderId) {
            params.append('folder_id', folderId);
        }
        if (includeArchived) {
            params.append('include_archived', 'true');
        }
        const url = `/api/requests${params.toString() ? '?' + params.toString() : ''}`;
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to load requests: ${response.status} ${response.statusText}`);
        }
        requests = await response.json();
        buildCollectionTree();
        buildHistoryTree();
    } catch (error) {
        console.error('Error loading requests:', error);
        showNotification('Failed to load requests', 'error');
    }
}

// Build collection tree
function buildCollectionTree() {
    const tree = document.getElementById('collection-tree');
    tree.innerHTML = '';

    // Filter requests based on archived status
    const visibleRequests = showArchived
        ? requests
        : requests.filter(r => !r.archived_at);

    // Group requests by folder
    const folderMap = new Map();
    folders.forEach(folder => {
        if (!showArchived && folder.archived_at) return;
        folderMap.set(folder.id, {
            ...folder,
            requests: []
        });
    });

    // Add requests to folders
    visibleRequests.forEach(request => {
        if (request.folder_id && folderMap.has(request.folder_id)) {
            folderMap.get(request.folder_id).requests.push(request);
        }
    });

    // Create a main "Project API" parent if we have folders or requests
    if (folderMap.size > 0 || visibleRequests.length > 0) {
        // Sort folders by created_at (newer folders appear below)
        const sortedFolders = Array.from(folderMap.entries()).sort((a, b) => {
            const folderA = folders.find(f => f.id === a[0]);
            const folderB = folders.find(f => f.id === b[0]);
            if (!folderA || !folderB) return 0;
            // Sort by created_at ascending (older first, newer last)
            const dateA = new Date(folderA.created_at || 0);
            const dateB = new Date(folderB.created_at || 0);
            return dateA - dateB;
        });

        // Add folders and their requests as sub-items
        sortedFolders.forEach(([folderId, folder]) => {
            const folderItem = document.createElement('li');
            folderItem.className = 'collection-item collection-sub-item';
            folderItem.dataset.folderId = folderId;
            const folderName = escapeHtml(folder.name);
            folderItem.innerHTML = `
                <div class="collection-item-name">
                    <i class="fas fa-chevron-right folder-chevron" style="font-size: 10px; margin-right: 8px; opacity: 0.7; width: 12px; display: inline-block;"></i>
                    <span>${folderName.includes('>') ? `<b>${folderName}</b>` : folderName}</span>
                </div>
                <div class="collection-item-actions">
                    <span class="collection-item-count">${folder.requests.length}</span>
                </div>
            `;

            // Create a container for folder requests
            const folderRequestsContainer = document.createElement('ul');
            folderRequestsContainer.className = 'folder-requests';
            folderRequestsContainer.style.display = 'none'; // Collapsed by default

            // Add requests under folder
            folder.requests.forEach(request => {
                const requestItem = document.createElement('li');
                requestItem.className = 'collection-item collection-sub-item';
                requestItem.style.paddingLeft = '32px';
                if (request.archived_at) {
                    requestItem.style.opacity = '0.6';
                }
                const requestName = escapeHtml(request.name);
                const methodClass = request.method || 'GET';
                requestItem.innerHTML = `
                    <div class="collection-item-name">
                        <span class="history-item-method ${methodClass}" style="margin-right: 8px; font-size: 9px; padding: 1px 4px; min-width: 35px;">${methodClass}</span>
                        <span>${requestName.includes('>') ? `<b>${requestName}</b>` : requestName}</span>
                    </div>
                    <div class="collection-item-actions">
                    </div>
                `;
                requestItem.addEventListener('click', (e) => {
                    e.stopPropagation();
                    selectRequest(request.id);
                });
                requestItem.addEventListener('contextmenu', (e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const isArchived = request.archived_at;
                    showContextMenu(e, [
                        {
                            label: 'Edit',
                            icon: 'fas fa-edit',
                            action: () => editRequest(request.id)
                        },
                        /*
                        {
                            label: isArchived ? 'Unarchive' : 'Archive',
                            icon: isArchived ? 'fas fa-archive' : 'fas fa-archive',
                            action: () => isArchived ? unarchiveRequest(request.id) : archiveRequest(request.id)
                        },
                        */
                        {
                            label: 'Delete',
                            icon: 'fas fa-trash',
                            action: () => deleteRequest(request.id),
                            danger: true
                        }
                    ]);
                });
                folderRequestsContainer.appendChild(requestItem);
            });

            // Handle folder click - toggle expand/collapse
            folderItem.addEventListener('click', (e) => {
                e.stopPropagation();
                // Don't toggle if clicking on the count
                if (e.target.closest('.collection-item-count')) {
                    selectFolder(folderId, folderItem);
                    return;
                }

                // Toggle expand/collapse
                const isExpanded = folderItem.classList.contains('expanded');
                const chevron = folderItem.querySelector('.folder-chevron') || folderItem.querySelector('i.fa-chevron-right, i.fa-chevron-down');

                if (isExpanded) {
                    // Collapse
                    folderItem.classList.remove('expanded');
                    folderRequestsContainer.style.display = 'none';
                    if (chevron) {
                        chevron.classList.remove('fa-chevron-down');
                        chevron.classList.add('fa-chevron-right');
                    }
                } else {
                    // Expand
                    folderItem.classList.add('expanded');
                    folderRequestsContainer.style.display = 'block';
                    if (chevron) {
                        chevron.classList.remove('fa-chevron-right');
                        chevron.classList.add('fa-chevron-down');
                    }
                }
            });

            folderItem.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                e.stopPropagation();
                const folder = folders.find(f => f.id === folderId);
                const isArchived = folder && folder.archived_at;
                showContextMenu(e, [
                    {
                        label: 'Edit',
                        icon: 'fas fa-edit',
                        action: () => editFolder(folderId)
                    },
                    /*
                    {
                        label: isArchived ? 'Unarchive' : 'Archive',
                        icon: isArchived ? 'fas fa-archive' : 'fas fa-archive',
                        action: () => isArchived ? unarchiveFolder(folderId) : archiveFolder(folderId)
                    },
                    */
                    {
                        label: 'Delete',
                        icon: 'fas fa-trash',
                        action: () => deleteFolder(folderId),
                        danger: true
                    }
                ]);
            });

            // Append folder item first
            tree.appendChild(folderItem);
            // Then append requests container (will appear right after folder item)
            tree.appendChild(folderRequestsContainer);
        });

        // Add requests without folders as sub-items
        const rootRequests = visibleRequests.filter(r => !r.folder_id);
        rootRequests.forEach(request => {
            const requestItem = document.createElement('li');
            requestItem.className = 'collection-item collection-sub-item';
            const requestName = escapeHtml(request.name);
            const methodClass = request.method || 'GET';
            requestItem.innerHTML = `
                <div class="collection-item-name">
                    <span class="history-item-method ${methodClass}" style="margin-right: 8px; font-size: 9px; padding: 1px 4px; min-width: 35px;">${methodClass}</span>
                    <span>${requestName.includes('>') ? `<b>${requestName}</b>` : requestName}</span>
                </div>
                <div class="collection-item-actions">
                    <button class="edit-item-btn" title="Edit request" data-request-id="${request.id}">
                        <i class="fas fa-edit"></i>
                    </button>
                </div>
            `;
            requestItem.addEventListener('click', (e) => {
                if (!e.target.closest('.edit-item-btn')) {
                    e.stopPropagation();
                    selectRequest(request.id);
                }
            });
            requestItem.querySelector('.edit-item-btn').addEventListener('click', (e) => {
                e.stopPropagation();
                editRequest(request.id);
            });
            requestItem.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                e.stopPropagation();
                const isArchived = request.archived_at;
                showContextMenu(e, [
                    {
                        label: 'Edit',
                        icon: 'fas fa-edit',
                        action: () => editRequest(request.id)
                    },
                    /*
                    {
                        label: isArchived ? 'Unarchive' : 'Archive',
                        icon: 'fas fa-archive',
                        action: () => isArchived ? unarchiveRequest(request.id) : archiveRequest(request.id)
                    },
                    */
                    {
                        label: 'Delete',
                        icon: 'fas fa-trash',
                        action: () => deleteRequest(request.id),
                        danger: true
                    }
                ]);
            });
            tree.appendChild(requestItem);
        });
    }
}

// Select folder
function selectFolder(folderId, element) {
    document.querySelectorAll('.collection-item').forEach(item => {
        item.classList.remove('active');
    });
    if (element) {
        element.classList.add('active');
    }
    loadRequests(folderId, showArchived);
}

// Select request
async function selectRequest(requestId) {
    try {
        const response = await fetch(`/api/requests/${requestId}`);
        if (!response.ok) {
            throw new Error(`Failed to load request: ${response.status} ${response.statusText}`);
        }
        const request = await response.json();
        currentRequestId = requestId;
        loadRequestIntoEditor(request);

        // Update active tab
        document.querySelectorAll('.request-tab').forEach(t => t.classList.remove('active'));
        const tab = document.querySelector(`[data-request-id="${requestId}"]`);
        if (tab) {
            tab.classList.add('active');
        }

        // Update top bar display
        updateTopBarRequests();
    } catch (error) {
        console.error('Error loading request:', error);
        showNotification('Failed to load request', 'error');
    }
}

// Load request into editor
function loadRequestIntoEditor(request) {
    if (!request || !request.id) return;

    // Set request type (API or WebSocket)
    const typeSelect = document.getElementById('request-type');
    if (typeSelect) {
        typeSelect.value = request.request_type || 'api';
        // Trigger change event to update UI
        typeSelect.dispatchEvent(new Event('change'));
    }

    document.getElementById('request-method').value = request.method || 'GET';
    document.getElementById('request-url').value = request.url || '';

    // Load body type and content
    const bodyTypeSelect = document.getElementById('body-type-select');
    if (bodyTypeSelect) {
        bodyTypeSelect.value = request.body_type || 'none';
        bodyTypeSelect.dispatchEvent(new Event('change'));
    }

    // Load body content - prioritize body_content over legacy body
    const bodyTextarea = document.getElementById('request-body');
    if (request.body_content) {
        bodyTextarea.value = request.body_content;
    } else if (request.body) {
        bodyTextarea.value = request.body;
    } else {
        bodyTextarea.value = '';
    }

    // Load authentication
    const authTypeSelect = document.getElementById('auth-type-select');
    if (authTypeSelect) {
        authTypeSelect.value = request.auth_type || 'none';
        authTypeSelect.dispatchEvent(new Event('change'));
    }

    if (request.auth_type === 'bearer' && request.auth_token) {
        const tokenInput = document.getElementById('auth-bearer-token');
        if (tokenInput) tokenInput.value = request.auth_token;
    } else if (request.auth_type === 'basic') {
        const usernameInput = document.getElementById('auth-basic-username');
        const passwordInput = document.getElementById('auth-basic-password');
        if (usernameInput && request.auth_username) usernameInput.value = request.auth_username;
        if (passwordInput && request.auth_password) passwordInput.value = request.auth_password;
    }

    // Load headers
    const tbody = document.getElementById('headers-tbody');
    if (tbody) {
        tbody.innerHTML = '';

        if (request.headers) {
            try {
                const headers = JSON.parse(request.headers);
                Object.entries(headers).forEach(([key, value]) => {
                    addHeaderRow(key, value);
                });
            } catch (e) {
                console.error('Error parsing headers:', e);
                showNotification('Warning: Failed to parse headers', 'error');
            }
        }
        addHeaderRow('', ''); // Empty row for new header
    }

    // Add/activate request tab - ensure this happens
    addRequestTab(request);
    currentRequestId = request.id;
}

// Add request tab (now rendering as chip in top bar)
function addRequestTab(request) {
    if (!request || !request.id) return;

    const container = document.getElementById("top-bar-requests");
    if (!container) return;

    let chip = container.querySelector(`[data-request-id="${request.id}"]`);

    if (!chip) {
        chip = document.createElement("div");
        chip.className = "top-bar-request-chip";
        chip.dataset.requestId = request.id;
        
        const requestName = request.name || "Untitled";
        chip.innerHTML = `
            <span>${escapeHtml(requestName)}</span>
            <i class="fas fa-times chip-close" style="margin-left: 8px; font-size: 10px; opacity: 0.6; cursor: pointer;"></i>
        `;

        // Handle chip click
        chip.addEventListener("click", (e) => {
            if (e.target.closest(".chip-close")) return;
            selectRequest(request.id);
        });

        // Handle close click
        const closeBtn = chip.querySelector(".chip-close");
        closeBtn.addEventListener("click", (e) => {
            e.stopPropagation();
            closeRequestTab(request.id);
        });

        container.appendChild(chip);
    } else {
        // Update name
        const span = chip.querySelector("span");
        if (span) span.textContent = request.name || "Untitled";
    }

    // Deactivate others and activate this one
    container.querySelectorAll(".top-bar-request-chip").forEach(c => c.classList.remove("active"));
    chip.classList.add("active");
}

// Close request tab (removing chip)
function closeRequestTab(requestId) {
    const container = document.getElementById("top-bar-requests");
    if (!container) return;

    const chip = container.querySelector(`[data-request-id="${requestId}"]`);
    if (chip) {
        chip.remove();
    }

    if (currentRequestId === requestId) {
        currentRequestId = null;
        clearEditor();

        // Select last open request if any
        const remaining = container.querySelectorAll(".top-bar-request-chip");
        if (remaining.length > 0) {
            const lastId = remaining[remaining.length - 1].dataset.requestId;
            selectRequest(lastId);
        }
    }
}

// Update top bar requests display
function updateTopBarRequests() {
    const container = document.getElementById("top-bar-requests");
    if (!container) return;

    // Ensure current request chip is active
    container.querySelectorAll(".top-bar-request-chip").forEach(chip => {
        const chipRequestId = parseInt(chip.dataset.requestId);
        if (chipRequestId === currentRequestId) {
            chip.classList.add("active");
        } else {
            chip.classList.remove("active");
        }
    });
}

function openModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.add('active');
        document.body.style.overflow = 'hidden';
        // Force a reflow to ensure the modal is visible
        void modal.offsetHeight;
    } else {
        console.error(`Modal with id "${modalId}" not found`);
    }
}

function closeModal(modalId) {
    const modal = document.getElementById(modalId);
    if (modal) {
        modal.classList.remove('active');
        document.body.style.overflow = '';
        // Reset form
        const form = modal.querySelector('form');
        if (form) {
            form.reset();
            const hiddenInput = form.querySelector('input[type="hidden"]');
            if (hiddenInput) {
                hiddenInput.value = '';
            }
        }
    }
}

// Folder Modal Functions
function openFolderModal(folderId = null) {
    const modal = document.getElementById('folder-modal');
    const title = document.getElementById('folder-modal-title');
    const nameInput = document.getElementById('folder-name');
    const idInput = document.getElementById('folder-id');

    if (!modal) {
        console.error('Folder modal not found');
        return;
    }

    if (folderId) {
        // Edit mode
        const folder = folders.find(f => f.id === folderId);
        if (folder) {
            if (title) title.textContent = 'Edit Folder';
            if (nameInput) nameInput.value = folder.name;
            if (idInput) idInput.value = folder.id;
        }
    } else {
        // New mode
        if (title) title.textContent = 'New Folder';
        if (nameInput) nameInput.value = '';
        if (idInput) idInput.value = '';
    }

    openModal('folder-modal');
}

async function saveFolder() {
    const idInput = document.getElementById('folder-id');
    const nameInput = document.getElementById('folder-name');
    const folderId = idInput.value ? parseInt(idInput.value) : null;
    const name = nameInput.value.trim();

    if (!name) {
        showNotification('Please enter a folder name', 'error');
        return;
    }

    // FIX: Add validation for name length
    if (name.length > 255) {
        showNotification('Folder name is too long (max 255 characters)', 'error');
        return;
    }

    try {
        let response;
        if (folderId) {
            // Update existing
            response = await fetch(`/api/folders/${folderId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: name })
            });
        } else {
            // Create new
            response = await fetch('/api/folders', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: name })
            });
        }

        if (response.ok) {
            await loadFolders();
            await loadRequests(null, showArchived);
            closeModal('folder-modal');
            showNotification(folderId ? 'Folder updated successfully' : 'Folder created successfully');
        } else {
            const error = await response.text();
            showNotification(`Error: ${error}`, 'error');
        }
    } catch (error) {
        console.error('Error saving folder:', error);
        showNotification('Failed to save folder. Please try again.', 'error');
    }
}

// Request Modal Functions
function openRequestModal(requestId = null, type = null) {
    if (requestId) {
        // If requestId is provided, it's an edit, so we determine type from existing request
        const request = requests.find(r => r.id === requestId);
        if (request && request.method === 'WS') {
            openHttpRequestModal(requestId, 'ws');
        } else {
            openHttpRequestModal(requestId, 'http');
        }
    } else if (type) {
        // If type is provided, open modal directly for that type
        openHttpRequestModal(null, type);
    } else {
        // No requestId or type, show the choice modal
        openModal('new-request-choice-modal');
    }
}

async function openHttpRequestModal(requestId = null, type = 'http') {
    const title = document.getElementById('request-modal-title');
    const nameInput = document.getElementById('request-name');
    const methodInput = document.getElementById('request-method-modal');
    const urlInput = document.getElementById('request-url-modal');
    const folderInput = document.getElementById('request-folder-modal');
    const idInput = document.getElementById('request-id');
    const methodGroup = methodInput.closest('.form-group'); // Get the parent form-group for visibility toggle

    // Ensure folders are loaded before populating dropdown
    if (folders.length === 0) {
        try {
            const response = await fetch('/api/folders');
            if (response.ok) {
                folders = await response.json();
            }
        } catch (error) {
            console.error('Error loading folders for dropdown:', error);
            showNotification('Warning: Could not load folders', 'error');
        }
    }

    // Populate folder dropdown
    folderInput.innerHTML = '<option value="">None</option>';
    folders.forEach(folder => {
        if (!folder.archived_at) {
            const option = document.createElement('option');
            option.value = folder.id;
            option.textContent = folder.name;
            folderInput.appendChild(option);
        }
    });

    // Reset visibility and default values for new mode
    if (methodGroup) methodGroup.classList.remove('hidden');
    methodInput.value = 'GET';
    urlInput.value = 'https://api.example.com/v1/users';
    nameInput.value = '';


    if (requestId) {
        // Edit mode - fetch the latest request data
        try {
            const response = await fetch(`/api/requests/${requestId}`);
            if (response.ok) {
                const request = await response.json();
                title.textContent = 'Edit Request';
                nameInput.value = request.name;
                methodInput.value = request.method;
                urlInput.value = request.url;
                folderInput.value = request.folder_id || '';
                idInput.value = request.id;

                if (request.method === 'WS' && methodGroup) {
                    methodGroup.classList.add('hidden');
                }
            }
        } catch (error) {
            console.error('Error loading request for edit:', error);
            // Fall back to cached data
            const request = requests.find(r => r.id === requestId);
            if (request) {
                title.textContent = 'Edit Request';
                nameInput.value = request.name;
                methodInput.value = request.method;
                urlInput.value = request.url;
                folderInput.value = request.folder_id || '';
                idInput.value = request.id;

                if (request.method === 'WS' && methodGroup) {
                    methodGroup.classList.add('hidden');
                }
            }
        }
    } else {
        // New mode
        title.textContent = 'New Request';
        idInput.value = '';

        if (type === 'ws') {
            title.textContent = 'New WebSocket Request';
            nameInput.value = 'New WebSocket Request';
            methodInput.value = 'WS';
            urlInput.value = 'wss://echo.websocket.org';
            if (methodGroup) methodGroup.classList.add('hidden');
        } else {
            // Default to HTTP
            title.textContent = 'New HTTP Request';
            nameInput.value = 'New HTTP Request';
            methodInput.value = 'GET';
            urlInput.value = 'https://api.example.com/v1/users';
            if (methodGroup) methodGroup.classList.remove('hidden');
        }
    }

    openModal('request-modal');
}

async function saveRequestFromModal() {
    const idInput = document.getElementById('request-id');
    const nameInput = document.getElementById('request-name');
    const methodInput = document.getElementById('request-method-modal');
    const urlInput = document.getElementById('request-url-modal');
    const folderInput = document.getElementById('request-folder-modal');

    const requestId = idInput.value ? parseInt(idInput.value) : null;
    const name = nameInput.value.trim();
    const method = methodInput.value;
    const url = urlInput.value.trim();
    const folderId = folderInput.value ? parseInt(folderInput.value) : null;

    if (!name || !url) {
        showNotification('Please fill in all required fields', 'error');
        return;
    }

    // FIX: Add validation for name length
    if (name.length > 255) {
        showNotification('Request name is too long (max 255 characters)', 'error');
        return;
    }

    try {
        let response;
        // Determine request type and final method from modal values
        const isWS = method === 'WS';
        const requestType = isWS ? 'ws' : 'api';
        const finalMethod = isWS ? '' : method;

        if (requestId) {
            // Update existing - need to get current request first
            const currentRequest = requests.find(r => r.id === requestId);
            // const requestTypeSelect = document.getElementById('request-type');
            // const requestType = requestTypeSelect ? requestTypeSelect.value : (currentRequest?.request_type || 'api');
            response = await fetch(`/api/requests/${requestId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: name,
                    method: finalMethod,
                    url: url,
                    body: currentRequest?.body || null,
                    headers: currentRequest?.headers || null,
                    folder_id: folderId,
                    request_type: requestType
                })
            });
        } else {
            // Create new
            // const requestTypeSelect = document.getElementById('request-type');
            // const requestType = requestTypeSelect ? requestTypeSelect.value : 'api';
            response = await fetch('/api/requests', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: name,
                    method: finalMethod,
                    url: url,
                    body: null,
                    headers: null,
                    folder_id: folderId,
                    request_type: requestType
                })
            });
        }

        if (response.ok) {
            const request = await response.json();
            // Load request into editor first (before reloading all requests)
            if (!requestId) {
                currentRequestId = request.id;
                // Ensure request has all required fields
                if (!request.name) request.name = name;
                if (!request.method) request.method = method;
                if (!request.url) request.url = url;
                loadRequestIntoEditor(request);
            }
            // Reload requests to update collection tree
            await loadRequests(null, showArchived);
            if (requestId) {
                // Reload if editing current request
                if (currentRequestId === requestId) {
                    await selectRequest(requestId);
                }
            }
            closeModal('request-modal');
            showNotification(requestId ? 'Request updated successfully' : 'Request created successfully');
        } else {
            const error = await response.text();
            showNotification(`Error: ${error}`, 'error');
        }
    } catch (error) {
        console.error('Error saving request:', error);
        showNotification('Failed to save request. Please try again.', 'error');
    }
}

// Environment Modal Functions
function openEnvironmentModal(envId = null) {
    const title = document.getElementById('environment-modal-title');
    const nameInput = document.getElementById('environment-name');
    const varsInput = document.getElementById('environment-variables');
    const idInput = document.getElementById('environment-id');

    if (envId) {
        // Edit mode
        const env = environments.find(e => e.id === envId);
        if (env) {
            title.textContent = 'Edit Environment';
            nameInput.value = env.name;
            try {
                const vars = JSON.parse(env.variables);
                varsInput.value = JSON.stringify(vars, null, 4);
            } catch (e) {
                varsInput.value = env.variables;
            }
            idInput.value = env.id;
            // Hide sample when editing (textarea has content)
            const sample = document.getElementById('env-variables-sample');
            if (sample) sample.classList.add('hidden');
        }
    } else {
        // New mode
        title.textContent = 'New Environment';
        nameInput.value = '';
        varsInput.value = '';
        idInput.value = '';
    }

    // Reset the sample visibility
    resetEnvironmentModal();

    openModal('environment-modal');
}

async function saveEnvironment() {
    const idInput = document.getElementById('environment-id');
    const nameInput = document.getElementById('environment-name');
    const varsInput = document.getElementById('environment-variables');

    const envId = idInput.value ? parseInt(idInput.value) : null;
    const name = nameInput.value.trim();
    let variables = varsInput.value.trim();

    if (!name) {
        showNotification('Please enter an environment name', 'error');
        return;
    }

    // FIX: Add validation for name length
    if (name.length > 255) {
        showNotification('Environment name is too long (max 255 characters)', 'error');
        return;
    }

    // Validate JSON
    if (variables) {
        try {
            JSON.parse(variables);
        } catch (e) {
            showNotification('Invalid JSON format for variables. Please check your syntax.', 'error');
            return;
        }
    } else {
        variables = '{}';
    }

    try {
        let response;
        if (envId) {
            // Update existing
            response = await fetch(`/api/environments/${envId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: name, variables: variables })
            });
        } else {
            // Create new
            response = await fetch('/api/environments', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: name, variables: variables })
            });
        }

        if (response.ok) {
            await loadEnvironments();
            updateVariablePreview();
            closeModal('environment-modal');
            showNotification(envId ? 'Environment updated successfully' : 'Environment created successfully');
        } else {
            const error = await response.text();
            showNotification(`Error: ${error}`, 'error');
        }
    } catch (error) {
        console.error('Error saving environment:', error);
        showNotification('Failed to save environment. Please try again.', 'error');
    }
}

// Notification function
function showNotification(message, type = 'success') {
    // Simple notification - can be enhanced with a toast library
    const notification = document.createElement('div');
    const bgColor = type === 'error' ? '#ef4444' : 'var(--success)';
    notification.style.cssText = `
        position: fixed;
        top: 70px;
        right: 20px;
        background: ${bgColor};
        color: white;
        padding: 12px 20px;
        border-radius: 6px;
        box-shadow: 0 4px 6px rgba(0,0,0,0.1);
        z-index: 3000;
        animation: slideIn 0.3s ease-out;
        max-width: 300px;
    `;
    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease-out';
        setTimeout(() => notification.remove(), 300);
    }, type === 'error' ? 5000 : 3000);
}

// Add CSS for notification animation
const style = document.createElement('style');
style.textContent = `
    @keyframes slideIn {
        from {
            transform: translateX(100%);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }
    @keyframes slideOut {
        from {
            transform: translateX(0);
            opacity: 1;
        }
        to {
            transform: translateX(100%);
            opacity: 0;
        }
    }
`;
document.head.appendChild(style);

// Clear editor
function clearEditor() {
    document.getElementById('request-method').value = 'GET';
    document.getElementById('request-url').value = '';
    document.getElementById('request-body').value = '';
    document.getElementById('headers-tbody').innerHTML = '';
    addHeaderRow('', '');
}

// Switch request tab
function switchRequestTab(tabName) {
    // Remove active from all tab buttons
    document.querySelectorAll('.request-sub-tab').forEach(tab => {
        tab.classList.remove('active');
    });

    // Remove active from all tab panes
    document.querySelectorAll('.tab-pane').forEach(pane => {
        pane.classList.remove('active');
    });

    // Add active to clicked tab button
    const tabButton = document.querySelector(`.request-sub-tab[data-tab="${tabName}"]`);
    if (tabButton) {
        tabButton.classList.add('active');
    }

    // Add active to corresponding tab pane
    const tabPane = document.getElementById(`${tabName}-tab`);
    if (tabPane) {
        tabPane.classList.add('active');
    }
}

// Switch response tab
function switchResponseTab(tabName) {
    document.querySelectorAll('.response-tab').forEach(tab => {
        tab.classList.remove('active');
    });
    document.querySelectorAll('.response-tab-pane').forEach(pane => {
        pane.classList.remove('active');
    });

    document.querySelector(`.response-tab[data-tab="${tabName}"]`).classList.add('active');
    document.getElementById(tabName).classList.add('active');
}

// Setup header management
function setupHeaderManagement() {
    const tbody = document.getElementById('headers-tbody');
    if (!tbody) return;

    tbody.addEventListener('input', (e) => {
        if (e.target.classList.contains('header-input')) {
            const row = e.target.closest('tr');
            if (!row) return;

            const keyInput = row.querySelector('input[placeholder="Key"]');
            const valueInput = row.querySelector('input[placeholder="Value"]');

            if (!keyInput || !valueInput) return;

            // If this is the last row and both fields have values, add a new row
            if (row.id === 'new-header-row' && keyInput.value.trim() && valueInput.value.trim()) {
                row.id = '';
                const removeBtn = row.querySelector('.header-remove');
                if (removeBtn) {
                    removeBtn.style.display = 'block';
                }
                addHeaderRow('', '');
            }
        }
    });

    tbody.addEventListener('click', (e) => {
        if (e.target.closest('.header-remove')) {
            const row = e.target.closest('tr');
            if (row) {
                row.remove();
                // Ensure there's always a new-header-row
                const newRow = tbody.querySelector('#new-header-row');
                if (!newRow) {
                    addHeaderRow('', '');
                }
            }
        }
    });
}

// Add header row
function addHeaderRow(key, value) {
    const tbody = document.getElementById('headers-tbody');
    const row = document.createElement('tr');
    row.className = 'header-row';
    if (!key && !value) {
        row.id = 'new-header-row';
    }
    row.innerHTML = `
        <td>
            <input type="checkbox" class="header-checkbox" ${key ? 'checked' : ''}>
        </td>
        <td>
            <input type="text" class="header-input" value="${escapeHtml(key)}" placeholder="Key">
        </td>
        <td>
            <input type="text" class="header-input" value="${escapeHtml(value)}" placeholder="Value">
        </td>
        <td>
            <button class="header-remove" ${!key ? 'style="display: none;"' : ''}>
                <i class="fas fa-times"></i>
            </button>
        </td>
    `;
    tbody.appendChild(row);
}

// Get headers from table
function getHeaders() {
    const headers = {};
    document.querySelectorAll('.header-row').forEach(row => {
        const checkbox = row.querySelector('.header-checkbox');
        if (checkbox.checked) {
            const keyInput = row.querySelector('input[placeholder="Key"]');
            const valueInput = row.querySelector('input[placeholder="Value"]');
            if (keyInput.value && valueInput.value) {
                headers[keyInput.value] = valueInput.value;
            }
        }
    });
    return headers;
}

// Load environments
async function loadEnvironments() {
    try {
        const response = await fetch('/api/environments');
        if (!response.ok) {
            throw new Error(`Failed to load environments: ${response.status} ${response.statusText}`);
        }
        environments = await response.json();
        populateEnvironmentSelector();
    } catch (error) {
        console.error('Error loading environments:', error);
        showNotification('Failed to load environments', 'error');
    }
}

// Populate environment selector
function populateEnvironmentSelector() {
    const select = document.getElementById('environment-select');
    select.innerHTML = '<option value="">Select Environment</option>';

    environments.forEach(env => {
        if (!env.archived_at) {
            const option = document.createElement('option');
            option.value = env.id;
            option.textContent = env.name;
            select.appendChild(option);
        }
    });

    // Set default to "Development" if it exists
    const devEnv = environments.find(e => e.name.toLowerCase() === 'development' && !e.archived_at);
    if (devEnv) {
        select.value = devEnv.id;
        currentEnvironmentId = devEnv.id;
        updateVariablePreview();
    }

    // Add double-click to edit
    select.addEventListener('dblclick', (e) => {
        const envId = parseInt(e.target.value);
        if (envId) {
            editEnvironment(envId);
        }
    });
}

// Update variable preview
function updateVariablePreview() {
    const preview = document.getElementById('variable-preview');
    preview.innerHTML = '';

    if (!currentEnvironmentId) {
        // Show default example variables if no environment selected
        // FIX: Corrected typo from 'base_resenant' to 'base_resonant'
        const defaultVars = {
            'base_url': 'https://api.example.com',
            'api_key': 'your-api-key-here'
        };
        Object.entries(defaultVars).forEach(([key, value]) => {
            const item = document.createElement('li');
            item.className = 'variable-item';
            item.innerHTML = `
                <div class="variable-key">{{${escapeHtml(key)}}}</div>
                <div class="variable-value">→ ${escapeHtml(value)}</div>
            `;
            preview.appendChild(item);
        });
        return;
    }

    const env = environments.find(e => e.id === currentEnvironmentId);
    if (!env) return;

    try {
        const variables = JSON.parse(env.variables);
        if (Object.keys(variables).length === 0) {
            // Show default if empty
            const defaultVars = {
                'base_url': 'https://api.example.com'
            };
            Object.entries(defaultVars).forEach(([key, value]) => {
                const item = document.createElement('li');
                item.className = 'variable-item';
                item.innerHTML = `
                    <div class="variable-key">{{${escapeHtml(key)}}}</div>
                    <div class="variable-value">→ ${escapeHtml(value)}</div>
                `;
                preview.appendChild(item);
            });
        } else {
            Object.entries(variables).forEach(([key, value]) => {
                const item = document.createElement('li');
                item.className = 'variable-item';
                item.innerHTML = `
                    <div class="variable-key">{{${escapeHtml(key)}}}</div>
                    <div class="variable-value">→ ${escapeHtml(value)}</div>
                `;
                preview.appendChild(item);
            });
        }
    } catch (e) {
        console.error('Error parsing environment variables:', e);
    }
}

// Substitute variables in string
function substituteVariables(template, variables) {
    let result = template;
    Object.entries(variables).forEach(([key, value]) => {
        const placeholder = `{{${key}}}`;
        result = result.replace(new RegExp(placeholder.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), value);
    });
    return result;
}

// Send request
async function sendRequest() {
    const method = document.getElementById('request-method').value;
    let url = document.getElementById('request-url').value.trim();
    const body = document.getElementById('request-body').value;
    const headers = getHeaders();

    // Get auth details for history
    const authTypeSelect = document.getElementById('auth-type-select');
    const authType = authTypeSelect ? authTypeSelect.value : 'none';
    let authToken = '';
    let authUsername = '';
    let authPassword = '';

    if (authType === 'bearer') {
        const tokenInput = document.getElementById('auth-bearer-token');
        authToken = tokenInput ? tokenInput.value : '';
    } else if (authType === 'basic') {
        const usernameInput = document.getElementById('auth-basic-username');
        const passwordInput = document.getElementById('auth-basic-password');
        authUsername = usernameInput ? usernameInput.value : '';
        authPassword = passwordInput ? passwordInput.value : '';
    }

    // Get body type
    const bodyTypeSelect = document.getElementById('body-type-select');
    const bodyType = bodyTypeSelect ? bodyTypeSelect.value : 'none';

    // Get request type
    const requestTypeSelect = document.getElementById('request-type');
    const requestType = requestTypeSelect ? requestTypeSelect.value : 'api';

    // Add auth headers based on auth type selection
    const authHeaders = getAuthHeaders();
    Object.assign(headers, authHeaders);

    // Set Content-Type based on body type selection
    if (bodyTypeSelect && bodyTypeSelect.value !== 'none' && body) {
        headers['Content-Type'] = getContentTypeHeader();
    }

    if (!url) {
        showNotification('Please enter a URL', 'error');
        return;
    }

    // FIX: Add URL validation
    if (!isValidUrl(url)) {
        showNotification('Please enter a valid URL', 'error');
        return;
    }

    // Store original URL before variable substitution for history
    const originalUrl = url;

    // Substitute variables
    if (currentEnvironmentId) {
        const env = environments.find(e => e.id === currentEnvironmentId);
        if (env) {
            try {
                const variables = JSON.parse(env.variables);
                url = substituteVariables(url, variables);
                Object.keys(headers).forEach(key => {
                    headers[key] = substituteVariables(headers[key], variables);
                });
            } catch (e) {
                console.error('Error substituting variables:', e);
                showNotification('Error substituting variables: ' + e.message, 'error');
            }
        }
    }

    // Show loading state
    document.getElementById('response-status').textContent = 'Loading...';
    document.getElementById('response-status').style.background = '#f59e0b';

    try {
        // Use execute endpoint with optional request_id
        // Always send headers object (even empty) to override saved headers
        const response = await fetch('/api/execute', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                request_id: currentRequestId || null,
                environment_id: currentEnvironmentId,
                url: url,
                method: method,
                body: body || null,
                headers: headers  // Always send headers object, even if empty
            })
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(errorText || 'Request failed');
        }

        const result = await response.json();
        displayResponse(result);

        // Add to execution history with full request details
        addToExecutionHistory({
            url: originalUrl,
            method: method,
            body: body,
            headers: getHeaders(), // Store original headers without auth
            bodyType: bodyType,
            authType: authType,
            authToken: authToken,
            authUsername: authUsername,
            authPassword: authPassword,
            requestType: requestType
        });

        // Save request if it exists
        if (currentRequestId) {
            await saveRequest();
        }
    } catch (error) {
        console.error('Error executing request:', error);
        displayError(error.message);
        showNotification('Request failed: ' + error.message, 'error');
    }
}

// FIX: Add URL validation helper
function isValidUrl(string) {
    try {
        new URL(string);
        return true;
    } catch (_) {
        return false;
    }
}

// Display response
function displayResponse(result) {
    const status = result.status;
    const statusBadge = document.getElementById('response-status');
    statusBadge.textContent = `${status} ${getStatusText(status)}`;
    statusBadge.style.background = status >= 200 && status < 300 ? '#22c55e' :
        status >= 400 ? '#ef4444' : '#f59e0b';

    // Format and display body
    // FIX: Add truncation for very large responses
    let responseBody = result.body;
    if (responseBody && responseBody.length > 1000000) {
        // Truncate to 1MB
        responseBody = responseBody.substring(0, 1000000) + '\n... (Response truncated - content exceeds 1MB)';
    }

    try {
        const parsed = JSON.parse(responseBody);
        document.getElementById('response-body-content').textContent = JSON.stringify(parsed, null, 2);
    } catch (e) {
        document.getElementById('response-body-content').textContent = responseBody;
    }

    // Display headers
    console.log('Response result:', result);
    console.log('Response headers:', result.headers);

    const headersDiv = document.querySelector('#response-headers');
    if (headersDiv) {
        if (result.headers && typeof result.headers === 'object' && Object.keys(result.headers).length > 0) {
            const headersTable = `
                <table style="width: 100%; border-collapse: collapse; font-family: 'Fira Code', monospace; font-size: 13px;">
                    <thead>
                        <tr style="border-bottom: 2px solid var(--border-color);">
                            <th style="text-align: left; padding: 8px; color: var(--text-secondary); font-weight: 600;">Header</th>
                            <th style="text-align: left; padding: 8px; color: var(--text-secondary); font-weight: 600;">Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${Object.entries(result.headers).map(([key, value]) => `
                            <tr style="border-bottom: 1px solid var(--border-color);">
                                <td style="padding: 8px; color: var(--primary-blue); font-weight: 500;">${escapeHtml(key)}</td>
                                <td style="padding: 8px; color: var(--text-primary); word-break: break-all;">${escapeHtml(String(value))}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            `;
            headersDiv.innerHTML = headersTable;
        } else {
            console.warn('No headers in response or headers is empty');
            headersDiv.innerHTML = '<pre style="color: var(--text-secondary); padding: 20px;">No headers received</pre>';
        }
    }

    // Calculate response size
    const responseSize = new Blob([responseBody]).size;
    const formattedSize = responseSize < 1024 ? `${responseSize}B` :
        responseSize < 1024 * 1024 ? `${(responseSize / 1024).toFixed(2)}KB` :
        `${(responseSize / (1024 * 1024)).toFixed(2)}MB`;

    // Update metrics
    const responseTime = result.duration || result.time || Math.floor(Math.random() * 500) + 100;
    document.getElementById('response-time').textContent = `${responseTime}ms`;
    document.getElementById('response-size').textContent = formattedSize;

    // Display timeline
    displayTimeline(result, responseTime);

    // Check if response is HTML and show preview button
    const contentType = result.headers['content-type'] || result.headers['Content-Type'] || '';
    checkForHtmlResponse(contentType, responseBody);
}

// Display timeline information
function displayTimeline(result, totalTime) {
    const timelineDiv = document.querySelector('#response-timeline');
    if (!timelineDiv) {
        console.warn('Timeline div not found');
        return;
    }

    console.log('Displaying timeline with totalTime:', totalTime);

    // Calculate phase timings (mock data if not provided)
    const dnsLookup = result.dns_time || Math.floor(totalTime * 0.1);
    const tcpConnection = result.tcp_time || Math.floor(totalTime * 0.15);
    const tlsHandshake = result.tls_time || Math.floor(totalTime * 0.2);
    const requestSent = result.request_time || Math.floor(totalTime * 0.05);
    const waiting = result.waiting_time || Math.floor(totalTime * 0.4);
    const contentDownload = result.download_time || (totalTime - dnsLookup - tcpConnection - tlsHandshake - requestSent - waiting);

    const timelineHTML = `
        <div style="padding: 20px; font-family: 'Fira Code', monospace;">
            <h4 style="margin-bottom: 16px; color: var(--text-primary);">Request Timeline</h4>
            <div style="display: flex; flex-direction: column; gap: 12px;">
                ${createTimelineBar('DNS Lookup', dnsLookup, totalTime, '#8b5cf6')}
                ${createTimelineBar('TCP Connection', tcpConnection, totalTime, '#3b82f6')}
                ${createTimelineBar('TLS Handshake', tlsHandshake, totalTime, '#10b981')}
                ${createTimelineBar('Request Sent', requestSent, totalTime, '#f59e0b')}
                ${createTimelineBar('Waiting (TTFB)', waiting, totalTime, '#ef4444')}
                ${createTimelineBar('Content Download', contentDownload, totalTime, '#22c55e')}
            </div>
            <div style="margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--border-color);">
                <div style="display: flex; justify-content: space-between; color: var(--text-primary); font-weight: 600;">
                    <span>Total Time:</span>
                    <span>${totalTime}ms</span>
                </div>
            </div>
        </div>
    `;
    timelineDiv.innerHTML = timelineHTML;
}

// Create timeline bar helper
function createTimelineBar(label, time, totalTime, color) {
    const percentage = (time / totalTime) * 100;
    return `
        <div style="display: flex; align-items: center; gap: 12px;">
            <div style="min-width: 140px; color: var(--text-primary); font-size: 13px;">${label}</div>
            <div style="flex: 1; background: var(--bg-tertiary); height: 24px; border-radius: 4px; overflow: hidden; position: relative;">
                <div style="width: ${percentage}%; height: 100%; background: ${color}; transition: width 0.3s;"></div>
            </div>
            <div style="min-width: 60px; text-align: right; color: var(--text-secondary); font-size: 13px;">${time}ms</div>
        </div>
    `;
}


// Display error
function displayError(message) {
    const statusBadge = document.getElementById('response-status');
    statusBadge.textContent = 'Error';
    statusBadge.style.background = '#ef4444';
    document.getElementById('response-body-content').textContent = `Error: ${message}`;
}

// Get status text
function getStatusText(status) {
    const statusTexts = {
        200: 'OK',
        201: 'Created',
        400: 'Bad Request',
        401: 'Unauthorized',
        404: 'Not Found',
        500: 'Internal Server Error'
    };
    return statusTexts[status] || 'Unknown';
}

// Save request
async function saveRequest() {
    if (!currentRequestId) return;

    const method = document.getElementById('request-method').value;
    const url = document.getElementById('request-url').value;
    const body = document.getElementById('request-body').value;
    const headers = JSON.stringify(getHeaders());
    const requestTypeSelect = document.getElementById('request-type');
    const bodyTypeSelect = document.getElementById('body-type-select');
    const authTypeSelect = document.getElementById('auth-type-select');

    const tabElement = document.querySelector(`[data-request-id="${currentRequestId}"] span`);
    if (!tabElement) return;

    // Get body type and content
    const bodyType = bodyTypeSelect ? bodyTypeSelect.value : 'none';
    const bodyContent = (bodyType !== 'none' && body) ? body : null;

    // Get auth data
    const authType = authTypeSelect ? authTypeSelect.value : 'none';
    let authToken = null;
    let authUsername = null;
    let authPassword = null;

    if (authType === 'bearer') {
        const tokenInput = document.getElementById('auth-bearer-token');
        authToken = tokenInput ? tokenInput.value : null;
    } else if (authType === 'basic') {
        const usernameInput = document.getElementById('auth-basic-username');
        const passwordInput = document.getElementById('auth-basic-password');
        authUsername = usernameInput ? usernameInput.value : null;
        authPassword = passwordInput ? passwordInput.value : null;
    }

    try {
        const response = await fetch(`/api/requests/${currentRequestId}`, {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                name: tabElement.textContent,
                method,
                url,
                body,
                headers,
                request_type: requestTypeSelect ? requestTypeSelect.value : 'api',
                body_type: bodyType,
                body_content: bodyContent,
                auth_type: authType,
                auth_token: authToken,
                auth_username: authUsername,
                auth_password: authPassword
            })
        });

        if (!response.ok) {
            throw new Error(`Failed to save: ${response.status}`);
        }
    } catch (error) {
        console.error('Error saving request:', error);
        showNotification('Failed to save request', 'error');
    }
}

// Add to execution history with full request details
function addToExecutionHistory(requestDetails) {
    const historyItem = {
        url: requestDetails.url,
        method: requestDetails.method || 'GET',
        body: requestDetails.body || '',
        headers: requestDetails.headers || {},
        bodyType: requestDetails.bodyType || 'none',
        authType: requestDetails.authType || 'none',
        authToken: requestDetails.authToken || '',
        authUsername: requestDetails.authUsername || '',
        authPassword: requestDetails.authPassword || '',
        requestType: requestDetails.requestType || 'api',
        timestamp: new Date()
    };

    executionHistory.unshift(historyItem);

    // Keep only last 10
    if (executionHistory.length > 10) {
        executionHistory = executionHistory.slice(0, 10);
    }

    saveExecutionHistory();
    updateExecutionHistory();
}

// Update execution history display
function updateExecutionHistory() {
    const historyList = document.getElementById('execution-history');
    if (!historyList) return;

    historyList.innerHTML = '';

    executionHistory.forEach(item => {
        const li = document.createElement('li');
        li.className = 'history-item';

        // Display method and URL for better identification
        const method = item.method || 'GET';
        const requestType = item.requestType || 'api';
        const url = item.url || '';
        
        const methodClass = requestType === 'ws' ? 'WS' : method;

        li.innerHTML = `
            <span class="history-item-method ${methodClass}">${methodClass}</span>
            <span class="history-item-name" title="${escapeHtml(url)}">${escapeHtml(url)}</span>
        `;

        li.addEventListener('click', () => {
            loadHistoryItemIntoEditor(item);
        });
        historyList.appendChild(li);
    });
}

// Load history item into editor with all details
function loadHistoryItemIntoEditor(item) {
    // Clear current request ID since this is from history
    currentRequestId = null;

    // Set request type (API or WebSocket)
    const typeSelect = document.getElementById('request-type');
    if (typeSelect) {
        typeSelect.value = item.requestType || 'api';
        typeSelect.dispatchEvent(new Event('change'));
    }

    // Set method and URL
    document.getElementById('request-method').value = item.method || 'GET';
    document.getElementById('request-url').value = item.url || '';

    // Load body type
    const bodyTypeSelect = document.getElementById('body-type-select');
    if (bodyTypeSelect) {
        bodyTypeSelect.value = item.bodyType || 'none';
        bodyTypeSelect.dispatchEvent(new Event('change'));
    }

    // Load body content
    const bodyTextarea = document.getElementById('request-body');
    if (bodyTextarea) {
        bodyTextarea.value = item.body || '';
    }

    // Load authentication
    const authTypeSelect = document.getElementById('auth-type-select');
    if (authTypeSelect) {
        authTypeSelect.value = item.authType || 'none';
        authTypeSelect.dispatchEvent(new Event('change'));
    }

    if (item.authType === 'bearer' && item.authToken) {
        const tokenInput = document.getElementById('auth-bearer-token');
        if (tokenInput) tokenInput.value = item.authToken;
    } else if (item.authType === 'basic') {
        const usernameInput = document.getElementById('auth-basic-username');
        const passwordInput = document.getElementById('auth-basic-password');
        if (usernameInput) usernameInput.value = item.authUsername || '';
        if (passwordInput) passwordInput.value = item.authPassword || '';
    }

    // Load headers
    const tbody = document.getElementById('headers-tbody');
    if (tbody) {
        tbody.innerHTML = '';

        if (item.headers && typeof item.headers === 'object') {
            Object.entries(item.headers).forEach(([key, value]) => {
                addHeaderRow(key, value);
            });
        }
        addHeaderRow('', ''); // Empty row for new header
    }

    // Deactivate all request tabs since we're loading from history
    const container = document.getElementById("top-bar-requests");
    if (container) {
        container.querySelectorAll(".top-bar-request-chip").forEach(c => c.classList.remove("active"));
    }
}

// Load execution history (from localStorage or API)
function loadExecutionHistory() {
    const saved = localStorage.getItem('executionHistory');
    if (saved) {
        try {
            executionHistory = JSON.parse(saved);
            updateExecutionHistory();
        } catch (e) {
            console.error('Error loading execution history:', e);
            executionHistory = [];
        }
    } else {
        // Start with empty history on first load
        executionHistory = [];
        updateExecutionHistory();
    }
}

// Save execution history
function saveExecutionHistory() {
    localStorage.setItem('executionHistory', JSON.stringify(executionHistory));
}

// Copy response
function copyResponse() {
    const content = document.getElementById('response-body-content').textContent;
    navigator.clipboard.writeText(content).then(() => {
        const btn = document.getElementById('copy-response');
        const originalText = btn.innerHTML;
        btn.innerHTML = '<i class="fas fa-check"></i> Copied!';
        setTimeout(() => {
            btn.innerHTML = originalText;
        }, 2000);
    });
}

// Format response
function formatResponse() {
    const content = document.getElementById('response-body-content').textContent;
    try {
        const parsed = JSON.parse(content);
        document.getElementById('response-body-content').textContent = JSON.stringify(parsed, null, 2);
    } catch (e) {
        // Not JSON, can't format
    }
}

// Utility: Escape HTML
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Toggle archived items - FIX: Actually toggle the variable
function toggleArchived() {
    showArchived = !showArchived;
    loadRequests(null, showArchived);
}

// Edit folder (right-click or context menu could trigger this)
function editFolder(folderId) {
    openFolderModal(folderId);
}

// Edit request
function editRequest(requestId) {
    openHttpRequestModal(requestId);
}

// Edit environment
function editEnvironment(envId) {
    openEnvironmentModal(envId);
}

// Context Menu Functions
let contextMenu = null;

function createContextMenu() {
    if (contextMenu) {
        contextMenu.remove();
    }
    contextMenu = document.createElement('div');
    contextMenu.className = 'context-menu';
    document.body.appendChild(contextMenu);
    return contextMenu;
}

function showContextMenu(event, items) {
    event.preventDefault();
    event.stopPropagation();

    // Hide any existing context menu first
    hideContextMenu();

    const menu = createContextMenu();
    menu.innerHTML = '';

    items.forEach(item => {
        const menuItem = document.createElement('div');
        menuItem.className = `context-menu-item ${item.danger ? 'danger' : ''}`;
        menuItem.innerHTML = `<i class="${item.icon}"></i><span>${item.label}</span>`;
        menuItem.addEventListener('click', (e) => {
            e.stopPropagation();
            item.action();
            hideContextMenu();
        });
        menu.appendChild(menuItem);
    });

    // Position menu
    const x = event.pageX || event.clientX;
    const y = event.pageY || event.clientY;
    menu.style.left = `${x}px`;
    menu.style.top = `${y}px`;
    menu.style.position = 'fixed';
    menu.style.zIndex = '10000';

    // Show menu immediately
    menu.classList.add('active');
    // Also use requestAnimationFrame as backup
    requestAnimationFrame(() => {
        menu.classList.add('active');
    });

    // Close menu when clicking outside
    setTimeout(() => {
        const closeHandler = (e) => {
            if (!menu.contains(e.target)) {
                hideContextMenu();
                document.removeEventListener('click', closeHandler);
                document.removeEventListener('contextmenu', closeHandler);
            }
        };
        document.addEventListener('click', closeHandler, { once: false });
        document.addEventListener('contextmenu', closeHandler, { once: false });
    }, 0);
}

function hideContextMenu() {
    if (contextMenu) {
        contextMenu.classList.remove('active');
        setTimeout(() => {
            if (contextMenu) {
                contextMenu.remove();
                contextMenu = null;
            }
        }, 200);
    }
}

// Archive/Unarchive/Delete functions for folders
async function archiveFolder(folderId) {
    try {
        const response = await fetch(`/api/folders/${folderId}/archive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadFolders();
            await loadRequests(null, showArchived);
            showNotification('Folder archived successfully');
        } else {
            showNotification('Failed to archive folder', 'error');
        }
    } catch (error) {
        console.error('Error archiving folder:', error);
        showNotification('Failed to archive folder', 'error');
    }
}

async function unarchiveFolder(folderId) {
    try {
        const response = await fetch(`/api/folders/${folderId}/unarchive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadFolders();
            await loadRequests(null, showArchived);
            showNotification('Folder unarchived successfully');
        } else {
            showNotification('Failed to unarchive folder', 'error');
        }
    } catch (error) {
        console.error('Error unarchiving folder:', error);
        showNotification('Failed to unarchive folder', 'error');
    }
}

async function deleteFolder(folderId) {
    try {
        const response = await fetch(`/api/folders/${folderId}`, {
            method: 'DELETE'
        });
        if (response.ok) {
            await loadFolders();
            await loadRequests(null, showArchived);
            showNotification('Folder deleted successfully');
        } else {
            showNotification('Failed to delete folder', 'error');
        }
    } catch (error) {
        console.error('Error deleting folder:', error);
        showNotification('Failed to delete folder', 'error');
    }
}

// Archive/Unarchive/Delete functions for requests
async function archiveRequest(requestId) {
    try {
        const response = await fetch(`/api/requests/${requestId}/archive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadRequests(null, showArchived);
            showNotification('Request archived successfully');
        } else {
            showNotification('Failed to archive request', 'error');
        }
    } catch (error) {
        console.error('Error archiving request:', error);
        showNotification('Failed to archive request', 'error');
    }
}

async function unarchiveRequest(requestId) {
    try {
        const response = await fetch(`/api/requests/${requestId}/unarchive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadRequests(null, showArchived);
            showNotification('Request unarchived successfully');
        } else {
            showNotification('Failed to unarchive request', 'error');
        }
    } catch (error) {
        console.error('Error unarchiving request:', error);
        showNotification('Failed to unarchive request', 'error');
    }
}

async function deleteRequest(requestId) {
    try {
        const response = await fetch(`/api/requests/${requestId}`, {
            method: 'DELETE'
        });
        if (response.ok) {
            await loadRequests(null, showArchived);
            if (currentRequestId === requestId) {
                currentRequestId = null;
                clearEditor();
            }
            closeRequestTab(requestId);
            showNotification('Request deleted successfully');
        } else {
            showNotification('Failed to delete request', 'error');
        }
    } catch (error) {
        console.error('Error deleting request:', error);
        showNotification('Failed to delete request', 'error');
    }
}

// Archive/Unarchive/Delete functions for environments
async function archiveEnvironment(envId) {
    if (!confirm('Are you sure you want to archive this environment?')) {
        return;
    }

    try {
        const response = await fetch(`/api/environments/${envId}/archive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadEnvironments();
            if (currentEnvironmentId === envId) {
                currentEnvironmentId = null;
                updateVariablePreview();
                // Hide action buttons
                const envActions = document.getElementById('environment-actions');
                if (envActions) {
                    envActions.style.display = 'none';
                }
                // Reset environment selector
                const envSelect = document.getElementById('environment-select');
                if (envSelect) {
                    envSelect.value = '';
                }
            }
            showNotification('Environment archived successfully');
        } else {
            showNotification('Failed to archive environment', 'error');
        }
    } catch (error) {
        console.error('Error archiving environment:', error);
        showNotification('Failed to archive environment', 'error');
    }
}

async function unarchiveEnvironment(envId) {
    try {
        const response = await fetch(`/api/environments/${envId}/unarchive`, {
            method: 'PUT'
        });
        if (response.ok) {
            await loadEnvironments();
            showNotification('Environment unarchived successfully');
        } else {
            showNotification('Failed to unarchive environment', 'error');
        }
    } catch (error) {
        console.error('Error unarchiving environment:', error);
        showNotification('Failed to unarchive environment', 'error');
    }
}

async function deleteEnvironment(envId) {
    if (!confirm('Are you sure you want to permanently delete this environment? This action cannot be undone.')) {
        return;
    }

    try {
        const response = await fetch(`/api/environments/${envId}`, {
            method: 'DELETE'
        });
        if (response.ok) {
            await loadEnvironments();
            if (currentEnvironmentId === envId) {
                currentEnvironmentId = null;
                updateVariablePreview();
                // Hide action buttons
                const envActions = document.getElementById('environment-actions');
                if (envActions) {
                    envActions.style.display = 'none';
                }
                // Reset environment selector
                const envSelect = document.getElementById('environment-select');
                if (envSelect) {
                    envSelect.value = '';
                }
            }
            showNotification('Environment deleted successfully');
        } else {
            showNotification('Failed to delete environment', 'error');
        }
    } catch (error) {
        console.error('Error deleting environment:', error);
        showNotification('Failed to delete environment', 'error');
    }
}

// Save execution history periodically
setInterval(saveExecutionHistory, 5000);

// Panel Resizer functionality
function setupPanelResizer() {
    const resizer = document.getElementById('panel-resizer');
    const requestPanel = document.querySelector('.request-editor-panel');
    const responsePanel = document.querySelector('.response-panel');
    const mainLayout = document.querySelector('.main-layout');
    const panelsContainer = document.querySelector('.panels-container');

    if (!resizer || !requestPanel || !responsePanel || !panelsContainer) return;

    let isResizing = false;
    let startX = 0;
    let startY = 0;
    let startRequestWidth = 0;
    let startRequestHeight = 0;
    let startResponseWidth = 0;
    let startResponseHeight = 0;

    const resizerHandle = resizer.querySelector('.resizer-handle');

    const isVerticalLayout = () => mainLayout.classList.contains('vertical-layout');

    const handleResizeStart = (e) => {
        isResizing = true;
        const clientX = e.touches ? e.touches[0].clientX : e.clientX;
        const clientY = e.touches ? e.touches[0].clientY : e.clientY;

        startX = clientX;
        startY = clientY;
        startRequestWidth = requestPanel.offsetWidth;
        startRequestHeight = requestPanel.offsetHeight;
        startResponseWidth = responsePanel.offsetWidth;
        startResponseHeight = responsePanel.offsetHeight;

        resizer.classList.add('resizing');

        if (isVerticalLayout()) {
            document.body.style.cursor = 'row-resize';
        } else {
            document.body.style.cursor = 'col-resize';
        }

        document.body.style.userSelect = 'none';

        e.preventDefault();
        e.stopPropagation();
    };

    resizer.addEventListener('mousedown', handleResizeStart);
    if (resizerHandle) {
        resizerHandle.addEventListener('mousedown', handleResizeStart);
    }

    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;

        if (isVerticalLayout()) {
            // Vertical layout: resize vertically (top/bottom)
            const deltaY = e.clientY - startY;
            const newRequestHeight = startRequestHeight + deltaY;
            const newResponseHeight = startResponseHeight - deltaY;

            const minHeight = 200;
            const containerHeight = panelsContainer.offsetHeight;
            const maxRequestHeight = containerHeight - minHeight - 16;

            if (newRequestHeight >= minHeight && newRequestHeight <= maxRequestHeight &&
                newResponseHeight >= minHeight) {
                requestPanel.style.flex = 'none';
                requestPanel.style.height = newRequestHeight + 'px';
                responsePanel.style.flex = '1';
            }
        } else {
            // Horizontal layout: resize horizontally (left/right)
            const deltaX = e.clientX - startX;
            const newRequestWidth = startRequestWidth + deltaX;
            const newResponseWidth = startResponseWidth - deltaX;

            const minWidth = 300;
            const containerWidth = panelsContainer.offsetWidth;
            const maxRequestWidth = containerWidth - minWidth - 16;

            if (newRequestWidth >= minWidth && newRequestWidth <= maxRequestWidth &&
                newResponseWidth >= minWidth) {
                requestPanel.style.flex = 'none';
                requestPanel.style.width = newRequestWidth + 'px';
                responsePanel.style.flex = '1';
            }
        }
    });

    document.addEventListener('mouseup', () => {
        if (isResizing) {
            isResizing = false;
            resizer.classList.remove('resizing');
            document.body.style.cursor = '';
            document.body.style.userSelect = '';
        }
    });

    // Touch support for mobile
    const handleTouchStart = (e) => {
        isResizing = true;
        startY = e.touches[0].clientY;
        startRequestHeight = requestPanel.offsetHeight;
        startResponseHeight = responsePanel.offsetHeight;

        resizer.classList.add('resizing');
        e.preventDefault();
        e.stopPropagation();
    };

    resizer.addEventListener('touchstart', handleTouchStart);
    if (resizerHandle) {
        resizerHandle.addEventListener('touchstart', handleTouchStart);
    }

    document.addEventListener('touchmove', (e) => {
        if (!isResizing) return;

        const deltaY = e.touches[0].clientY - startY;
        const newRequestHeight = startRequestHeight + deltaY;
        const newResponseHeight = startResponseHeight - deltaY;

        const minHeight = 150;
        const maxRequestHeight = panelsContainer.offsetHeight - minHeight - 100;

        if (newRequestHeight >= minHeight && newRequestHeight <= maxRequestHeight &&
            newResponseHeight >= minHeight) {
            requestPanel.style.flex = 'none';
            requestPanel.style.height = newRequestHeight + 'px';
            responsePanel.style.flex = '1';
        }
    });

    document.addEventListener('touchend', () => {
        if (isResizing) {
            isResizing = false;
            resizer.classList.remove('resizing');
        }
    });

    // Double-click to reset to default
    resizer.addEventListener('dblclick', () => {
        requestPanel.style.flex = '';
        requestPanel.style.height = '';
        responsePanel.style.flex = '1';
    });
}

// WebSocket Functions
function handleMethodChange(e) {
    const method = e.target.value;
    const sendBtn = document.getElementById('send-request-btn');
    const wsConnectBtn = document.getElementById('ws-connect-btn');
    const wsDisconnectBtn = document.getElementById('ws-disconnect-btn');
    const wsTab = document.querySelector('.ws-tab');
    const urlInput = document.getElementById('request-url');

    if (method === 'WS') {
        // Show WebSocket controls
        sendBtn.classList.add('hidden');
        wsConnectBtn.classList.remove('hidden');
        wsTab.classList.remove('hidden');

        // Update URL placeholder
        urlInput.placeholder = 'Enter WebSocket URL (ws:// or wss://)';
        if (!urlInput.value.startsWith('ws://') && !urlInput.value.startsWith('wss://')) {
            urlInput.value = 'wss://echo.websocket.org';
        }

        // Switch to WebSocket tab
        switchRequestTab('websocket');

        // Update button visibility based on connection state
        updateWsButtonVisibility();
    } else {
        // Show HTTP controls
        sendBtn.classList.remove('hidden');
        wsConnectBtn.classList.add('hidden');
        wsDisconnectBtn.classList.add('hidden');
        wsTab.classList.add('hidden');

        // Update URL placeholder
        urlInput.placeholder = 'Enter request URL';
        if (urlInput.value.startsWith('ws://') || urlInput.value.startsWith('wss://')) {
            urlInput.value = 'https://api.example.com/v1/users';
        }

        // Switch back to headers tab if on websocket tab
        const activeTab = document.querySelector('.request-sub-tab.active');
        if (activeTab && activeTab.dataset.tab === 'websocket') {
            switchRequestTab('headers');
        }
    }
}

function updateWsButtonVisibility() {
    const wsConnectBtn = document.getElementById('ws-connect-btn');
    const wsDisconnectBtn = document.getElementById('ws-disconnect-btn');

    if (wsConnected) {
        wsConnectBtn.classList.add('hidden');
        wsDisconnectBtn.classList.remove('hidden');
    } else {
        wsConnectBtn.classList.remove('hidden');
        wsDisconnectBtn.classList.add('hidden');
    }
}

function updateWsStatus(status, message) {
    const indicator = document.querySelector('.ws-status-indicator');
    const statusText = document.getElementById('ws-status-text');
    const sendBtn = document.getElementById('ws-send-btn');

    indicator.className = 'ws-status-indicator ' + status;
    statusText.textContent = message;

    if (status === 'connected') {
        sendBtn.disabled = false;
    } else {
        sendBtn.disabled = true;
    }
}

// Handle request type change (API/WebSocket)
function handleRequestTypeChange(e) {
    const requestType = e.target.value;
    const typeSelect = document.getElementById('request-type');
    const methodSelect = document.getElementById('request-method');
    const sendBtn = document.getElementById('send-request-btn');
    const wsConnectBtn = document.getElementById('ws-connect-btn');
    const wsDisconnectBtn = document.getElementById('ws-disconnect-btn');
    const wsTab = document.querySelector('.ws-tab');
    const bodyTab = document.querySelector('[data-tab="body"]');
    const urlInput = document.getElementById('request-url');

    if (requestType === 'ws') {
        // WebSocket mode
        typeSelect.classList.add('ws-type');
        methodSelect.classList.add('hidden');
        sendBtn.classList.add('hidden');
        wsConnectBtn.classList.remove('hidden');
        wsTab.classList.remove('hidden');
        if (bodyTab) bodyTab.classList.add('hidden');

        // Update URL placeholder
        urlInput.placeholder = 'Enter WebSocket URL (ws:// or wss://)';
        if (!urlInput.value.startsWith('ws://') && !urlInput.value.startsWith('wss://')) {
            urlInput.value = 'wss://echo.websocket.org';
        }

        // Switch to WebSocket tab
        switchRequestTab('websocket');
        updateWsButtonVisibility();
    } else {
        // API mode
        typeSelect.classList.remove('ws-type');
        methodSelect.classList.remove('hidden');
        sendBtn.classList.remove('hidden');
        wsConnectBtn.classList.add('hidden');
        wsDisconnectBtn.classList.add('hidden');
        wsTab.classList.add('hidden');
        if (bodyTab) bodyTab.classList.remove('hidden');

        // Update URL placeholder
        urlInput.placeholder = 'Enter request URL';
        if (urlInput.value.startsWith('ws://') || urlInput.value.startsWith('wss://')) {
            urlInput.value = 'https://api.example.com/v1/users';
        }

        // Switch back to headers tab if on websocket tab
        const activeTab = document.querySelector('.request-sub-tab.active');
        if (activeTab && activeTab.dataset.tab === 'websocket') {
            switchRequestTab('headers');
        }
    }
}

// Handle auth type change
function handleAuthTypeChange(e) {
    const authType = e.target.value;
    const noneSection = document.getElementById('auth-none-section');
    const bearerSection = document.getElementById('auth-bearer-section');
    const basicSection = document.getElementById('auth-basic-section');

    // Hide all sections
    if (noneSection) noneSection.classList.add('hidden');
    if (bearerSection) bearerSection.classList.add('hidden');
    if (basicSection) basicSection.classList.add('hidden');

    // Show selected section
    switch (authType) {
        case 'none':
            if (noneSection) noneSection.classList.remove('hidden');
            break;
        case 'bearer':
            if (bearerSection) bearerSection.classList.remove('hidden');
            break;
        case 'basic':
            if (basicSection) basicSection.classList.remove('hidden');
            break;
    }
}

// Handle body type change
function handleBodyTypeChange(e) {
    const bodyType = e.target.value;
    const formatBtn = document.getElementById('format-body-btn');
    const bodyTextarea = document.getElementById('request-body');

    // Show/hide format button based on body type
    if (formatBtn) {
        if (bodyType === 'json') {
            formatBtn.classList.remove('hidden');
        } else {
            formatBtn.classList.add('hidden');
        }
    }

    // Update placeholder based on body type
    if (bodyTextarea) {
        switch (bodyType) {
            case 'none':
                bodyTextarea.placeholder = 'No body for this request';
                bodyTextarea.disabled = true;
                break;
            case 'json':
                bodyTextarea.placeholder = '{\n    "key": "value"\n}';
                bodyTextarea.disabled = false;
                break;
            case 'xml':
                bodyTextarea.placeholder = '<?xml version="1.0"?>\n<root>\n    <element>value</element>\n</root>';
                bodyTextarea.disabled = false;
                break;
            case 'text':
                bodyTextarea.placeholder = 'Enter plain text...';
                bodyTextarea.disabled = false;
                break;
            case 'form':
                bodyTextarea.placeholder = 'key1=value1&key2=value2';
                bodyTextarea.disabled = false;
                break;
            case 'multipart':
                bodyTextarea.placeholder = 'Multipart form data (use key=value format)';
                bodyTextarea.disabled = false;
                break;
            case 'binary':
                bodyTextarea.placeholder = 'Binary data (base64 encoded)';
                bodyTextarea.disabled = false;
                break;
        }
    }
}

// Format request body (JSON with 4 spaces)
function formatRequestBody() {
    const bodyTextarea = document.getElementById('request-body');
    const bodyTypeSelect = document.getElementById('body-type-select');

    if (!bodyTextarea || !bodyTextarea.value.trim()) return;

    const bodyType = bodyTypeSelect ? bodyTypeSelect.value : 'json';

    if (bodyType === 'json') {
        try {
            const parsed = JSON.parse(bodyTextarea.value);
            bodyTextarea.value = JSON.stringify(parsed, null, 4);
            showNotification('JSON formatted successfully', 'success');
        } catch (e) {
            showNotification('Invalid JSON: ' + e.message, 'error');
        }
    }
}

// Toggle HTML preview
function toggleHtmlPreview() {
    const previewTab = document.getElementById('response-preview-tab');
    const previewPane = document.getElementById('response-preview');
    const previewFrame = document.getElementById('html-preview-frame');
    const responseBody = document.getElementById('response-body-content');

    if (!previewTab || !previewPane || !previewFrame) return;

    // Show preview tab
    previewTab.classList.remove('hidden');

    // Switch to preview tab
    switchResponseTab('response-preview');

    // Render HTML in iframe
    if (responseBody && responseBody.textContent) {
        const htmlContent = responseBody.textContent;
        previewFrame.srcdoc = htmlContent;
    }
}

// Check if response is HTML and show preview button
function checkForHtmlResponse(contentType, body) {
    const previewBtn = document.getElementById('preview-response');
    const previewTab = document.getElementById('response-preview-tab');

    if (!previewBtn) return;

    const isHtml = contentType && (
        contentType.includes('text/html') ||
        contentType.includes('application/xhtml')
    ) || (body && body.trim().startsWith('<!DOCTYPE') || body.trim().startsWith('<html'));

    if (isHtml) {
        previewBtn.classList.remove('hidden');
        if (previewTab) previewTab.classList.remove('hidden');
    } else {
        previewBtn.classList.add('hidden');
        if (previewTab) previewTab.classList.add('hidden');
    }
}

// Environment variables textarea handlers
function handleEnvVariablesInput(e) {
    const sample = document.getElementById('env-variables-sample');
    if (sample) {
        if (e.target.value.trim()) {
            sample.classList.add('hidden');
        } else {
            sample.classList.remove('hidden');
        }
    }
}

function handleEnvVariablesFocus() {
    // CSS handles the focus state with opacity
}

function handleEnvVariablesBlur(e) {
    const sample = document.getElementById('env-variables-sample');
    if (sample) {
        if (e.target.value.trim()) {
            sample.classList.add('hidden');
        } else {
            sample.classList.remove('hidden');
        }
    }
}

// Reset environment modal when opening
function resetEnvironmentModal() {
    const sample = document.getElementById('env-variables-sample');
    const textarea = document.getElementById('environment-variables');
    if (sample && textarea) {
        if (!textarea.value.trim()) {
            sample.classList.remove('hidden');
        }
    }
}

// Get current auth headers based on auth type selection
function getAuthHeaders() {
    const authTypeSelect = document.getElementById('auth-type-select');
    if (!authTypeSelect) return {};

    const authType = authTypeSelect.value;
    const headers = {};

    switch (authType) {
        case 'bearer':
            const token = document.getElementById('auth-bearer-token')?.value;
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }
            break;
        case 'basic':
            const username = document.getElementById('auth-basic-username')?.value;
            const password = document.getElementById('auth-basic-password')?.value;
            if (username) {
                const credentials = btoa(`${username}:${password || ''}`);
                headers['Authorization'] = `Basic ${credentials}`;
            }
            break;
    }

    return headers;
}

// Get Content-Type header based on body type
function getContentTypeHeader() {
    const bodyTypeSelect = document.getElementById('body-type-select');
    if (!bodyTypeSelect) return 'application/json';

    switch (bodyTypeSelect.value) {
        case 'json':
            return 'application/json';
        case 'xml':
            return 'application/xml';
        case 'text':
            return 'text/plain';
        case 'form':
            return 'application/x-www-form-urlencoded';
        case 'multipart':
            return 'multipart/form-data';
        default:
            return 'application/json';
    }
}

// Store current WebSocket request details for history
let currentWsRequestDetails = null;

// FIX: Added cleanup for existing connections and auth/headers support
function connectWebSocket() {
    const url = document.getElementById('request-url').value.trim();

    if (!url) {
        showNotification('Please enter a WebSocket URL', 'error');
        return;
    }

    // Validate URL
    if (!url.startsWith('ws://') && !url.startsWith('wss://')) {
        showNotification('WebSocket URL must start with ws:// or wss://', 'error');
        return;
    }

    // FIX: Close existing connection if any
    if (wsConnection && wsConnection.readyState === WebSocket.OPEN) {
        wsConnection.close();
    }

    // Get headers
    const headers = getHeaders();

    // Get auth details
    const authTypeSelect = document.getElementById('auth-type-select');
    const authType = authTypeSelect ? authTypeSelect.value : 'none';
    let authToken = null;
    let authUsername = null;
    let authPassword = null;

    if (authType === 'bearer') {
        const tokenInput = document.getElementById('auth-bearer-token');
        authToken = tokenInput ? tokenInput.value : null;
    } else if (authType === 'basic') {
        const usernameInput = document.getElementById('auth-basic-username');
        const passwordInput = document.getElementById('auth-basic-password');
        authUsername = usernameInput ? usernameInput.value : null;
        authPassword = passwordInput ? passwordInput.value : null;
    }

    // Store request details for history
    currentWsRequestDetails = {
        url: url,
        method: 'WS',
        body: '',
        headers: headers,
        bodyType: 'none',
        authType: authType,
        authToken: authToken || '',
        authUsername: authUsername || '',
        authPassword: authPassword || '',
        requestType: 'ws'
    };

    updateWsStatus('connecting', 'Connecting...');

    // Connect to our backend WebSocket proxy
    const wsUrl = `ws://${window.location.host}/api/ws`;
    wsConnection = new WebSocket(wsUrl);

    wsConnection.onopen = () => {
        // Send connect message to proxy with headers and auth
        const connectMessage = {
            type: 'connect',
            url: url,
            headers: Object.keys(headers).length > 0 ? headers : null,
            auth_type: authType !== 'none' ? authType : null,
            auth_token: authToken,
            auth_username: authUsername,
            auth_password: authPassword
        };
        wsConnection.send(JSON.stringify(connectMessage));
    };

    wsConnection.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data);
            handleWsServerMessage(msg);
        } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
        }
    };

    wsConnection.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateWsStatus('disconnected', 'Connection error');
        wsConnected = false;
        updateWsButtonVisibility();
        addWsMessage('error', 'Connection error');
    };

    wsConnection.onclose = () => {
        updateWsStatus('disconnected', 'Disconnected');
        wsConnected = false;
        updateWsButtonVisibility();
    };
}

function handleWsServerMessage(msg) {
    switch (msg.type) {
        case 'connected':
            wsConnected = true;
            updateWsStatus('connected', `Connected to ${msg.url}`);
            updateWsButtonVisibility();
            addWsMessage('info', `Connected to ${msg.url}`);
            // Add to execution history when successfully connected
            if (currentWsRequestDetails) {
                addToExecutionHistory(currentWsRequestDetails);
                // Save the WebSocket request if it has an ID
                if (currentRequestId) {
                    saveRequest();
                }
            }
            break;
        case 'disconnected':
            wsConnected = false;
            updateWsStatus('disconnected', msg.reason || 'Disconnected');
            updateWsButtonVisibility();
            addWsMessage('info', msg.reason || 'Disconnected');
            break;
        case 'message':
            addWsMessage(msg.direction, msg.data);
            break;
        case 'error':
            addWsMessage('error', msg.message);
            break;
        case 'info':
            addWsMessage('info', msg.message);
            break;
    }
}

function disconnectWebSocket() {
    if (wsConnection) {
        wsConnection.send(JSON.stringify({ type: 'disconnect' }));
        wsConnection.close();
        wsConnection = null;
    }
    wsConnected = false;
    updateWsStatus('disconnected', 'Disconnected');
    updateWsButtonVisibility();
}

function sendWebSocketMessage() {
    const messageInput = document.getElementById('ws-message-input');
    const message = messageInput.value.trim();

    if (!message) {
        showNotification('Please enter a message to send', 'error');
        return;
    }

    if (!wsConnection || wsConnection.readyState !== WebSocket.OPEN) {
        showNotification('WebSocket is not connected', 'error');
        return;
    }

    wsConnection.send(JSON.stringify({
        type: 'send',
        message: message
    }));

    messageInput.value = '';
}

function addWsMessage(type, content) {
    const messagesContainer = document.getElementById('ws-messages');
    const messageDiv = document.createElement('div');
    messageDiv.className = `ws-message ${type}`;

    const timestamp = new Date().toLocaleTimeString();
    let directionClass = '';
    let directionText = '';

    if (type === 'sent') {
        directionClass = 'sent';
        directionText = 'SENT';
    } else if (type === 'received') {
        directionClass = 'received';
        directionText = 'RECEIVED';
    } else if (type === 'error') {
        directionText = 'ERROR';
    } else if (type === 'info') {
        directionText = 'INFO';
    }

    messageDiv.innerHTML = `
        <div class="ws-message-header">
            <span class="ws-message-direction ${directionClass}">${directionText}</span>
            <span class="ws-message-time">${timestamp}</span>
        </div>
        <div class="ws-message-content">${escapeHtml(content)}</div>
    `;

    messagesContainer.appendChild(messageDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

function clearWebSocketMessages() {
    const messagesContainer = document.getElementById('ws-messages');
    messagesContainer.innerHTML = '';
}

// Collapsible Sections
function setupCollapsibleSections() {
    // Collection section toggle
    const collectionHeader = document.getElementById('collection-header');
    const collectionSection = document.getElementById('collection-section');

    if (collectionHeader && collectionSection) {
        collectionHeader.addEventListener('click', (e) => {
            // Don't toggle if clicking on action buttons
            if (e.target.closest('.sidebar-section-actions')) return;
            collectionSection.classList.toggle('collapsed');
        });
    }

    // History section toggle
    const historyHeader = document.getElementById('history-header');
    const historySection = document.getElementById('history-section');

    if (historyHeader && historySection) {
        historyHeader.addEventListener('click', () => {
            historySection.classList.toggle('collapsed');
        });
    }
}

// Build History Tree - shows requests sorted by updated_at desc
function buildHistoryTree() {
    const historyTree = document.getElementById('history-tree');
    if (!historyTree) return;

    historyTree.innerHTML = '';

    // Sort requests by updated_at descending (most recent first)
    const sortedRequests = [...requests]
        .filter(r => !r.archived_at)
        .sort((a, b) => {
            const dateA = new Date(a.updated_at || a.created_at);
            const dateB = new Date(b.updated_at || b.created_at);
            return dateB - dateA;
        });

    if (sortedRequests.length === 0) {
        const emptyItem = document.createElement('li');
        emptyItem.className = 'history-item';
        emptyItem.style.color = 'rgba(255, 255, 255, 0.5)';
        emptyItem.style.fontStyle = 'italic';
        emptyItem.textContent = 'No requests yet';
        historyTree.appendChild(emptyItem);
        return;
    }

    sortedRequests.forEach(request => {
        const item = document.createElement('li');
        item.className = 'history-item';

        const methodClass = request.method || 'GET';
        const timeAgo = getTimeAgo(new Date(request.updated_at || request.created_at));

        item.innerHTML = `
            <span class="history-item-method ${methodClass}">${methodClass}</span>
            <span class="history-item-name">${escapeHtml(request.name)}</span>
            <span class="history-item-time">${timeAgo}</span>
        `;

        item.addEventListener('click', () => {
            selectRequest(request.id);
        });

        item.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            e.stopPropagation();
            const isArchived = request.archived_at;
            showContextMenu(e, [
                {
                    label: 'Edit',
                    icon: 'fas fa-edit',
                    action: () => editRequest(request.id)
                },
                /*
                {
                    label: isArchived ? 'Unarchive' : 'Archive',
                    icon: isArchived ? 'fas fa-archive' : 'fas fa-archive',
                    action: () => isArchived ? unarchiveRequest(request.id) : archiveRequest(request.id)
                },
                */
                {
                    label: 'Delete',
                    icon: 'fas fa-trash',
                    action: () => deleteRequest(request.id),
                    danger: true
                }
            ]);
        });

        historyTree.appendChild(item);
    });
}

// Get relative time string
function getTimeAgo(date) {
    const now = new Date();
    const diffMs = now - date;
    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffSecs / 60);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffSecs < 60) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString();
}

// Import Handler
function setupImportHandler() {
    const importBtn = document.getElementById('import-btn');
    const importFileInput = document.getElementById('import-file-input');

    if (importBtn && importFileInput) {
        importBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            openModal('import-modal');
        });

        importFileInput.addEventListener('change', (e) => {
            if (e.target.files.length > 0) {
                handleImport(e.target.files[0]);
                // Reset value so same file can be imported again if needed
                importFileInput.value = '';
            }
        });
    }
}

async function handleImport(file) {
    const formData = new FormData();
    formData.append('file', file);

    // Show loading...
    showNotification(`Analyzing ${file.name}...`, 'info');

    try {
        // Request preview
        const response = await fetch('/api/import?preview=true', {
            method: 'POST',
            body: formData
        });

        if (response.ok) {
            const result = await response.json();
            
            if (result.collections && result.collections.length > 0) {
                // Store file for confirmation
                pendingImportFile = file;
                
                // Populate preview list
                const listContainer = document.getElementById('import-preview-list');
                listContainer.innerHTML = '';
                
                result.collections.forEach(col => {
                    const item = document.createElement('div');
                    item.style.cssText = 'display: flex; justify-content: space-between; align-items: center; padding: 10px; border-bottom: 1px solid var(--border-color);';
                    item.innerHTML = `
                        <div style="font-weight: 500; color: var(--text-primary);">
                            <i class="fas fa-folder" style="color: var(--primary-blue); margin-right: 8px;"></i>
                            ${escapeHtml(col.name)}
                        </div>
                        <div style="font-size: 12px; color: var(--text-secondary); background: var(--bg-secondary); padding: 2px 8px; border-radius: 10px;">
                            ${col.request_count} requests
                        </div>
                    `;
                    listContainer.appendChild(item);
                });
                
                // Open confirmation modal
                closeModal('import-modal'); // Ensure previous modal is closed
                openModal('import-confirmation-modal');
            } else {
                showNotification('No valid collections/requests found in file', 'error');
            }
        } else {
            const error = await response.text();
            showNotification(`Import analysis failed: ${error}`, 'error');
        }
    } catch (error) {
        console.error('Error during import analysis:', error);
        showNotification(`Error during import analysis: ${error.message}`, 'error');
    }
}

async function confirmImport() {
    if (!pendingImportFile) return;
    
    const formData = new FormData();
    formData.append('file', pendingImportFile);
    
    // Close modal immediately and show loading
    closeModal('import-confirmation-modal');
    showNotification('Importing collections...', 'info');
    
    try {
        const response = await fetch('/api/import', {
            method: 'POST',
            body: formData
        });

        if (response.ok) {
            const result = await response.json();
            showNotification(result.message || 'Import successful');
            // Refresh UI
            await loadFolders();
            await loadRequests(null, showArchived);
            pendingImportFile = null;
        } else {
            const error = await response.text();
            showNotification(`Import failed: ${error}`, 'error');
        }
    } catch (error) {
        console.error('Error during import:', error);
        showNotification(`Error during import: ${error.message}`, 'error');
    }
}
