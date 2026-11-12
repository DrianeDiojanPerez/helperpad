BIN          = $(HOME)/.local/bin/helperpad
LOG          = $(or $(XDG_STATE_HOME),$(HOME)/.local/state)/helperpad/helperpad.log
HOST_NAME    = com.topaz.sigplusextlite.win.2
EXTENSION_ID = dhcpobccjkdnmibckgpejmbpmpembgco
STORE_URL    = https://chromewebstore.google.com/detail/topaz-sigplusextlite-exte/$(EXTENSION_ID)
BROWSERS     = google-chrome chromium
UDEV_RULE    = /etc/udev/rules.d/70-topaz-sigpad.rules

.DEFAULT_GOAL := help

.PHONY: help
help:
	@echo "helperpad: Topaz signature pad helper for Chrome on Linux"
	@echo ""
	@echo "  make install          set up everything: tools, pad access, helperpad, extension"
	@echo "  make update           download and install the newest release"
	@echo "  make uninstall        remove the binary and the Chrome registration"
	@echo "  make status           show what's installed and whether the pad is found"
	@echo "  make logs             show the last 50 log lines"
	@echo "  make logs-follow      watch the log live (Ctrl+C to stop)"
	@echo "  make logs-clear       empty the log"
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
	@./install.sh

.PHONY: update
update:
	@test -x $(BIN) || { echo "Run make install first."; exit 1; }
	@$(BIN) update

.PHONY: uninstall
uninstall:
	@rm -fv $(BIN)
	@for browser in $(BROWSERS); do rm -fv "$(HOME)/.config/$$browser/NativeMessagingHosts/$(HOST_NAME).json"; done
	@echo "Logs kept in $(LOG). Run 'make logs-clear' or delete it to remove them."

.PHONY: status
status:
	@printf "binary:     "; test -x $(BIN) && echo "$(BIN)" || echo "not installed (run make install)"
	@for browser in $(BROWSERS); do \
		f="$(HOME)/.config/$$browser/NativeMessagingHosts/$(HOST_NAME).json"; \
		[ -f "$$f" ] && echo "chrome:     registered ($$f)"; \
	done; true
	@printf "extension:  "; ls -d $(HOME)/.config/google-chrome/*/Extensions/$(EXTENSION_ID) >/dev/null 2>&1 \
		&& echo "installed" || echo "not found in Chrome ($(STORE_URL))"
	@printf "pad:        "; dev=$$(grep -l 000006A8 /sys/class/hidraw/*/device/uevent 2>/dev/null | head -1 | cut -d/ -f5); \
		if [ -z "$$dev" ]; then echo "not plugged in"; \
		elif [ -r /dev/$$dev ]; then echo "/dev/$$dev (readable)"; \
		else echo "/dev/$$dev (no access: udev rule missing, replug the pad)"; fi
	@printf "udev rule:  "; test -f $(UDEV_RULE) && echo "$(UDEV_RULE)" || echo "missing"
	@printf "log:        "; test -f $(LOG) && echo "$(LOG) ($$(wc -l < $(LOG)) lines)" || echo "$(LOG) (empty)"

.PHONY: logs
logs:
	@test -f $(LOG) && tail -n 50 $(LOG) || echo "No log yet at $(LOG). It appears once Chrome starts helperpad."

.PHONY: logs-follow
logs-follow:
	@mkdir -p $(dir $(LOG)) && touch $(LOG) && tail -f $(LOG)

.PHONY: logs-clear
logs-clear:
	@: > $(LOG) 2>/dev/null; echo "Cleared $(LOG)"

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
