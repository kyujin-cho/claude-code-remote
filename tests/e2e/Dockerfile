# E2E Test Container for claude-code-telegram
# Uses Python for test runner + telethon userbot, builds Rust binary

FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# Copy Rust project files first (for caching)
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the Rust binary
RUN cargo build --release

# Copy Python test files
COPY tests/e2e/requirements.txt ./tests/e2e/
RUN pip install --no-cache-dir -r tests/e2e/requirements.txt

COPY tests/e2e ./tests/e2e

# Create .claude directory for config
RUN mkdir -p /root/.claude

# Set binary path
ENV CLAUDE_CODE_TELEGRAM_BIN=/app/target/release/claude-code-telegram
ENV PYTHONUNBUFFERED=1

# Default command runs pytest
WORKDIR /app/tests/e2e
CMD ["pytest", "-v", "--tb=short"]
