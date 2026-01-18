-- Add body type and authentication fields to requests table
-- Body type: 'none', 'json', 'xml', 'text', 'form', 'multipart', 'binary'
-- Auth type: 'none', 'bearer', 'basic'

ALTER TABLE requests ADD COLUMN body_type TEXT NOT NULL DEFAULT 'none';
ALTER TABLE requests ADD COLUMN body_content TEXT;

ALTER TABLE requests ADD COLUMN auth_type TEXT NOT NULL DEFAULT 'none';
ALTER TABLE requests ADD COLUMN auth_token TEXT;
ALTER TABLE requests ADD COLUMN auth_username TEXT;
ALTER TABLE requests ADD COLUMN auth_password TEXT;
