use anyhow::Result;
use git2::Repository;
use std::{path::Path, process::Command};

/// Wraps some git commands and libgit2 calls.
pub struct Git {
    repo: Repository,
}

impl Git {
    pub fn new(path: &Path) -> Result<Self> {
        let repo = Repository::open(path)?;
        Ok(Self { repo })
    }

    /// Check if the repository is clean (i.e. no uncommitted changes).
    pub fn clean(&self) -> Result<bool> {
        let head = self.repo.head()?;
        let head_commit = self.repo.find_commit(head.target().unwrap())?;
        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(Some(&head_commit.tree()?), None)?;
        if diff.deltas().len() > 0 {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn get_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        if head.is_branch() {
            let branch = head.shorthand().unwrap_or_default();
            Ok(branch.to_string())
        } else {
            anyhow::bail!("Git HEAD is not pointing to a branch")
        }
    }

    /// Commit all changes in the current directory with the given message.
    pub fn commit_all(&self, message: &str) -> Result<()> {
        // This uses the running user's git config, so it's easier to do it with
        // the CLI than libgit2.
        Command::new("git").args(["add", "."]).status()?;
        Command::new("git")
            .args(["commit", "-m", message])
            .status()?;
        Ok(())
    }

    /// Checkout a new branch with the given name.
    pub fn checkout_new_branch(&self, branch: &str) -> Result<()> {
        let head = self.repo.head()?;
        let oid = head.target().unwrap();
        let commit = self.repo.find_commit(oid)?;
        let branch_ref = self.repo.branch(branch, &commit, false)?;
        self.repo.set_head(branch_ref.get().name().unwrap())?;
        self.repo.checkout_tree(commit.as_object(), None)?;
        Ok(())
    }

    /// Push the current branch to the remote repository.
    pub fn push(&self) -> Result<()> {
        let branch = self.get_branch()?;
        Command::new("git")
            .args(["push", "--set-upstream", "origin", &branch])
            .status()?;
        Ok(())
    }
}
