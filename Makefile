# GA4GH Infra — local Docker lifecycle (no `just` required).
#
# Full stack: broker, ADS, admin-ui, mock IdP, registries, Postgres, etc.

export DOCKER_BUILDKIT := 1
export COMPOSE_DOCKER_CLI_BUILD := 1

COMPOSE := docker compose -f docker/docker-compose.yml --env-file docker/.env.example
COMPOSE_SQLITE := docker compose -f docker/docker-compose.sqlite.yml --env-file docker/.env.example
SECRETS_DIR := docker/secrets

# Catch "make up local" / "make up sqlite" (space instead of hyphen) before any work runs.
ifeq ($(filter up local,$(MAKECMDGOALS)),up local)
$(error Typo: use "make up-local" (with a hyphen), not "make up local")
endif
ifeq ($(filter up sqlite,$(MAKECMDGOALS)),up sqlite)
$(error Typo: use "make up-sqlite" (with a hyphen), not "make up sqlite")
endif

.PHONY: help up up-local up-sqlite down destroy logs test seed seed-sqlite prepare-secrets prepare-vendor prepare-admin-ui-static print-urls print-urls-sqlite local uplocal up_local upsqlite up_sqlite

help:
	@echo "ga4gh-infra — local Docker stack"
	@echo ""
	@echo "  make up-local   Build and start full stack (recommended for testing admin-ui)"
	@echo "  make up         Alias for up-local"
	@echo "  make up-sqlite  Lighter stack (SQLite; admin-ui on :8195)"
	@echo "  make down       Stop containers, keep volumes/data"
	@echo "  make destroy    Stop containers and remove volumes (fresh start)"
	@echo ""
	@echo "  make prepare-vendor  Refresh docker/vendor for offline image builds"
	@echo "  make logs       Follow compose logs (make logs ARGS='broker -f')"
	@echo "  make seed       Load demo data into running postgres stack"
	@echo "  make seed-sqlite  Load demo data into running sqlite stack"
	@echo "  make test       cargo test --workspace"
	@echo ""
	@echo "Requires: Docker + Docker Compose"
	@echo ""
	@echo "Note: targets use hyphens — make up-local, not \"make up local\"."

# Friendly hints for common typos (wrong separator or target name).
local:
	@echo "Unknown target 'local'." >&2
	@echo "Did you mean:  make up-local   (full stack)" >&2
	@exit 1

uplocal up_local:
	@echo "Unknown target '$@'." >&2
	@echo "Did you mean:  make up-local" >&2
	@exit 1

upsqlite up_sqlite:
	@echo "Unknown target '$@'." >&2
	@echo "Did you mean:  make up-sqlite" >&2
	@exit 1

# Generate dev RSA keys when missing (broker, registry, mock IdP).
prepare-secrets:
	@mkdir -p $(SECRETS_DIR)
	@for name in broker_rs256.pem registry_rs256.pem mock_idp_rs256.pem; do \
		path="$(SECRETS_DIR)/$$name"; \
		if [ ! -f "$$path" ]; then \
			echo "Generating $$name …"; \
			if command -v ga4gh-infra >/dev/null 2>&1; then \
				ga4gh-infra keygen --output "$$path"; \
			else \
				cargo run -q -p ga4gh-infra-cli -- keygen --output "$$path"; \
			fi; \
		fi; \
	done

# Vendor crates on the host so Docker builds do not hit crates.io (avoids SSL flakes).
prepare-vendor:
	@chmod +x scripts/prepare-docker-vendor.sh
	@./scripts/prepare-docker-vendor.sh

prepare-admin-ui-static:
	@chmod +x scripts/prepare-admin-ui-static.sh
	@./scripts/prepare-admin-ui-static.sh

# Primary target: full local test stack including admin-ui + broker login.
up-local: prepare-secrets prepare-vendor prepare-admin-ui-static
	$(COMPOSE) up --build --wait
	@$(MAKE) --no-print-directory seed
	@$(MAKE) --no-print-directory print-urls

up: up-local

up-sqlite: prepare-secrets prepare-admin-ui-static
	$(COMPOSE_SQLITE) up --build --wait
	@$(MAKE) --no-print-directory seed-sqlite
	@$(MAKE) --no-print-directory print-urls-sqlite

seed:
	@chmod +x scripts/seed-dev-stack.sh
	@GA4GH_SEED_PROFILE=postgres ./scripts/seed-dev-stack.sh postgres

seed-sqlite:
	@chmod +x scripts/seed-dev-stack.sh
	@GA4GH_SEED_PROFILE=sqlite ./scripts/seed-dev-stack.sh sqlite

print-urls:
	@echo ""
	@echo "Stack is ready. Open in your browser:"
	@echo "  Admin UI (login):     http://localhost:8095"
	@echo "  AAI broker:           http://localhost:8080"
	@echo "  Access Decision Svc:  http://localhost:8090"
	@echo "  Agreement registry:   http://localhost:8086"
	@echo "  Mock IdP:             http://localhost:9000"
	@echo ""
	@echo "  make logs     — tail logs"
	@echo "  make down     — stop (keep data)"
	@echo "  make seed     — reload demo data (postgres stack)"
	@echo "  make destroy  — stop and wipe volumes"
	@echo ""

print-urls-sqlite:
	@echo ""
	@echo "SQLite stack is ready:"
	@echo "  Admin UI (login):     http://localhost:8195"
	@echo "  AAI broker:           http://localhost:8080"
	@echo ""

down:
	-$(COMPOSE) down
	-$(COMPOSE_SQLITE) down

destroy:
	-$(COMPOSE) down -v --remove-orphans
	-$(COMPOSE_SQLITE) down -v --remove-orphans

logs:
	$(COMPOSE) logs -f $(ARGS)

test:
	cargo test --workspace
