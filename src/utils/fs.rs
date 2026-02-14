use crate::prompts::{BuildTool, Framework, Language, Platform};

pub fn generate_path(
    language: &Option<Language>,
    build_tool: &Option<BuildTool>,
    framework: &Framework,
    platform: &Option<Platform>,
) -> String {
    let mut path_parts = Vec::new();

    if let Some(lg) = language {
        path_parts.push(lg.to_string());
    };

    if let Some(bt) = build_tool {
        path_parts.push(bt.to_string());
    };

    path_parts.push(framework.to_string());

    if let Some(pl) = platform {
        path_parts.push(pl.to_string());
    };

    path_parts.join("/")
}
