"""
Git hook installer for Rocky.

`rocky install` writes a post-commit hook that calls `rocky --after`
with the last commit message after every commit.
"""

import subprocess
import sys
from pathlib import Path

_HOOK_SCRIPT = """\
#!/bin/sh
# Rocky post-commit hook
# Runs Rocky in after-mode to review what was just committed.
# Remove this file or run `rocky uninstall` to disable.
COMMIT_MSG=$(git log -1 --pretty=%B)
rocky --after "$COMMIT_MSG" 2>/dev/null || true
"""

_HOOK_MARKER = "# Rocky post-commit hook"


def _find_git_root() -> Path | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True, text=True, check=True,
        )
        return Path(result.stdout.strip())
    except subprocess.CalledProcessError:
        return None


def install_git_hook() -> tuple[bool, str]:
    """
    Write a post-commit hook into the nearest git repo.
    Returns (success, message).
    """
    git_root = _find_git_root()
    if not git_root:
        return False, "not inside a git repository"

    hooks_dir = git_root / ".git" / "hooks"
    hooks_dir.mkdir(exist_ok=True)
    hook_file = hooks_dir / "post-commit"

    if hook_file.exists():
        content = hook_file.read_text()
        if _HOOK_MARKER in content:
            return True, f"Rocky hook already installed at {hook_file}"
        # Another hook exists — append rather than overwrite
        hook_file.write_text(content.rstrip() + "\n\n" + _HOOK_SCRIPT)
        hook_file.chmod(0o755)
        return True, f"Rocky appended to existing hook at {hook_file}"

    hook_file.write_text(_HOOK_SCRIPT)
    hook_file.chmod(0o755)
    return True, f"Hook installed at {hook_file}"


def uninstall_git_hook() -> tuple[bool, str]:
    """Remove Rocky's block from the post-commit hook."""
    git_root = _find_git_root()
    if not git_root:
        return False, "not inside a git repository"

    hook_file = git_root / ".git" / "hooks" / "post-commit"
    if not hook_file.exists():
        return True, "no post-commit hook found — nothing to remove"

    content = hook_file.read_text()
    if _HOOK_MARKER not in content:
        return True, "Rocky hook not found in post-commit — nothing to remove"

    # Remove Rocky's block (from marker line to end of script block)
    lines = content.splitlines(keepends=True)
    filtered = []
    skip = False
    for line in lines:
        if _HOOK_MARKER in line:
            skip = True
        if not skip:
            filtered.append(line)

    new_content = "".join(filtered).rstrip()
    if new_content in ("", "#!/bin/sh"):
        hook_file.unlink()
        return True, f"Hook removed: {hook_file}"

    hook_file.write_text(new_content + "\n")
    return True, f"Rocky removed from hook at {hook_file}"
