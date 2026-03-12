use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchResult {
    pub id: u32,
    pub title: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let result = SearchResult {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
        };
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(result, deserialized);
    }
}
