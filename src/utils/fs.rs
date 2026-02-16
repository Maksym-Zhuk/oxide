use crate::prompts::{BuildTool, Language, Platform};

pub fn generate_path(
  language: &Option<Language>,
  build_tool: &Option<BuildTool>,
  framework_name: &str,
  platform: &Option<Platform>,
) -> String {
  let mut path_parts = Vec::new();

  if let Some(lg) = language {
    if lg == &Language::JavaScript {
      path_parts.push("js".to_string());
    } else if lg == &Language::TypeScript {
      path_parts.push("ts".to_string());
    }
  };

  if let Some(bt) = build_tool {
    path_parts.push(bt.to_string().to_lowercase());
  };

  if framework_name == "Qwik" {
    path_parts.push("vite".to_string());
  }

  path_parts.push(framework_name.to_string().to_lowercase());

  if let Some(pl) = platform {
    path_parts.push(pl.to_string().to_lowercase());
  };

  path_parts.join("/")
}
