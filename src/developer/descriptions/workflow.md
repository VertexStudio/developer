Manages multi-step problem-solving processes with support for sequential progression, branching paths, and step revisions.

Use this tool to:
- Break down complex problems into sequential steps
- Create alternative solution paths through branching
- Revise previous steps as your understanding evolves
- Maintain context across multi-step reasoning processes

Returns JSON response with workflow status.

## Parameters

### Required
- `step_description`: Detailed description of what this step accomplishes
- `step_number`: Current position in the workflow sequence (e.g., 1 for first step)
- `total_steps`: Estimated total number of steps in the complete workflow
- `next_step_needed`: Set to true if another step will follow this one, false if this is the final step

### Optional
- `is_step_revision`: Set to true if this step revises a previous step
- `revises_step`: If revising a previous step, specify which step number is being revised
- `branch_from_step`: If creating a branch, specify which step number this branch starts from
- `branch_id`: A unique identifier for this branch (required when creating a branch)
- `needs_more_steps`: Indicates whether additional steps are required to complete the workflow

## Features

- **Sequential Progression**: Track steps in order with automatic numbering
- **Branching**: Create alternative solution paths from any step
- **Step Revision**: Update and improve previous steps as understanding evolves

- **Context Preservation**: Maintain workflow state across multiple interactions
- **Validation**: Automatic validation of step relationships and branching logic

## Output

Returns a JSON response containing:
- Current step information and status
- Branch information and active branch status
- Step count and completion status 