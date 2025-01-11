//! Llama3 function

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The function description
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Function {
    /// The name of the function
    pub name: String,
    /// The description of the function
    pub description: String,
    /// The parameters of the function
    pub parameters: Parameters,
}

/// The parameter of the function
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Parameters {
    /// The name of the parameter
    #[serde(rename = "type")]
    pub ty: String,
    /// The required of the parameter
    pub required: Vec<String>,
    /// The properties of the parameter
    pub properties: HashMap<String, Property>,
}

/// The property of the parameter
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Property {
    /// The type of the property
    #[serde(rename = "type")]
    pub ty: String,
    /// The description of the property
    pub description: String,
    /// The default value of the property
    pub default: Option<String>,
}

#[test]
fn test_serde() {
    const JSON: &str = r#"{
    "name": "get_user_info",
    "description": "Retrieve details for a specific user by their unique identifier. Note that the provided function is in Python 3 syntax.",
    "parameters": {
        "type": "dict",
        "required": [
            "user_id"
        ],
        "properties": {
            "user_id": {
                "type": "integer",
                "description": "The unique identifier of the user. It is used to fetch the specific user details from the database."
            },
            "special": {
                "type": "string",
                "description": "Any special information or parameters that need to be considered while fetching user details.",
                "default": "none"
            }
        }
    }
}"#;

    let mut expected = Function {
        name: "get_user_info".to_string(),
        description: "Retrieve details for a specific user by their unique identifier. Note that the provided function is in Python 3 syntax.".to_string(),
        parameters: Parameters {
            ty: "dict".to_string(),
            required: vec!["user_id".to_string()],
            properties: HashMap::new(),
        },
    };

    expected.parameters.properties.insert(
        "user_id".to_string(),
        Property {
            ty: "integer".to_string(),
            description: "The unique identifier of the user. It is used to fetch the specific user details from the database.".to_string(),
            default: None,
        },
    );

    expected.parameters.properties.insert(
        "special".to_string(),
        Property {
            ty: "string".to_string(),
            description: "Any special information or parameters that need to be considered while fetching user details.".to_string(),
            default: Some("none".to_string()),
        },
    );

    let actual: Function = serde_json::from_str(JSON).unwrap();
    assert_eq!(expected, actual);
}
