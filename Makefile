# Makefile

# Copyright (C) 2017-2019 The Nitrocli Developers
# SPDX-License-Identifier: GPL-3.0-or-later

SHELL := bash

PS2PDF ?= ps2pdf

NITROCLI_MAN := doc/nitrocli.1
NITROCLI_PDF := $(addsuffix .pdf,$(NITROCLI_MAN))

.PHONY: doc
doc: $(NITROCLI_PDF) $(NITROCLI_HTML)

# We assume and do not check existence of man, which, false, and echo
# commands.
$(NITROCLI_PDF): $(NITROCLI_MAN)
	@which $(PS2PDF) &> /dev/null || \
		(echo "$(PS2PDF) command not found, unable to generate documentation"; false)
	@man --local-file --troff $(<) | $(PS2PDF) - $(@)

KEY ?= 0x952DD6F8F34D8B8E

.PHONY: sign
sign:
	@test -n "$(REL)" || \
		(echo "Please set REL environment variable to the release to verify (e.g., '0.2.1')."; false)
	@mkdir -p pkg/
	wget --quiet "https://github.com/d-e-s-o/nitrocli/archive/v$(REL).zip" \
		-O "pkg/nitrocli-$(REL).zip"
	@set -euo pipefail && DIR1=$$(mktemp -d) && DIR2=$$(mktemp -d) && \
		unzip -q pkg/nitrocli-$(REL).zip -d $${DIR1} && \
		git -C $$(git rev-parse --show-toplevel) archive --prefix=nitrocli-$(REL)/ v$(REL) | \
			tar -x -C $${DIR2} && \
		diff -u -r $${DIR1} $${DIR2} && \
		echo "Github zip archive verified successfully" && \
		(rm -r $${DIR1} && rm -r $${DIR2})
	wget --quiet "https://github.com/d-e-s-o/nitrocli/archive/v$(REL).tar.gz" \
		-O "pkg/nitrocli-$(REL).tar.gz"
	@set -euo pipefail && DIR1=$$(mktemp -d) && DIR2=$$(mktemp -d) && \
		tar -xz -C $${DIR1} -f pkg/nitrocli-$(REL).tar.gz && \
		git -C $$(git rev-parse --show-toplevel) archive --prefix=nitrocli-$(REL)/ v$(REL) | \
			tar -x -C $${DIR2} && \
		diff -u -r $${DIR1} $${DIR2} && \
		echo "Github tarball verified successfully" && \
		(rm -r $${DIR1} && rm -r $${DIR2})
	@cd pkg && sha256sum nitrocli-$(REL).tar.gz nitrocli-$(REL).zip > nitrocli-$(REL).sha256.DIGEST
	@gpg --sign --armor --detach-sign --default-key=$(KEY) --yes \
		--output pkg/nitrocli-$(REL).sha256.DIGEST.sig pkg/nitrocli-$(REL).sha256.DIGEST
	@gpg --verify pkg/nitrocli-$(REL).sha256.DIGEST.sig
	@cd pkg && sha256sum --check < nitrocli-$(REL).sha256.DIGEST
	@echo "All checks successful. Please attach"
	@echo "  pkg/nitrocli-$(REL).sha256.DIGEST"
	@echo "  pkg/nitrocli-$(REL).sha256.DIGEST.sig"
	@echo "to https://github.com/d-e-s-o/nitrocli/releases/tag/v$(REL)"
