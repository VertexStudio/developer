# Workflow Tool: Guiding Complex Problem-Solving

Manages multi-step problem-solving processes with support for sequential progression, branching paths, and step revisions. This tool is designed to help you (the LLM) structure your reasoning, explore alternatives, and adapt your approach as your understanding of a problem evolves.

Returns a JSON response detailing the current workflow status, including the last step taken, active branches, and overall progress.

## When to Use This Workflow Tool

This tool is most effective when you need to:

* **Deconstruct Complex Problems**: Break down large or multifaceted tasks into a manageable sequence of discrete steps.
* **Plan and Design Iteratively**: Develop plans or designs where initial assumptions might need revision as more information is processed.
* **Explore Multiple Solution Paths**: Investigate different approaches or hypotheses by creating distinct branches in your reasoning process.
* **Perform In-depth Analysis with Course Correction**: Conduct detailed analysis that may require backtracking or revising earlier conclusions.
* **Handle Problems with Unclear Scope**: Tackle tasks where the full extent of steps or requirements isn't known at the outset.
* **Maintain Long-Term Context**: Keep track of progress and decisions across a series of interactions for a multi-step task.
* **Systematically Generate and Verify Hypotheses**: Formulate a potential solution or explanation, then use subsequent steps to test and validate it.
* **Document Your Reasoning Process**: Clearly lay out each step of your thought process for transparency and review.

## Best Practices for Using the Workflow Tool

To leverage this tool most effectively, please follow these guidelines:

1.  **Start with an Initial Estimate, Adapt as Needed**: Begin with a reasonable estimate for `total_steps`. However, feel free to adjust this value (up or down) in subsequent steps as your understanding of the task's complexity changes. The tool will accommodate these adjustments.
2.  **Embrace Revisions**: Don't hesitate to use `is_step_revision` and `revises_step` if you realize an earlier step needs correction or refinement. Clearly describe what is being revised and why.
3.  **Utilize Branching for Alternatives**: If multiple approaches seem plausible, use `branch_from_step` and `branch_id` to explore them in parallel without losing the main line of thought. Give your branches descriptive `branch_id`s.
4.  **Be Explicit About Uncertainty**: If you are unsure about a step or a direction, you can note this in the `step_description`.
5.  **Clearly Define Each Step**: Ensure each `step_description` is detailed and clearly states what that specific step accomplishes, any conclusions drawn, or any questions that arise.
6.  **Manage `next_step_needed` and `needs_more_steps` Deliberately**:
    * Set `next_step_needed` to `false` only when the *current specific sequence or sub-task* is complete.
    * Use `needs_more_steps` (optional) to indicate if the *overall problem or workflow* requires further steps, even if the current local sequence (`next_step_needed: false`) is ending. For example, you might finish a branch (`next_step_needed: false`) but still need to return to the main workflow or start a new one (`needs_more_steps: true`). Set `needs_more_steps: false` when you are confident the entire problem is resolved.
7.  **Hypothesis Management**:
    * When appropriate, use a step to explicitly state a hypothesis in the `step_description`.
    * In subsequent steps, describe how you are testing or verifying this hypothesis.
8.  **Sequential Integrity**: Ensure `step_number` increments logically. While you can revise past steps, new steps should generally follow in sequence.
9.  **Final Answer**: When the entire workflow is complete (`next_step_needed: false` and `needs_more_steps: false` (or omitted and implied false)), the final step's description should ideally contain or clearly lead to the final answer or solution.

## Parameters

### Required
-   `step_description` (string): Detailed description of what this step accomplishes, including any reasoning, conclusions, or questions.
-   `step_number` (integer): Current position in the workflow sequence (e.g., 1 for first step). Must be >= 1.
-   `total_steps` (integer): Your current best estimate of the total number of steps needed for the *entire* workflow. Can be adjusted in subsequent steps. Must be >= `step_number`.
-   `next_step_needed` (boolean): Set to `true` if another step is expected to *immediately follow this one* in the current sequence or branch. Set to `false` if this is the last step of the current sequence/branch.

### Optional
-   `is_step_revision` (boolean): Set to `true` if this step revises or corrects a previous step.
-   `revises_step` (integer): If `is_step_revision` is true, specify the `step_number` of the step being revised. Must be a valid, existing step number.
-   `branch_from_step` (integer): If creating a new branch, specify the `step_number` from which this new branch originates. Must be a valid, existing step number.
-   `branch_id` (string): A unique and descriptive identifier for the branch being created or continued (e.g., "explore-alternative-api", "deep-dive-cause-analysis"). Required if `branch_from_step` is provided for a new branch, or if continuing an existing branch.
-   `needs_more_steps` (boolean): Set to `true` if you anticipate more steps are needed to fully resolve the *overall problem*, even if `next_step_needed` is `false` for the current step (e.g., you've completed a branch but need to return to the main flow or analyze another aspect). If omitted, the system may infer based on `next_step_needed` and `total_steps`. Set explicitly to `false` when the entire problem is solved.

## Features Summary

-   **Sequential Progression**: Steps are tracked in order.
-   **Dynamic `total_steps`**: The `total_steps` can be adjusted by you as the workflow progresses.
-   **Branching**: Create and switch between alternative solution paths.
-   **Step Revision**: Mark steps that update or correct prior steps.
-   **Context Preservation**: Workflow state (history, branches) is maintained across calls.
-   **Input Validation**: Ensures logical consistency of parameters.

## Output

Returns a JSON object detailing the current workflow status:
-   `step_number` (integer): The number of the step just processed.
-   `total_steps` (integer): The current total number of steps, possibly updated by the last call.
-   `next_step_needed` (boolean): The `next_step_needed` value from the last step.
-   `last_step_description` (string): The description of the step just processed.
-   `current_branch` (string | null): The ID of the active branch, if any.
-   `branches` (array of strings): A list of all unique `branch_id`s created so far.
-   `step_history_length` (integer): The total number of steps recorded in the main history (includes steps from all branches). 