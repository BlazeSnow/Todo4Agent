# Todo4Agent backend messages (English, US).
# Keys are kebab-case; {$name} marks placeholders. Mirrors src/i18n/en-US.ts.

# ---------- Common ----------
db-error = Database error: {$err}

# ---------- Auth ----------
not-signed-in = Not signed in or session expired
invalid-credentials = Invalid username or password
invalid-credentials-user = Invalid username or password: {$user}
username-empty = Username cannot be empty
password-too-short = Password must be at least 4 characters
registration-disabled = Registration is disabled
username-taken = Username already exists
new-password-too-short = New password must be at least 4 characters
wrong-password = Current password is incorrect

# ---------- Groups ----------
group-name-empty = Group name cannot be empty
group-name-taken = Group name already exists
no-fields-to-update = No fields to update
no-group-rename = The system group "Ungrouped" cannot be renamed
group-not-found = Group not found
no-group-lock = The system group "Ungrouped" cannot be locked; it is where tasks go when their group is deleted
no-group-delete = The system group "Ungrouped" cannot be deleted
group-ids-empty = group_ids cannot be empty

# ---------- Tasks ----------
task-title-empty = Task title cannot be empty
status-invalid = status must be either pending or done
task-not-found = Task not found
task-not-found-or-archived = Task not found or already archived
task-not-archived = Task is not in the archive
task-ids-empty = task_ids cannot be empty

# ---------- Trash ----------
task-not-in-trash = Task is not in the trash
group-not-in-trash = Group is not in the trash

# ---------- Import / Settings / Prompt ----------
import-empty = Import content is empty
port-range = Port range: 1024-65535
prompt-save-error = Unexpected result while saving the prompt

# ---------- MCP: startup & runtime ----------
mcp-env-credentials = MCP requires the TODO4AGENT_USERNAME and TODO4AGENT_PASSWORD environment variables (run todo4agent help for setup instructions)
verify-user-failed = Failed to verify user: {$err}
locked-err = List "{$name}" is locked and cannot be edited by the Agent (ask the user to unlock it from the sidebar group menu)
import-locked = The document contains locked lists: {$names} (ask the user to import from the UI or unlock them first)
import-doc-invalid = Invalid argument: doc must be a task list document JSON: {$err}
unknown-tool = Unknown tool: {$name}
password-changed-note = Password changed; update TODO4AGENT_PASSWORD in the MCP client config accordingly (the current connection is unaffected; the next launch needs the new password)

# ---------- MCP: argument validation ----------
arg-error-required = Invalid argument: {$key} is required
arg-error-required-nonempty = Invalid argument: {$key} is required and cannot be empty
arg-error-required-string = Invalid argument: {$key} is required and must be a string
arg-error-int = Invalid argument: {$key} must be an integer
arg-error-string = Invalid argument: {$key} must be a string
arg-error-bool = Invalid argument: {$key} must be a boolean
arg-error-title-empty = Invalid argument: title cannot be empty
arg-error-status = Invalid argument: status must be either pending or done
arg-error-due = Invalid argument: due_at must be a string or null
arg-error-done = Invalid argument: done is required and must be a boolean
arg-error-new-password-short = Invalid argument: new_password must be at least 4 characters

# ---------- MCP: tool descriptions ----------
tool-app-version = Get the app version
tool-app-release = Get the app release page URL (GitHub Releases)
tool-db-path = Get the database file path in use (local SQLite; override with the TODO4AGENT_DB environment variable)
tool-group-list = List all task groups
tool-group-create = Create a task group; group names must be unique
tool-group-rename = Rename a task group (optionally update its description)
tool-group-delete = Delete a task group (its tasks, archived included, move to "Ungrouped"; the system group "Ungrouped" cannot be deleted)
tool-task-list = List tasks; filterable by group, optionally including archived ones
tool-task-create = Create a task (status defaults to pending)
tool-task-update = Update task fields (only provided fields are changed)
tool-task-complete = Complete / uncomplete a task (toggles the done state)
tool-task-archive = Archive a task (removed from its list; view and restore on the app's Archive page)
tool-task-unarchive = Unarchive a task (returns it to its original list)
tool-task-delete = Delete a task
tool-task-export = Export task lists and the prompt as a JSON document (same structure as the UI export)
tool-task-import = Import a JSON document (same structure as task_export output: same-name groups merge, new groups are created; a prompt field also imports the prompt)
tool-user-password = Change the password of the current account (the user whose credentials started this server); all its signed-in sessions are revoked on success, so update TODO4AGENT_PASSWORD in the client config accordingly
tool-prompt-get = Read the current user's Agent prompt (collaboration guidelines, like AGENTS.md); empty by default, an empty content means not set
tool-prompt-update = Fully update the current user's Agent prompt; fetch it with prompt_get first, edit, then write the whole content back; an empty string clears it

# ---------- MCP: parameter descriptions ----------
tp-name-required = Group name (required)
tp-desc-purpose = Group description (optional): what this list is for
tp-group-id = Group id
tp-name-new = New group name (required)
tp-desc-update = Group description (optional): updated when provided, empty string clears it
tp-group-id-required = Group id (required)
tp-group-id-optional-all = Group id (optional; defaults to all groups)
tp-include-archived = Include archived tasks (optional; defaults to false for unarchived only)
tp-owning-group-required = Owning group id (required)
tp-title-required = Task title (required)
tp-desc-optional = Detailed description (optional)
tp-due-optional = Due time, ISO8601 (optional)
tp-task-id-required = Task id (required)
tp-move-group-id = Group id to move the task to
tp-new-title = New title
tp-new-desc = New description
tp-new-status = New status
tp-new-due = New due time; null clears it
tp-done = true marks done, false restores pending (required)
tp-doc = Task list document (required): contains version, exported_at and groups; each group has name and tasks; each task has title, description, status, due_at
tp-old-password = Current password (required)
tp-new-password = New password, at least 4 characters (required)
tp-content = Full new prompt text; an empty string clears it (required)
