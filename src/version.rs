use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreRelease {
    pub label: String,
    pub number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre: Option<PreRelease>,
}

impl SemVer {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch, pre: None }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let (version_part, pre_part) = match s.split_once('-') {
            Some((v, p)) => (v, Some(p)),
            None => (s, None),
        };
        let parts: Vec<&str> = version_part.splitn(3, '.').collect();
        if parts.len() != 3 { return None; }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        let pre = match pre_part {
            Some(p) => {
                let pre_parts: Vec<&str> = p.splitn(2, '.').collect();
                if pre_parts.len() != 2 { return None; }
                let label = pre_parts[0];
                if label.is_empty() { return None; }
                let number = pre_parts[1].parse().ok()?;
                Some(PreRelease { label: label.to_string(), number })
            }
            None => None,
        };
        Some(Self { major, minor, patch, pre })
    }

    pub fn bump_minor(&self) -> Self { Self::new(self.major, self.minor + 1, 0) }
    pub fn bump_patch(&self) -> Self { Self::new(self.major, self.minor, self.patch + 1) }

    pub fn with_rc(&self, number: u32) -> Self {
        Self {
            major: self.major, minor: self.minor, patch: self.patch,
            pre: Some(PreRelease { label: "rc".to_string(), number }),
        }
    }

    pub fn bump_rc(&self) -> Self {
        match &self.pre {
            Some(pre) => Self {
                major: self.major, minor: self.minor, patch: self.patch,
                pre: Some(PreRelease { label: pre.label.clone(), number: pre.number + 1 }),
            },
            None => self.with_rc(1),
        }
    }

    pub fn to_release(&self) -> Self { Self::new(self.major, self.minor, self.patch) }
    pub fn is_pre_release(&self) -> bool { self.pre.is_some() }
    pub fn tag_name(&self) -> String { format!("v{self}") }
    pub fn release_branch(&self) -> String { format!("release/{}.{}.{}", self.major, self.minor, self.patch) }
    pub fn hotfix_branch(&self) -> String { format!("hotfix/{}", self.to_release()) }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then(match (&self.pre, &other.pre) {
                (None, None) => std::cmp::Ordering::Equal,
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (Some(a), Some(b)) => a.label.cmp(&b.label).then(a.number.cmp(&b.number)),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(other)) }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{}.{}", pre.label, pre.number)?;
        }
        Ok(())
    }
}
