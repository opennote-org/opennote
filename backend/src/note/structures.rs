use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    /// Subpages of this page
    pub children: Option<Vec<Page>>,
    /// Mark the states of this page. It can be deleted, pinned, archived, etc.
    pub flags: Flags,
    /// Actual data contained in this page
    pub payloads: Vec<Payload>,
}

/// Next: do we store dynamic data? like hashmap?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// For displaying on the UI. It is the title of the page.
    pub title: Option<String>,
    /// The position of the payload on a page. Same order number means they will show up on the same line
    pub order: usize,
    /// Can be Text, Image bytes. extensible for other modality
    pub content: Content,
    pub created_at: usize,
    pub last_modified: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub value: Vec<u8>,
    pub format: Format, // Text? Image?
    pub style: Style, // Content style presented in which style. For example, text can be P1 or so. but need to consider formats inside of a content, like **bold**.
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Flags {
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Format {
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Style {
    Title,
    Body,
    Code,
    Math,
    Image,
}
