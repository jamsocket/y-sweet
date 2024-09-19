use anyhow::Result;
use git2::Repository;
use std::path::Path;

struct Git {
    repo: Repository,
}

impl Git {
    pub fn new(path: &Path) -> Result<Self> {
        let repo = Repository::open(path)?;
        Ok(Self { repo })
    }

    pub fn ensure_clean(&self) -> Result<()> {
        let head = self.repo.head()?;
        let head_commit = self.repo.find_commit(head.target().unwrap())?;
        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(Some(&head_commit.tree()?), None)?;
        if diff.deltas().len() > 0 {
            anyhow::bail!("Repository is not clean");
        }
        Ok(())
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

    pub fn checkout_new_branch(&self, branch: &str) -> Result<()> {
        let head = self.repo.head()?;
        let oid = head.target().unwrap();
        let commit = self.repo.find_commit(oid)?;
        let branch_ref = self.repo.branch(branch, &commit, false)?;
        self.repo.set_head(branch_ref.get().name().unwrap())?;
        self.repo.checkout_tree(&commit.as_object(), None)?;
        Ok(())
    }

    pub fn push(&self) -> Result<()> {
        let mut remote = self.repo.find_remote("origin")?;
        remote.push::<&str>(&[], None)?;
        Ok(())
    }
}