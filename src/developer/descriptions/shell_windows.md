Execute a command in the shell.

This will return the output and error concatenated into a single string, as
you would see from running on the command line. There will also be an indication
of if the command succeeded or failed.

Avoid commands that produce a large amount of output, and consider piping those outputs to files.

**Important**: For searching files and code:

Preferred: Use ripgrep (`rg`) when available - it respects .gitignore and is fast:
  - To locate a file by name: `rg --files | rg example.py`
  - To locate content inside files: `rg 'class Example'`

Alternative Windows commands (if ripgrep is not installed):
  - To locate a file by name: `dir /s /b example.py`
  - To locate content inside files: `findstr /s /i "class Example" *.py`

Note: Alternative commands may show ignored/hidden files that should be excluded. 