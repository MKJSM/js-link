# Just command runner

# This command runs the application in development mode
run:
    cargo run

# This command runs the tests
test:
    cargo test

# This command checks the code for errors
check:
    cargo check

watch:
    cargo watch -w src -w static -w templates -x run
