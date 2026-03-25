use rig::Embed;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Structured extraction result for a secondhand item listing.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct ListingDetails {
    pub title: String,
    pub category: String,
    pub brand: String,
    pub condition_score: u8,
    pub suggested_price_cny: i64,
    pub defects: Vec<String>,
}

/// A document stored in the PostgreSQL vector database for semantic search.
/// Document must implement Embed so EmbeddingsBuilder can extract its content.
#[derive(Embed, Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub id: String,
    #[embed]
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listing_details_serialization() {
        let details = ListingDetails {
            title: "iPhone 13 Pro".to_string(),
            category: "electronics".to_string(),
            brand: "Apple".to_string(),
            condition_score: 8,
            suggested_price_cny: 5000,
            defects: vec!["Minor scratch on screen".to_string()],
        };
        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("iPhone 13 Pro"));
        assert!(json.contains("Apple"));
        assert!(json.contains("electronics"));
        assert!(json.contains("8"));
    }

    #[test]
    fn test_listing_details_deserialization() {
        let json = r#"{
            "title": "MacBook Air",
            "category": "electronics",
            "brand": "Apple",
            "condition_score": 9,
            "suggested_price_cny": 8000,
            "defects": []
        }"#;
        let details: ListingDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.title, "MacBook Air");
        assert_eq!(details.brand, "Apple");
        assert_eq!(details.condition_score, 9);
        assert!(details.defects.is_empty());
    }

    #[test]
    fn test_document_serialization() {
        let doc = Document {
            id: "doc-123".to_string(),
            content: "Test content for embedding".to_string(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("Test content"));
    }

    #[test]
    fn test_document_deserialization() {
        let json = r#"{"id": "doc-456", "content": "Item description"}"#;
        let doc: Document = serde_json::from_str(json).unwrap();
        assert_eq!(doc.id, "doc-456");
        assert_eq!(doc.content, "Item description");
    }

    #[test]
    fn test_listing_details_with_defects() {
        let details = ListingDetails {
            title: "Test Item".to_string(),
            category: "electronics".to_string(),
            brand: "Brand".to_string(),
            condition_score: 5,
            suggested_price_cny: 1000,
            defects: vec!["Scratched".to_string(), "Missing part".to_string()],
        };
        assert_eq!(details.defects.len(), 2);
        assert!(details.defects.contains(&"Scratched".to_string()));
    }

    #[test]
    fn test_condition_score_bounds() {
        // condition_score is u8, but validated to 1-10 in the application layer
        let details = ListingDetails {
            title: "Test".to_string(),
            category: "other".to_string(),
            brand: "Brand".to_string(),
            condition_score: 1, // minimum valid
            suggested_price_cny: 100,
            defects: vec![],
        };
        assert_eq!(details.condition_score, 1);
    }
}
