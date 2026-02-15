.PHONY: help
help:
	@echo ======================================================================================
	@echo rOps
	@echo ======================================================================================
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sed -e 's/\\$$//' | sed -e 's/##//'
	@echo ======================================================================================


.PHONY: lint
lint:				## Run linters and fix issues
	@./dev/lint-rs fix


.PHONY: lint
lint-check:			## Run linters
	@./dev/lint-rs
