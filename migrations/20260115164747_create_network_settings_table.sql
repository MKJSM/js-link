-- Add migration script here
CREATE TABLE network_settings (
    id INTEGER PRIMARY KEY DEFAULT 1,
    auto_proxy BOOLEAN NOT NULL DEFAULT TRUE,
    http_proxy TEXT,
    https_proxy TEXT,
    no_proxy TEXT
);

INSERT INTO network_settings (id, auto_proxy, http_proxy, https_proxy, no_proxy) VALUES (1, TRUE, NULL, NULL, NULL);