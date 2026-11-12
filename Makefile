BIN          = $(HOME)/.local/bin/helperpad
LOG          = $(or $(XDG_STATE_HOME),$(HOME)/.local/state)/helperpad/helperpad.log

.DEFAULT_GOAL := help

.PHONY: help
help:
	@echo "helperpad: Topaz signature pad helper for Chrome on Linux"
	@echo ""
	@echo "  make install          set up everything: tools, pad access, helperpad, extension"
	@echo "  make update           download and install the newest release"
	@echo "  make uninstall        remove helperpad and its browser registration"
	@echo "  make status           show what's installed and whether the pad is found"
	@echo "  make logs             watch the log live (Ctrl+C to stop)"
	@echo "  make window           open the signing window to test the pad"
	@echo "  make check-sigstring  check a SigString with Topaz's own library (FILE=path, MODE=0)"
	@echo "  make test             run the tests"
	@echo "  make lint             run clippy"
	@echo "  make build            build without installing"
	@echo "  make add-version      bump, commit, and tag a version (VERSION=1.2.3)"
	@echo "  make dry-release      build every release package into dist/ without publishing"
	@echo "  make release          publish the tagged version to GitHub (needs GITHUB_TOKEN)"

.PHONY: build
build:
	@cargo build --release

.PHONY: install
install:
	@./tools/install-from-source.sh

.PHONY: update
update:
	@test -x $(BIN) || { echo "Run make install first."; exit 1; }
	@$(BIN) update

.PHONY: uninstall
uninstall:
	@test -x $(BIN) || { echo "helperpad isn't installed."; exit 1; }
	@$(BIN) uninstall --yes

.PHONY: status
status:
	@test -x $(BIN) || { echo "Not installed. Run make install first."; exit 1; }
	@$(BIN) status

.PHONY: logs
logs:
	@mkdir -p $(dir $(LOG)) && touch $(LOG) && tail -f $(LOG)

.PHONY: window
window:
	@test -x $(BIN) || { echo "Run make install first."; exit 1; }
	@$(BIN) --sign

.PHONY: check-sigstring
check-sigstring:
	@./tools/check-sigstring.sh $(or $(FILE),-) $(or $(MODE),0) $(OUT)

.PHONY: test
test:
	@cargo test

.PHONY: lint
lint:
	@cargo clippy

.PHONY: add-version
add-version:
	@./tools/add-version.sh $(VERSION)

.PHONY: dry-release
dry-release:
	@goreleaser release --snapshot --clean

.PHONY: release
release:
	@goreleaser release --clean
