use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Everything stored inside the encrypted vault file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
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
    #[serde(default)]
    pub favorite: bool,
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
    pub favorite: bool,
}

/// List view of an entry, without the password or secret fields.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub title: String,
    pub username: String,
    pub url: String,
    pub favorite: bool,
    pub updated_at: u64,
}

impl Entry {
    pub fn new(id: String, input: &EntryInput, now: u64) -> Self {
        let mut entry = Self {
            id,
            title: String::new(),
            username: String::new(),
            legacy_email: String::new(),
            password: String::new(),
            url: String::new(),
            notes: String::new(),
            custom_fields: Vec::new(),
            favorite: false,
            created_at: now,
            updated_at: now,
        };
        entry.apply(input, now);
        entry
    }

    pub fn apply(&mut self, input: &EntryInput, now: u64) {
        self.title = input.title.clone();
        self.username = input.username.clone();
        self.password = input.password.clone();
        self.url = input.url.clone();
        self.notes = input.notes.clone();
        self.custom_fields = input.custom_fields.clone();
        self.favorite = input.favorite;
        self.updated_at = now;
    }

    /// Moves a legacy email into `username`, or into a custom field if
    /// `username` is already set.
    pub fn migrate(&mut self) {
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
        self.title == other.title
            && self.username == other.username
            && self.password == other.password
            && self.url == other.url
    }

    pub fn summary(&self) -> EntrySummary {
        EntrySummary {
            id: self.id.clone(),
            title: self.title.clone(),
            username: self.username.clone(),
            url: self.url.clone(),
            favorite: self.favorite,
            updated_at: self.updated_at,
        }
    }
}
