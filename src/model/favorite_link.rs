pub type FavoriteLinkId = i64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteLink {
    pub id: FavoriteLinkId,
    pub label: String,
    pub url: String,
    pub description: Option<String>,
    pub tags: String,
}

#[derive(Debug, Clone)]
pub struct NewFavoriteLink {
    pub label: String,
    pub url: String,
    pub description: Option<String>,
    pub tags: String,
}
