use crate::prompts::{BuildTool, Framework, Language, Platform};

pub fn generate_path(
  language: &Option<Language>,
  build_tool: &Option<BuildTool>,
  framework: &Framework,
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

  if framework == &Framework::Qwik {
    path_parts.push(BuildTool::Vite.to_string().to_lowercase());
  }

  path_parts.push(framework.to_string().to_lowercase());

  if let Some(pl) = platform {
    path_parts.push(pl.to_string().to_lowercase());
  };

  path_parts.join("/")
}
