# Text Editor Tool: File Content Manipulation

Provides commands to perform text editing operations on files, such as viewing, creating, overwriting, and modifying content, along with an undo capability for recent changes.

This tool is designed to allow an LLM to programmatically interact with file content in a controlled manner.

## When to Use This Tool

This tool is ideal for tasks such as:

* **Viewing File Content**: Inspecting the contents of configuration files, source code, logs, or any text-based file.
* **Creating New Files**: Drafting new text files, scripts, or documents from scratch.
* **Overwriting Existing Files**: Completely replacing the content of an existing file with new text (e.g., updating a configuration).
* **Making Specific Modifications**: Performing targeted string replacements within a file (e.g., correcting a typo, updating a value).
* **Correcting Recent Edits**: Reverting the last file modification if an error was made.

## Commands Overview

The `command` parameter specifies the operation to perform. Allowed options are:

* `view`: View the content of a file.
* `write`: Create or overwrite a file with the given content.
* `str_replace`: Replace a specific string in a file with a new string.
* `undo_edit`: Undo the last edit made by `write` or `str_replace` to a file.

## General Parameters

These parameters are used by most commands:

* `command` (string, **required**): One of `view`, `write`, `str_replace`, `undo_edit`.
* `path` (string, **required**): Absolute path to the file to operate on (e.g., `/project/config.txt`).

## Detailed Command Descriptions and Parameters

### 1. `view`
* **Purpose**: Reads and returns the content of the specified file.
* **Output**: The file's content, formatted in a Markdown code block with language detection.
* **Limitations**:
    * Files are limited to 400KB in size.
    * File content is limited to 400,000 characters.

### 2. `write`
* **Purpose**: Creates a new file or fully overwrites an existing file with the provided text.
* **Parameters**:
    * `file_text` (string, **required**): The entire new content for the file.
* **Important Notes**:
    * **Full Overwrite**: This command completely replaces the file's content. If you only mean to modify a part of the file, consider using `str_replace` or a view-modify-write sequence.
    * Parent directories will be created if they do not exist.
    * The input `file_text` is limited to 400,000 characters.
* **Output**: A success message and the newly written content, formatted in a Markdown code block.

### 3. `str_replace`
* **Purpose**: Replaces an existing string within a file with a new string.
* **Parameters**:
    * `old_str` (string, **required**): The exact string to be replaced. This string must appear exactly once in the file.
    * `new_str` (string, **required**): The string that will replace `old_str`.
* **Important Notes**:
    * **Exact and Unique Match**: The `old_str` must be an *exact and unique* segment of the file content, including any whitespace. If `old_str` is not found, or if it appears multiple times, the operation will fail.
* **Output**: A success message and a snippet showing the context of the change.

### 4. `undo_edit`
* **Purpose**: Reverts the last change made to a file by a `write` or `str_replace` operation performed by this tool.
* **Important Notes**:
    * The system maintains a history of recent states (e.g., up to 10, configurable by the server admin) for each edited file.
    * If a file was newly created by a `write` command, undoing that write will effectively revert the file to an empty state (or how it was before its first save by this tool).
* **Output**: A success message indicating the undo operation was performed.

## Best Practices for LLMs

* **Verify Before Modifying**: Especially for `str_replace` or `write` on existing files, consider using `view` first to understand the current file structure and content. This helps in formulating accurate `old_str` values or ensuring `file_text` for `write` is complete.
* **Handle `write` with Care**: Remember that `write` is a destructive operation (full overwrite). Ensure `file_text` contains all the content the file should have, not just the parts you are changing.
* **Exactness in `str_replace`**: When using `str_replace`, provide enough of the surrounding text in `old_str` to ensure it's the unique segment you intend to change. The tool will verify uniqueness, but providing an accurate `old_str` is key.
* **Use `undo_edit` Promptly**: If you suspect an edit was incorrect, use `undo_edit` as soon as possible.
* **Check File Paths**: Ensure the `path` provided is an absolute path to a *file*, not a directory (unless the intention is for `write` to create a file within a path where parent directories might need creation).

## Important Limitations

* **File Size/Character Limits**:
    * `view`: Max file size 400KB, max characters 400,000.
    * `write` (input `file_text`): Max characters 400,000.
* **Undo History**: Undo history is limited per file (e.g., to the last 10 states, server configurable).
* **No Directory Operations**: This tool operates on individual files. It does not list directories, delete directories, or perform recursive operations on directories.