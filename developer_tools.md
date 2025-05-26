Extension: developer
  Tool: developer__shell
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

      Arguments (input schema):
        {
          "properties": {
            "command": {
              "type": "string"
            }
          },
          "required": [
            "command"
          ],
          "type": "object"
        }

  Tool: developer__text_editor
      Perform text editing operations on files.
      
      The `command` parameter specifies the operation to perform. Allowed options are:
      - `view`: View the content of a file.
      - `write`: Create or overwrite a file with the given content
      - `str_replace`: Replace a string in a file with a new string.
      - `undo_edit`: Undo the last edit made to a file.
      
      To use the write command, you must specify `file_text` which will become the new content of the file. Be careful with
      existing files! This is a full overwrite, so you must include everything - not just sections you are modifying.
      
      To use the str_replace command, you must specify both `old_str` and `new_str` - the `old_str` needs to exactly match one
      unique section of the original file, including any whitespace. Make sure to include enough context that the match is not
      ambiguous. The entire original string will be replaced with `new_str`.

      Arguments (input schema):
        {
          "properties": {
            "command": {
              "description": "Allowed options are: `view`, `write`, `str_replace`, undo_edit`.",
              "enum": [
                "view",
                "write",
                "str_replace",
                "undo_edit"
              ],
              "type": "string"
            },
            "file_text": {
              "type": "string"
            },
            "new_str": {
              "type": "string"
            },
            "old_str": {
              "type": "string"
            },
            "path": {
              "description": "Absolute path to file or directory, e.g. `/repo/file.py` or `/repo`.",
              "type": "string"
            }
          },
          "required": [
            "command",
            "path"
          ],
          "type": "object"
        }

  Tool: developer__list_windows
      List all available window titles that can be used with screen_capture.
      Returns a list of window titles that can be used with the window_title parameter
      of the screen_capture tool.

      Arguments (input schema):
        {
          "properties": {},
          "required": [],
          "type": "object"
        }

  Tool: developer__screen_capture
      Capture a screenshot of a specified display or window.
      You can capture either:
      1. A full display (monitor) using the display parameter
      2. A specific window by its title using the window_title parameter
      
      Only one of display or window_title should be specified.

      Arguments (input schema):
        {
          "properties": {
            "display": {
              "default": 0,
              "description": "The display number to capture (0 is main display)",
              "type": "integer"
            },
            "window_title": {
              "default": null,
              "description": "Optional: the exact title of the window to capture. use the list_windows tool to find the available windows.",
              "type": "string"
            }
          },
          "required": [],
          "type": "object"
        }

  Tool: developer__image_processor
      Process an image file from disk. The image will be:
      1. Resized if larger than max width while maintaining aspect ratio
      2. Converted to PNG format
      3. Returned as base64 encoded data
      
      This allows processing image files for use in the conversation.

      Arguments (input schema):
        {
          "properties": {
            "path": {
              "description": "Absolute path to the image file to process",
              "type": "string"
            }
          },
          "required": [
            "path"
          ],
          "type": "object"
        }