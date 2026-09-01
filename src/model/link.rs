use super::note::NoteId;

pub type LinkId = i64;

#[derive(Debug, Clone)]
pub struct Link {
    pub id: LinkId,
    pub note_id: NoteId,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct NewLink {
    pub note_id: NoteId,
    pub label: String,
    pub url: String,
}
