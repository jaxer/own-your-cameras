PYTHON ?= python3
.PHONY: tools-test fast area gate
tools-test:
	$(PYTHON) -B tools/mi-360/test_tools.py
	$(PYTHON) -B tools/reolink-e1-zoom/test_tools.py
fast: tools-test
	$(MAKE) -C firmware fast
area: tools-test
	$(MAKE) -C firmware area
gate: area
