.PHONY: help build test fmt

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-10s %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

test: ## Run tests
	cargo test

fmt: ## Format code
	cargo fmt

lint: ## Lint code
	cargo clippy
