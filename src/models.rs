//! Typed serde models for the Bitbucket API responses we read.
//!
//! These replace the PHP `array_get('a.b.c')` dot-lookups with compile-time
//! field access. Only the fields the CLI actually uses are modeled; unknown
//! fields are ignored by serde.

use serde::Deserialize;

/// A paginated collection envelope (`values` + optional `next` URL + `size`).
#[derive(Debug, Deserialize)]
pub struct Paginated<T> {
    #[serde(default = "Vec::new")]
    pub values: Vec<T>,
    #[serde(default)]
    pub next: Option<String>,
}

/// A branch ref from `/refs/branches`.
#[derive(Debug, Deserialize)]
pub struct Branch {
    pub name: String,
    #[serde(default)]
    pub target: Option<BranchTarget>,
}

#[derive(Debug, Deserialize)]
pub struct BranchTarget {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub author: Option<Author>,
}

/// A commit author. `user.display_name` when the author maps to a Bitbucket
/// user; otherwise the raw `name <email>` string.
#[derive(Debug, Deserialize)]
pub struct Author {
    #[serde(default)]
    pub user: Option<AuthorUser>,
    #[serde(default)]
    pub raw: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthorUser {
    #[serde(default)]
    pub display_name: Option<String>,
}

// ---- pull requests --------------------------------------------------------

/// A pull request as returned by the list endpoint (`state=OPEN`).
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    #[serde(default)]
    pub author: Option<PrAuthor>,
    #[serde(default)]
    pub source: Option<PrRef>,
    #[serde(default)]
    pub destination: Option<PrRef>,
    #[serde(default)]
    pub links: Option<Links>,
}

#[derive(Debug, Deserialize)]
pub struct PrAuthor {
    #[serde(default)]
    pub nickname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    #[serde(default)]
    pub branch: Option<BranchName>,
}

#[derive(Debug, Deserialize)]
pub struct BranchName {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    #[serde(default)]
    pub html: Option<Href>,
}

#[derive(Debug, Deserialize)]
pub struct Href {
    #[serde(default)]
    pub href: Option<String>,
}

/// The detailed PR (`/pullrequests/{id}`) carrying reviewers and participants.
#[derive(Debug, Deserialize)]
pub struct PullRequestDetail {
    #[serde(default)]
    pub reviewers: Vec<DisplayNameUser>,
    #[serde(default)]
    pub participants: Vec<Participant>,
}

#[derive(Debug, Deserialize)]
pub struct DisplayNameUser {
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Participant {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub user: Option<DisplayNameUser>,
}

/// A diffstat row (`/pullrequests/{id}/diffstat`).
#[derive(Debug, Deserialize)]
pub struct DiffstatRow {
    #[serde(default)]
    pub new: Option<DiffstatPath>,
}

#[derive(Debug, Deserialize)]
pub struct DiffstatPath {
    #[serde(default)]
    pub path: Option<String>,
}

/// A commit (`/pullrequests/{id}/commits`).
#[derive(Debug, Deserialize)]
pub struct Commit {
    #[serde(default)]
    pub summary: Option<CommitSummary>,
}

#[derive(Debug, Deserialize)]
pub struct CommitSummary {
    #[serde(default)]
    pub raw: Option<String>,
}

/// The authenticated user (`/user`).
#[derive(Debug, Deserialize)]
pub struct CurrentUser {
    #[serde(default)]
    pub uuid: Option<String>,
}

impl PullRequest {
    pub fn author_nickname(&self) -> String {
        self.author
            .as_ref()
            .and_then(|a| a.nickname.clone())
            .unwrap_or_default()
    }
    pub fn source_branch(&self) -> String {
        branch_name(&self.source)
    }
    pub fn destination_branch(&self) -> String {
        branch_name(&self.destination)
    }
    pub fn html_link(&self) -> String {
        self.links
            .as_ref()
            .and_then(|l| l.html.as_ref())
            .and_then(|h| h.href.clone())
            .unwrap_or_default()
    }
}

fn branch_name(r: &Option<PrRef>) -> String {
    r.as_ref()
        .and_then(|r| r.branch.as_ref())
        .and_then(|b| b.name.clone())
        .unwrap_or_default()
}

impl Branch {
    /// Resolve the display owner: `target.author.user.display_name`, falling
    /// back to `target.author.raw` (ports `Branch.php:69-70`).
    pub fn owner(&self) -> String {
        let author = self.target.as_ref().and_then(|t| t.author.as_ref());
        author
            .and_then(|a| a.user.as_ref())
            .and_then(|u| u.display_name.clone())
            .or_else(|| author.and_then(|a| a.raw.clone()))
            .unwrap_or_default()
    }

    /// Resolve the target date, if any.
    pub fn date(&self) -> String {
        self.target
            .as_ref()
            .and_then(|t| t.date.clone())
            .unwrap_or_default()
    }
}
