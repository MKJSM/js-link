# Installation Guide

You can install `js-link` using Cargo, Rust's package manager.

## Prerequisites

Before installing, ensure you have the following:
- [Rust and Cargo](https://rustup.rs/) installed.
- SQLite installed on your system.

## Installing from Source

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/yourusername/js-link.git
    cd js-link
    ```

2.  **Install the binary**:
    ```bash
    cargo install --path .
    ```

This will compile the binary and place it in your `~/.cargo/bin` directory.

## Running the Application

Since `js-link` embeds all necessary assets and handles database setup automatically:

1.  Run the application from anywhere:
    ```bash
    js-link
    ```
2.  Open [http://localhost:3000](http://localhost:3000) in your browser.

The application will automatically create a `jslink.db` file in the current directory if it doesn't exist. You can override the database location by setting the `DATABASE_URL` environment variable:

```bash
export DATABASE_URL="sqlite:/path/to/your/db.sqlite"
js-link
```

## Installing via Crates.io (Future)

Once the package is published to [crates.io](https://crates.io), you will be able to install it directly:

```bash
cargo install js-link
```