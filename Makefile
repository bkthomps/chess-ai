.DEFAULT_GOAL := it
.PHONY: it
ifeq ($(OS),Windows_NT)
it:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/launch.ps1
else
it:
	sh scripts/launch.sh
endif
