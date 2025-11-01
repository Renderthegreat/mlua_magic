export RUST_BACKTRACE=1

export RUSTFLAGS="-Zmacro-backtrace"

cargo +nightly test example

# cargo expand --test main