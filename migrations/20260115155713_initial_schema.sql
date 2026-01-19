-- Initial Schema

CREATE TABLE folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMP
);

CREATE TABLE requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    body TEXT,
    headers TEXT, -- Stored as JSON
    folder_id INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMP,
    
    -- Request Type
    request_type TEXT NOT NULL DEFAULT 'api',
    
    -- Body & Auth
    body_type TEXT NOT NULL DEFAULT 'none',
    body_content TEXT,
    auth_type TEXT NOT NULL DEFAULT 'none',
    auth_token TEXT,
    auth_username TEXT,
    auth_password TEXT,

    FOREIGN KEY (folder_id) REFERENCES folders (id) ON DELETE CASCADE
);

CREATE TABLE environments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    variables TEXT NOT NULL, -- Stored as JSON
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived_at TIMESTAMP
);

CREATE TABLE network_settings (
    id INTEGER PRIMARY KEY DEFAULT 1,
    auto_proxy BOOLEAN NOT NULL DEFAULT TRUE,
    http_proxy TEXT,
    https_proxy TEXT,
    no_proxy TEXT
);

INSERT INTO network_settings (id, auto_proxy, http_proxy, https_proxy, no_proxy) VALUES (1, TRUE, NULL, NULL, NULL);
