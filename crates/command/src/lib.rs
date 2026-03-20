pub use crabtalk_command_codegen::command;
pub use wcore::service::{Service, install, render_service_template, uninstall, view_logs};

#[cfg(feature = "mcp")]
pub use wcore::service::{McpService, run_mcp};

#[cfg(feature = "client")]
pub use wcore::service::ClientService;
