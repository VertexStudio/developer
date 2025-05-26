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
