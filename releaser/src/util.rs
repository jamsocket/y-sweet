use dialoguer::{self, Select};
use semver::Version;
use std::fmt::Display;

pub fn wrapped_select<T: Clone>(items: Vec<(T, String)>) -> T {
    let selection = Select::new()
        .items(
            &items
                .iter()
                .map(|(item, name)| name.to_string())
                .collect::<Vec<String>>(),
        )
        .interact()
        .unwrap();

    items[selection].0.clone()
}

pub enum BumpType {
    Patch,
    Minor,
    Major,
}

impl Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BumpType::Patch => write!(f, "patch"),
            BumpType::Minor => write!(f, "minor"),
            BumpType::Major => write!(f, "major"),
        }
    }
}

impl BumpType {
    pub fn bump(&self, mut version: Version) -> Version {
        match self {
            BumpType::Patch => {
                version.patch = version.patch + 1;
            }
            BumpType::Minor => {
                version.minor = version.minor + 1;
                version.patch = 0;
            }
            BumpType::Major => {
                version.major = version.major + 1;
                version.minor = 0;
                version.patch = 0;
            }
        }

        version
    }
}
