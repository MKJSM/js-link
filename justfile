# Just command runner

# Install frontend dependencies
npm-install:
    cd frontend && npm install

# Build frontend
npm-build:
    cd frontend && npm run build

# Copy frontend assets to backend (includes build)
npm-copy:
    cd frontend && npm run copy

# Install, build and copy frontend
npm-setup: npm-install npm-copy

# Build rust backend
rust-build:
    cargo build

# Run rust backend
rust-run:
    cargo run

# Watch mode for development
watch:
    cargo watch -w src -w static -w templates -x run

# Build everything and run
all: npm-setup rust-run