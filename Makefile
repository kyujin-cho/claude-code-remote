.PHONY: all clean test lint build-scie build-scie-lazy install-dev help

PYTHON := .venv/bin/python
PEX := uv run pex
DIST_DIR := dist

help:
	@echo "Available targets:"
	@echo "  install-dev    Install development dependencies"
	@echo "  test           Run tests"
	@echo "  lint           Run linting"
	@echo "  build-scie     Build self-executable binary (eager mode, ~50MB)"
	@echo "  build-scie-lazy Build self-executable binary (lazy mode, ~5MB)"
	@echo "  clean          Remove build artifacts"

install-dev:
	uv sync --all-extras
	uv pip install pex

test:
	$(PYTHON) -m pytest tests/ -v

lint:
	$(PYTHON) -m ruff check claude_code_telegram/ tests/
	$(PYTHON) -m mypy claude_code_telegram/

format:
	$(PYTHON) -m ruff format claude_code_telegram/ tests/

# Build self-executable with bundled Python (~50MB, works offline)
build-scie: $(DIST_DIR)
	@echo "Building scie (eager mode - bundled Python)..."
	$(PEX) . \
		--scie eager \
		--scie-python-version 3.11 \
		-c claude-code-telegram-hook \
		-o $(DIST_DIR)/claude-code-telegram-hook
	@echo "Built: $(DIST_DIR)/claude-code-telegram-hook"
	@du -h $(DIST_DIR)/claude-code-telegram-hook

# Build self-executable that fetches Python on first run (~5MB)
build-scie-lazy: $(DIST_DIR)
	@echo "Building scie (lazy mode - fetch Python on first run)..."
	$(PEX) . \
		--scie lazy \
		--scie-python-version 3.11 \
		-c claude-code-telegram-hook \
		-o $(DIST_DIR)/claude-code-telegram-hook
	@echo "Built: $(DIST_DIR)/claude-code-telegram-hook"
	@du -h $(DIST_DIR)/claude-code-telegram-hook

$(DIST_DIR):
	mkdir -p $(DIST_DIR)

clean:
	rm -rf $(DIST_DIR)
	rm -rf .pex-build
	rm -rf *.egg-info
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name .pytest_cache -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name .mypy_cache -exec rm -rf {} + 2>/dev/null || true
