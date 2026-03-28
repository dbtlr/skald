pub const SYSTEM: &str = r###"{# System message — prepended to all prompts #}
You are a senior software developer writing git commit messages and pull
request descriptions. You are precise, concise, and focus on communicating
the intent and impact of code changes. You never include explanatory
preamble or postamble — only the requested output."###;

pub const COMMIT_TITLE: &str = r###"{# Commit Title Prompt — generates conventional commit message one-liners #}
You are an expert at writing concise, accurate git commit messages in conventional commit format.

Analyze the following git diff and generate exactly {{ num_suggestions }} commit messages.

Rules:
- Use conventional commit format: type(scope): description
- Types: feat, fix, refactor, docs, test, chore, style, perf, ci, build
- Scope should reflect the primary area of change (module, directory, or feature name)
- Description should be imperative mood ("add" not "added"), lowercase, no trailing period
- Keep each message under 72 characters
- Focus on WHAT changed and WHY, not HOW
- Output one message per line, no numbering, no bullet points, no extra text

{% if context %}
The developer provided this context about the changes:
{{ context }}

Use this to inform the intent of the commit message. The context explains WHY
the changes were made — reflect this in the message.
{% endif %}

<diffstat>
{{ diff_stat }}
</diffstat>

{% if language != "English" %}
IMPORTANT: Write all commit messages in {{ language }}.
{% endif %}"###;

pub const COMMIT_BODY: &str = r###"{# Commit Body Prompt — generates an extended description for a commit #}
You are writing the extended body for a git commit message.

The commit title is:
{{ title }}

Write a concise extended description that explains:
1. WHAT was changed (high-level summary, not a line-by-line restatement of the diff)
2. WHY it was changed (motivation, problem being solved)
3. Any important details about HOW it was done (only if non-obvious)

Format rules:
- Wrap lines at 72 characters
- Start with a brief summary paragraph (2-3 sentences max)
- If there are multiple distinct changes, list them with "- " bullet points
- Do NOT restate the commit title
- Do NOT include generic phrases like "This commit..." or "Changes include..."
- Do NOT list every single file that changed — focus on the meaningful changes
- Keep the total body under 15 lines

{% if context %}
Developer context:
{{ context }}
{% endif %}

<diffstat>
{{ diff_stat }}
</diffstat>

{% if language != "English" %}
IMPORTANT: Write in {{ language }}.
{% endif %}"###;

pub const PR_TITLE: &str = r###"{# PR Title Prompt — generates pull request title suggestions #}
You are writing a pull request title that summarizes an entire branch of work.

This branch ({{ branch }}) is being merged into {{ target_branch }}.

Analyze the full changeset — both the diff and the commit history — and
generate exactly {{ num_suggestions }} pull request titles.

Rules:
- Each title should summarize the overall PURPOSE of the branch, not individual commits
- Use a clear, descriptive format (e.g., "Add OAuth2 token refresh and session management")
- Keep titles under 72 characters
- Titles should make sense to a reviewer seeing the PR in a list
- Do NOT use conventional commit format (no "feat:" prefix) unless the project convention requires it
- Output one title per line, no numbering, no extra text

{% if context %}
Context from the developer:
{{ context }}
{% endif %}

<commits>
{{ commit_log }}
</commits>

<diffstat>
{{ diff_stat }}
</diffstat>

{% if language != "English" %}
IMPORTANT: Write all titles in {{ language }}.
{% endif %}"###;

pub const PR_DESCRIPTION: &str = r###"{# PR Description Prompt — generates a structured PR body #}
You are writing a pull request description for a code review.

Branch {{ branch }} is being merged into {{ target_branch }}.

Generate a clear, well-structured PR description. Use the following format:

## What
A 2-3 sentence summary of what this PR does. Focus on the user-facing or
system-level outcome, not implementation details.

## Why
Brief explanation of the motivation. What problem does this solve?
What requirement does it address?

## Key Changes
A bullet list of the most important changes (3-7 items). Group related
changes together. Focus on the meaningful changes, not every file touched.
Each bullet should be 1-2 sentences.

## Testing
Brief note on how this was tested, or what testing is recommended for
reviewers. If no tests were added, say so and explain why.

---

Formatting rules:
- Use the section headers exactly as shown above (## What, ## Why, etc.)
- Keep the total description under 40 lines
- Be specific — avoid vague phrases like "various improvements" or "code cleanup"
- Do NOT list every file changed — the reviewer can see the file list themselves
- Do NOT include the PR title in the description body
- If there are breaking changes, add a "## Breaking Changes" section after Key Changes

{% if context %}
Context from the developer:
{{ context }}
{% endif %}

<commits>
{{ commit_log }}
</commits>

<diffstat>
{{ diff_stat }}
</diffstat>

{% if language != "English" %}
IMPORTANT: Write all content in {{ language }}.
{% endif %}"###;

pub const EJECT_HEADER: &str = r###"{# ============================================================ #}
{# This prompt template was ejected from skald's defaults.       #}
{# Edit freely — skald will use this file instead of the         #}
{# built-in default. Delete this file to revert to defaults.     #}
{#                                                               #}
{# Available variables:                                          #}
{#   {{ branch }}          - current branch name                 #}
{#   {{ target_branch }}   - PR target branch                    #}
{#   {{ diff_stat }}       - git diff --stat output              #}
{#   {{ context }}         - user-provided --context string      #}
{#   {{ language }}        - configured language                 #}
{#   {{ num_suggestions }} - number of suggestions requested     #}
{#   {{ commit_log }}      - commit log (PR prompts only)        #}
{#   {{ title }}           - commit title (body prompt only)     #}
{#   {{ files_changed }}   - list of changed file paths          #}
{#                                                               #}
{# Tera template syntax: https://keats.github.io/tera/docs/     #}
{# ============================================================ #}"###;

/// Returns all valid template names.
pub fn all_template_names() -> Vec<&'static str> {
    vec!["system", "commit-title", "commit-body", "pr-title", "pr-description"]
}

/// Returns the built-in template content for a given name.
pub fn get_builtin(name: &str) -> Option<&'static str> {
    match name {
        "system" => Some(SYSTEM),
        "commit-title" => Some(COMMIT_TITLE),
        "commit-body" => Some(COMMIT_BODY),
        "pr-title" => Some(PR_TITLE),
        "pr-description" => Some(PR_DESCRIPTION),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_template_names_exist() {
        for name in all_template_names() {
            assert!(
                get_builtin(name).is_some(),
                "Template '{}' listed in all_template_names but not found in get_builtin",
                name
            );
        }
    }

    #[test]
    fn all_templates_listed() {
        let names = all_template_names();
        // Every match arm in get_builtin (except _ => None) should be in the names list
        assert!(names.contains(&"system"));
        assert!(names.contains(&"commit-title"));
        assert!(names.contains(&"commit-body"));
        assert!(names.contains(&"pr-title"));
        assert!(names.contains(&"pr-description"));
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn get_builtin_returns_content() {
        let system = get_builtin("system").unwrap();
        assert!(system.contains("senior software developer"));

        let commit_title = get_builtin("commit-title").unwrap();
        assert!(commit_title.contains("num_suggestions"));
        assert!(commit_title.contains("diff_stat"));

        let commit_body = get_builtin("commit-body").unwrap();
        assert!(commit_body.contains("title"));

        let pr_title = get_builtin("pr-title").unwrap();
        assert!(pr_title.contains("branch"));
        assert!(pr_title.contains("target_branch"));

        let pr_desc = get_builtin("pr-description").unwrap();
        assert!(pr_desc.contains("## What"));
        assert!(pr_desc.contains("## Why"));

        assert!(get_builtin("nonexistent").is_none());
    }

    #[test]
    fn eject_header_documents_variables() {
        let header = EJECT_HEADER;
        assert!(header.contains("branch"));
        assert!(header.contains("target_branch"));
        assert!(header.contains("diff_stat"));
        assert!(header.contains("context"));
        assert!(header.contains("language"));
        assert!(header.contains("num_suggestions"));
        assert!(header.contains("commit_log"));
        assert!(header.contains("title"));
        assert!(header.contains("files_changed"));
        assert!(header.contains("tera"));
    }
}
