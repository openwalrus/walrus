use cydonia_model::{config::Config, manifest::Manifest, model::Llama, Model};

const PROMPT: &str = r#"
[INST] 
<<SYS>>
You are a helpful, respectful and honest assistant.
<</SYS>>

Hello, world!
[/INST]
"#;

fn main() -> anyhow::Result<()> {
    let api = hf_hub::api::sync::Api::new()?;
    let mut model = Llama::build(api, Config::default(), Manifest::default())?;
    println!("model built");
    let _response = model.complete(&mut vec![PROMPT.into()])?;
    Ok(())
}
