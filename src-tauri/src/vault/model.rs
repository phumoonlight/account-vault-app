use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Everything stored inside the encrypted vault file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub folders: Vec<Folder>,
}

/// A named group of entries. Entries point to it with `Entry::folder_id`.
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    /// Unix time in milliseconds.
    #[serde(default)]
    pub created_at: u64,
}

/// What an entry holds. Entries saved before kinds existed are accounts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    /// Login: username, password, website, custom fields.
    #[default]
    Account,
    /// A set of environment variables, stored as dotenv text in `env`.
    Env,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    #[serde(default)]
    #[zeroize(skip)]
    pub kind: EntryKind,
    pub title: String,
    #[serde(default)]
    pub username: String,
    /// Read from vaults saved before email was merged into username; never written.
    #[serde(default, rename = "email", skip_serializing)]
    legacy_email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub custom_fields: Vec<CustomField>,
    /// Dotenv text (`KEY=value` lines) for `EntryKind::Env`; empty for accounts.
    #[serde(default)]
    pub env: String,
    #[serde(default)]
    pub favorite: bool,
    /// Labels for filtering, e.g. "work", "bank". Normalized by `normalize_tags`.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The folder this entry is in, or `None` for no folder.
    #[serde(default)]
    pub folder_id: Option<String>,
    /// Unix time in milliseconds.
    pub created_at: u64,
    pub updated_at: u64,
}

/// User-defined extra field, e.g. "PIN", "Recovery code", "Security question".
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub name: String,
    pub value: String,
    /// Hidden in the UI by default, like a password.
    #[serde(default)]
    pub secret: bool,
}

/// Editable fields sent from the frontend when adding or updating an entry.
#[derive(Debug, Clone, Default, Deserialize, Zeroize, ZeroizeOnDrop)]
// Field-level `default` (not container-level): serde's container default
// moves fields out of a `Default` value, which `ZeroizeOnDrop` forbids.
#[serde(rename_all = "camelCase")]
pub struct EntryInput {
    #[serde(default)]
    #[zeroize(skip)]
    pub kind: EntryKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub custom_fields: Vec<CustomField>,
    #[serde(default)]
    pub env: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub folder_id: Option<String>,
}

/// List view of an entry, without the password, secret fields or env values.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub kind: EntryKind,
    pub title: String,
    pub username: String,
    pub url: String,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub folder_id: Option<String>,
    pub updated_at: u64,
}

pub const MAX_TAGS: usize = 20;
pub const MAX_TAG_LEN: usize = 32;

/// Trims and collapses whitespace, truncates to `MAX_TAG_LEN` characters,
/// drops empties and case-insensitive duplicates (keeping the first
/// spelling), and keeps at most `MAX_TAGS`.
pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag: String = tag
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(MAX_TAG_LEN)
            .collect();
        let tag = tag.trim_end().to_owned();
        if !tag.is_empty() && !out.iter().any(|t| t.to_lowercase() == tag.to_lowercase()) {
            out.push(tag);
        }
        if out.len() == MAX_TAGS {
            break;
        }
    }
    out
}

pub const MAX_FOLDER_NAME_LEN: usize = 64;

/// Trims and collapses whitespace and truncates to `MAX_FOLDER_NAME_LEN`
/// characters. May return an empty string.
pub fn normalize_folder_name(name: &str) -> String {
    let name: String = name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_FOLDER_NAME_LEN)
        .collect();
    name.trim_end().to_owned()
}

impl Entry {
    pub fn new(id: String, input: &EntryInput, now: u64) -> Self {
        let mut entry = Self {
            id,
            kind: EntryKind::Account,
            title: String::new(),
            username: String::new(),
            legacy_email: String::new(),
            password: String::new(),
            url: String::new(),
            notes: String::new(),
            custom_fields: Vec::new(),
            env: String::new(),
            favorite: false,
            tags: Vec::new(),
            folder_id: None,
            created_at: now,
            updated_at: now,
        };
        entry.apply(input, now);
        entry
    }

    /// Copies the editable fields. Fields that don't belong to the input's
    /// kind are cleared, so an env entry never carries a stray password.
    pub fn apply(&mut self, input: &EntryInput, now: u64) {
        self.kind = input.kind;
        self.title = input.title.clone();
        self.notes = input.notes.clone();
        match input.kind {
            EntryKind::Account => {
                self.username = input.username.clone();
                self.password = input.password.clone();
                self.url = input.url.clone();
                self.custom_fields = input.custom_fields.clone();
                self.env.zeroize();
            }
            EntryKind::Env => {
                self.username.zeroize();
                self.password.zeroize();
                self.url.zeroize();
                self.custom_fields.zeroize();
                self.env = input.env.clone();
            }
        }
        self.favorite = input.favorite;
        self.tags = normalize_tags(&input.tags);
        self.folder_id = input.folder_id.clone();
        self.updated_at = now;
    }

    /// Moves a legacy email into `username`, or into a custom field if
    /// `username` is already set.
    /// Also normalizes tags, since imported backups may contain unnormalized ones.
    pub fn migrate(&mut self) {
        self.tags = normalize_tags(&self.tags);
        let email = std::mem::take(&mut self.legacy_email);
        if email.is_empty() {
            return;
        }
        if self.username.is_empty() {
            self.username = email;
        } else if self.username != email {
            self.custom_fields.insert(
                0,
                CustomField {
                    name: "Email".into(),
                    value: email,
                    secret: false,
                },
            );
        }
    }

    /// True if both entries hold the same login (used to skip duplicates on import).
    pub fn same_login(&self, other: &Entry) -> bool {
        self.kind == other.kind
            && self.env == other.env
            && self.title == other.title
            && self.username == other.username
            && self.password == other.password
            && self.url == other.url
    }

    pub fn summary(&self) -> EntrySummary {
        EntrySummary {
            id: self.id.clone(),
            kind: self.kind,
            title: self.title.clone(),
            username: self.username.clone(),
            url: self.url.clone(),
            favorite: self.favorite,
            tags: self.tags.clone(),
            folder_id: self.folder_id.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normalize_tags_trims_dedupes_and_limits() {
        let input = strings(&["  Work ", "work", "", "   ", "two   words", "Bank"]);
        assert_eq!(
            normalize_tags(&input),
            strings(&["Work", "two words", "Bank"])
        );

        let long = "x".repeat(50);
        assert_eq!(normalize_tags(&[long])[0].chars().count(), MAX_TAG_LEN);

        let many: Vec<String> = (0..30).map(|i| format!("t{i}")).collect();
        assert_eq!(normalize_tags(&many).len(), MAX_TAGS);
    }

    #[test]
    fn normalize_folder_name_trims_and_limits() {
        assert_eq!(normalize_folder_name("  Work   stuff "), "Work stuff");
        assert_eq!(normalize_folder_name("   "), "");
        let long = "x".repeat(100);
        assert_eq!(
            normalize_folder_name(&long).chars().count(),
            MAX_FOLDER_NAME_LEN
        );
    }

    #[test]
    fn vault_without_folders_deserializes() {
        let json = r#"{"entries":[{"id":"a","title":"A","createdAt":0,"updatedAt":0}]}"#;
        let data: VaultData = serde_json::from_str(json).unwrap();
        assert!(data.folders.is_empty());
        assert_eq!(data.entries[0].folder_id, None);
    }

    #[test]
    fn missing_tags_deserialize_as_empty() {
        let json = r#"{"id":"a","title":"A","createdAt":0,"updatedAt":0}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn missing_kind_deserializes_as_account() {
        let json = r#"{"id":"a","title":"A","createdAt":0,"updatedAt":0}"#;
        let entry: Entry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.kind, EntryKind::Account);
        assert!(entry.env.is_empty());
    }

    #[test]
    fn env_entry_drops_account_fields() {
        let mut input = EntryInput::default();
        input.kind = EntryKind::Env;
        input.title = "API".into();
        input.password = "stray".into();
        input.env = "TOKEN=abc
"
        .into();
        let entry = Entry::new("id".into(), &input, 1);
        assert_eq!(entry.kind, EntryKind::Env);
        assert_eq!(
            entry.env,
            "TOKEN=abc
"
        );
        assert!(entry.password.is_empty());

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains(r#""kind":"env""#));
    }

    #[test]
    fn env_entries_with_different_values_are_not_duplicates() {
        let mut input = EntryInput::default();
        input.kind = EntryKind::Env;
        input.title = "API".into();
        input.env = "TOKEN=a".into();
        let a = Entry::new("a".into(), &input, 1);
        input.env = "TOKEN=b".into();
        let b = Entry::new("b".into(), &input, 1);
        assert!(!a.same_login(&b));
        assert!(a.same_login(&a.clone()));
    }
}
