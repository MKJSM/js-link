# JS-Link

**A full-featured API client for testing HTTP and WebSocket APIs.**

## Documentation

Please refer to [install.md](install.md) for a comprehensive guide on:
- Installation & Database Setup
- Interface Overview
- Step-by-Step Usage Instructions

## Quick Start

1.  **Install Prerequisites**:
    ```bash
    cargo install sqlx-cli --no-default-features --features native-tls,sqlite
    ```

2.  **Setup Database**:
    ```bash
    export DATABASE_URL="sqlite:jslink.db"
    sqlx database create
    sqlx migrate run
    ```

3.  **Run**:
    ```bash
    cargo run
    ```

4.  **Open**: [http://localhost:3000](http://localhost:3000)
