# GA4GH Infra — Synaptic Four unified local lifecycle
# Wraps justfile targets; requires: https://github.com/casey/just (or use docker compose directly).

.PHONY: help up down destroy logs test

help:
	@echo "ga4gh-infra — local lifecycle (Synaptic Four GA4GH stack)"
	@echo ""
	@echo "  make up        Start Docker stack (PostgreSQL)"
	@echo "  make down      Stop stack; keep volumes"
	@echo "  make destroy   Stop stack; remove volumes"
	@echo ""
	@echo "  make logs      Tail compose logs"
	@echo "  make test      Run workspace unit tests (cargo test)"
	@echo ""
	@echo "Also: just up-sqlite (lighter SQLite stack), just prepare-secrets"

up:
	just up

down:
	just down

destroy:
	just destroy

logs:
	just logs

test:
	just test
