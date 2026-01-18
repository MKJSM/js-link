-- Add request_type column to requests table
-- 'api' for HTTP/API requests, 'ws' for WebSocket requests
ALTER TABLE requests ADD COLUMN request_type TEXT NOT NULL DEFAULT 'api';
