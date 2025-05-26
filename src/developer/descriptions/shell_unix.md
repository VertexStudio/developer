Execute a command in the shell.

This will return the output and error concatenated into a single string, as
you would see from running on the command line. There will also be an indication
of if the command succeeded or failed.

Avoid commands that produce a large amount of output, and consider piping those outputs to files.
If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
this tool does not run indefinitely.

**Important**: Each shell command runs in its own process. Things like directory changes or
sourcing files do not persist between tool calls. So you may need to repeat them each time by
stringing together commands, e.g. `cd example && ls` or `source env/bin/activate && pip install numpy`

**Important**: Use ripgrep - `rg` - when you need to locate a file or a code reference, other solutions
may show ignored or hidden files. For example *do not* use `find` or `ls -r`
  - List files by name: `rg --files | rg <filename>`
  - List files that contain a regex: `rg '<regex>' -l` 
