use std::collections::HashMap;

/// Generic where clause
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum GenericClause {
  /// Equal
  Eq(String),
  /// Not equal
  Ne(String),
  /// Greater than
  Gt(String),
  /// Less than
  Lt(String),
  /// Greater than or equal
  Ge(String),
  /// Less than or equal
  Le(String),
  /// Like
  Like(String),
  /// Not like
  NotLike(String),
  /// In
  In(Vec<String>),
  /// Not in
  NotIn(Vec<String>),
  /// Is null
  IsNull,
  /// Is not null
  IsNotNull,
  /// JSON contains
  Contains(serde_json::Value),
  /// JSON Has key
  HasKey(String),
}

/// Generic filter for list operation
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GenericFilter {
  /// Where clause
  #[cfg_attr(feature = "serde", serde(rename = "where"))]
  pub r#where: Option<HashMap<String, GenericClause>>,
  /// Limit number of items default (100)
  pub limit: Option<usize>,
  /// Offset to navigate through items
  pub offset: Option<usize>,
}

impl GenericFilter {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn limit(mut self, limit: usize) -> Self {
    self.limit = Some(limit);
    self
  }

  pub fn offset(mut self, offset: usize) -> Self {
    self.offset = Some(offset);
    self
  }

  pub fn r#where(mut self, key: &str, clause: GenericClause) -> Self {
    if self.r#where.is_none() {
      self.r#where = Some(HashMap::new());
    }
    self
      .r#where
      .as_mut()
      .unwrap()
      .insert(key.to_owned(), clause);
    self
  }
}
