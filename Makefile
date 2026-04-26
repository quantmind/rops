.PHONY: help
help:				## Show this help message
	@echo ======================================================================================
	@echo rOps
	@echo ======================================================================================
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sed -e 's/\\$$//' | sed -e 's/##//'
	@echo ======================================================================================

.PHONY: lint
lint:				## Run linters and fix issues
	@./dev/lint-rs fix

.PHONY: lint-check
lint-check:			## Run linters
	@./dev/lint-rs

.PHONY: tag
tag:				## Tag current version (from Cargo.toml) and push
	$(eval VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'))
	@read -p "Tagging with v$(VERSION), are you sure? [Y/n] " ans; \
	ans=$${ans:-Y}; \
	if [ "$$ans" = "Y" ] || [ "$$ans" = "y" ]; then \
		git tag v$(VERSION) && git push origin v$(VERSION); \
	else \
		echo "Aborted."; \
	fi
