.PHONY: help migrate-up migrate-down migrate-status migrate-create migrate-force migrate-drop db-reset

# Load environment variables
include .env
export

# Database connection
DB_URL := $(DATABASE_URL)

help: ## Show this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'

migrate-up: ## Run all pending migrations
	@echo "Running migrations..."
	@sqlx migrate run --database-url "$(DB_URL)"
	@echo "✓ Migrations completed"

migrate-down: ## Rollback the last migration
	@echo "Rolling back last migration..."
	@sqlx migrate revert --database-url "$(DB_URL)"
	@echo "✓ Migration rolled back"

migrate-status: ## Show migration status
	@echo "Migration status:"
	@sqlx migrate info --database-url "$(DB_URL)"

migrate-create: ## Create a new migration (use: make migrate-create NAME=migration_name)
	@if [ -z "$(NAME)" ]; then \
		echo "Error: NAME is required"; \
		echo "Usage: make migrate-create NAME=your_migration_name"; \
		exit 1; \
	fi
	@sqlx migrate add $(NAME)
	@echo "✓ Created migration files for: $(NAME)"

migrate-force: ## Force set migration version (use: make migrate-force VERSION=20250131000001)
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: VERSION is required"; \
		echo "Usage: make migrate-force VERSION=20250131000001"; \
		exit 1; \
	fi
	@sqlx migrate force $(VERSION) --database-url "$(DB_URL)"
	@echo "✓ Forced migration version to: $(VERSION)"

migrate-drop: ## Drop all tables and re-run migrations (DANGEROUS!)
	@echo "WARNING: This will drop all tables and data!"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		sqlx database drop --database-url "$(DB_URL)" -y; \
		sqlx database create --database-url "$(DB_URL)"; \
		sqlx migrate run --database-url "$(DB_URL)"; \
		echo "✓ Database reset complete"; \
	else \
		echo "Cancelled"; \
	fi

db-reset: migrate-drop ## Alias for migrate-drop

# Alternative: Use psql directly for migrations (if sqlx not installed)
migrate-up-psql: ## Run migrations using psql
	@echo "Running migrations with psql..."
	@for file in migrations/*.up.sql; do \
		echo "Applying $$file..."; \
		psql "$(DB_URL)" -f "$$file"; \
	done
	@echo "✓ Migrations completed"

migrate-down-psql: ## Rollback last migration using psql
	@echo "Rolling back with psql..."
	@file=$$(ls -t migrations/*.down.sql | head -1); \
	if [ -n "$$file" ]; then \
		echo "Applying $$file..."; \
		psql "$(DB_URL)" -f "$$file"; \
		echo "✓ Migration rolled back"; \
	else \
		echo "No migration files found"; \
	fi

# Build and run targets
build: ## Build the project
	cargo build --release

run: ## Run the server
	cargo run --release

dev: ## Run in development mode with auto-reload
	cargo watch -x run

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean

# Database utilities
db-connect: ## Connect to database with psql
	psql "$(DB_URL)"

db-backup: ## Backup database to file
	@timestamp=$$(date +%Y%m%d-%H%M%S); \
	filename="backup-$$timestamp.sql"; \
	pg_dump "$(DB_URL)" > "$$filename"; \
	echo "✓ Database backed up to: $$filename"

db-restore: ## Restore database from backup (use: make db-restore FILE=backup.sql)
	@if [ -z "$(FILE)" ]; then \
		echo "Error: FILE is required"; \
		echo "Usage: make db-restore FILE=backup.sql"; \
		exit 1; \
	fi
	@psql "$(DB_URL)" -f "$(FILE)"
	@echo "✓ Database restored from: $(FILE)"
