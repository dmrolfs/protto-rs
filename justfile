# Enhanced Justfile for Rust Development with Modern CLI Tools

# Show all available commands
default:
    @just --list

# === RUST DEVELOPMENT ===

# Build the project
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with nextest (faster)
test-fast:
    cargo nextest run

# Run clippy linting
clippy:
    cargo clippy -- -D warnings -W clippy::pedantic

# Format code
fmt:
    cargo fmt

# Check code without building
check:
    cargo check

# Generate and open documentation
doc:
    cargo doc --open

# Run coverage analysis
coverage:
    cargo tarpaulin --timeout 300 --out html --output-dir coverage/

# Security audit
audit:
    cargo audit

# Check for outdated dependencies
outdated:
    cargo outdated

# Find unused dependencies
unused-deps:
    cargo machete

# Expand macros
expand:
    cargo expand

# Clean build artifacts
clean:
    cargo clean

# Full development workflow
dev: fmt clippy test

# Watch for changes and run tests
watch:
    cargo watch -c -x test

# Watch for changes and run specific command
watch-cmd cmd:
    cargo watch -c -x "{{cmd}}"

# Run the application
run *args:
    cargo run {{args}}

# Benchmark with hyperfine
benchmark cmd:
    hyperfine "{{cmd}}"

# === CONTAINER SERVICES ===

# Start all lakehouse services
start:
    @echo "🚀 Starting Container System..."
    podman-compose up -d

# Stop all services
stop:
    @echo "🛑 Stopping Container System..."
    podman-compose down

# Show service status with health checks
status:
    @echo "📊 Service Status:"
    podman-compose ps
    @echo ""
    @echo "🔍 Health Checks:"
#    @echo -n "MinIO: "
#    @curl -s -f http://localhost:9000/minio/health/live && echo "✅ Healthy" || echo "❌ Unhealthy"
#    @echo -n "Nessie: "
#    @curl -s -f http://localhost:19120/api/v1/config > /dev/null && echo "✅ Healthy" || echo "❌ Unhealthy"
#    @echo -n "Trino: "
#    @curl -s -f http://localhost:8080/v1/status > /dev/null && echo "✅ Healthy" || echo "❌ Unhealthy"
#    @echo -n "API: "
#    @curl -s -f http://localhost:3030/query -X POST -H "Content-Type: application/json" -d '{"sql":"SELECT 1"}' > /dev/null && echo "✅ Healthy" || echo "❌ Unhealthy"

# View service logs
logs container="":
    @if [ -n "{{container}}" ]; then \
        podman-compose logs -f {{container}}; \
    else \
        podman-compose logs -f; \
    fi
