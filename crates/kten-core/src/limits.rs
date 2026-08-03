use crate::{Error, Result};

pub const DEFAULT_LIST_LIMIT: u32 = 20;
pub const DEFAULT_SEARCH_LIMIT: u32 = 20;
pub const DEFAULT_COMMENTS_LIMIT: u32 = 10;
pub const HARD_MAX_LIMIT: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    List,
    Search,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub list: u32,
    pub search: u32,
    pub comments: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            list: DEFAULT_LIST_LIMIT,
            search: DEFAULT_SEARCH_LIMIT,
            comments: DEFAULT_COMMENTS_LIMIT,
        }
    }
}

impl Limits {
    pub fn default_for(kind: LimitKind) -> u32 {
        match kind {
            LimitKind::List => DEFAULT_LIST_LIMIT,
            LimitKind::Search => DEFAULT_SEARCH_LIMIT,
            LimitKind::Comments => DEFAULT_COMMENTS_LIMIT,
        }
    }

    pub fn validate(value: Option<u32>, kind: LimitKind) -> Result<u32> {
        let value = value.unwrap_or_else(|| Self::default_for(kind));
        if value > HARD_MAX_LIMIT {
            return Err(Error::LimitTooHigh {
                value,
                max: HARD_MAX_LIMIT,
            });
        }
        Ok(value)
    }
}
