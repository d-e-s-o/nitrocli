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
