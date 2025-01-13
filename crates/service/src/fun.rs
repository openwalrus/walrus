//! Llama3 function

use serde::{Deserialize, Serialize};

/// The function description
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct Function {
    /// The name of the function
    pub name: String,
    /// The description of the function
    pub description: String,
    /// The arguments of the function
    pub arguments: Vec<Argument>,
}

/// The argument of the function
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct Argument {
    /// The name of the argument
    pub name: String,
    /// The description of the argument
    pub description: String,
    /// The type of the argument
    #[serde(rename = "type")]
    pub ty: String,
}

#[test]
fn test_serde() {
    const JSON: &str = r#"{
    "name": "my_function",
    "description": "My function",
    "arguments": [
        {
            "name": "a",
            "description": "The first number",
            "type": "uint64"
        },
        {
            "name": "b",
            "description": "The second number",
            "type": "uint64"
        }
    ]
}"#;

    let expected = Function {
        name: "my_function".to_string(),
        description: "My function".to_string(),
        arguments: vec![
            Argument {
                name: "a".to_string(),
                description: "The first number".to_string(),
                ty: "uint64".to_string(),
            },
            Argument {
                name: "b".to_string(),
                description: "The second number".to_string(),
                ty: "uint64".to_string(),
            },
        ],
    };

    let actual: Function = serde_json::from_str(JSON).unwrap();
    assert_eq!(expected, actual);
}
