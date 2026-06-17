# m5Tui task runner. Requires `just` (https://github.com/casey/just).
# Run `just` to see the available recipes.

# Default recipe: list available recipes.
default:
    @just --list

# Run the host-side tests, fmt, and clippy.
host-gates:
    cargo test --workspace
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Run the simulator bin and dump the cockpit PPM to /tmp/m5tui-sim.
sim:
    cargo run --bin m5tui-bin

# Build the device firmware (xtensa-esp32s3-espidf). Requires espup.
build-esp profile="release":
    ./scripts/build-esp.sh --{{profile}}

# Build and flash the device. PORT is e.g. /dev/ttyACM0.
flash-esp profile PORT:
    ./scripts/build-esp.sh --{{profile}} --flash {{PORT}}

# Build, flash, and tail the serial monitor.
flash-and-monitor profile PORT:
    ./scripts/build-esp.sh --{{profile}} --flash {{PORT}} --monitor {{PORT}}

# Push the repo to GitHub (used by m5Tui's release flow).
push tag:
    git push origin master
    git push origin {{tag}}

# Tag and push a new release.
release tag:
    git tag {{tag}}
    git push origin {{tag}}
