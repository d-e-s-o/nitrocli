# Makefile

#/***************************************************************************
# *   Copyright (C) 2017-2019 Daniel Mueller (deso@posteo.net)              *
# *                                                                         *
# *   This program is free software: you can redistribute it and/or modify  *
# *   it under the terms of the GNU General Public License as published by  *
# *   the Free Software Foundation, either version 3 of the License, or     *
# *   (at your option) any later version.                                   *
# *                                                                         *
# *   This program is distributed in the hope that it will be useful,       *
# *   but WITHOUT ANY WARRANTY; without even the implied warranty of        *
# *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
# *   GNU General Public License for more details.                          *
# *                                                                         *
# *   You should have received a copy of the GNU General Public License     *
# *   along with this program.  If not, see <http://www.gnu.org/licenses/>. *
# ***************************************************************************/

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
	@wget "https://github.com/d-e-s-o/nitrocli/archive/v$(REL).zip" -O "pkg/nitrocli-$(REL).zip"
	@DIR=$$(mktemp -d) && \
		unzip -q pkg/nitrocli-$(REL).zip -d $${DIR} && \
		echo "Please check that contents of $${DIR}/nitrocli-$(REL) match v$(REL) release." && \
		echo "For example:" && \
		echo "  git checkout v$(REL) && diff -u -r $${DIR}/nitrocli-$(REL) ../" && \
		echo "Then press Return." && \
		(read || (rm -r $${DIR}; false)) && \
		rm -r $${DIR}
	@wget "https://github.com/d-e-s-o/nitrocli/archive/v$(REL).tar.gz" -O "pkg/nitrocli-$(REL).tar.gz"
	@DIR=$$(mktemp -d) && \
		tar -xz -C $${DIR} -f pkg/nitrocli-$(REL).tar.gz && \
		echo "Please check that contents of $${DIR}/nitrocli-$(REL) match v$(REL) release." && \
		echo "For example:" && \
		echo "  git checkout v$(REL) && diff -u -r $${DIR}/nitrocli-$(REL) ../" && \
		echo "Then press Return." && \
		(read || (rm -r $${DIR}; false)) && \
		rm -r $${DIR}
	@cd pkg && sha256sum nitrocli-$(REL).tar.gz nitrocli-$(REL).zip > nitrocli-$(REL).DIGEST
	@gpg --sign --armor --detach-sign --default-key=$(KEY) --yes \
		--output pkg/nitrocli-$(REL).DIGEST.sig pkg/nitrocli-$(REL).DIGEST
	@gpg --verify pkg/nitrocli-$(REL).DIGEST.sig
	@cd pkg && sha256sum --check < nitrocli-$(REL).DIGEST
	@echo "All checks successful. Please attach"
	@echo "  pkg/nitrocli-$(REL).DIGEST"
	@echo "  pkg/nitrocli-$(REL).DIGEST.sig"
	@echo "to https://github.com/d-e-s-o/nitrocli/releases/tag/v$(REL)"
