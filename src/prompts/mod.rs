pub mod variables;

use std::fmt;

use clap::ValueEnum;

pub trait FrameworkConfig {
  fn needs_build_tool(&self) -> bool;
  fn needs_choose_language(&self) -> bool;
  fn needs_choose_paltform(&self, build_tool: &Option<BuildTool>) -> bool;
  fn is_tauri(&self) -> bool {
    false
  }
  fn compatible_build_tools(&self) -> Vec<BuildTool> {
    vec![]
  }
  fn compatible_platforms(&self, _build_tool: &Option<BuildTool>) -> Vec<Platform> {
    vec![]
  }
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum ProjectLayer {
  Frontend,
  Meta,
  Backend,
  Desktop,
  Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum FrontendTool {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
  Lit,
  Qwik,
  Angular,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum BackendTool {
  Nest,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum MetaFramework {
  Next,
  Nuxt,
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum DesktopRuntime {
  Tauri,
  Electron,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileTool {
  ReactNative,
}

#[derive(Debug, Clone, Copy)]
pub enum Platform {
  React(ReactVariant),
  Angular(AngularVariant),
  ElectronVite(ElectronVitePlatform),
  ElectronFarm(ElectronFarmPlatform),
  Tauri(TauriPlatform),
  Nest(NestPlatform),
}

#[derive(Debug, Clone, Copy)]
pub enum ReactVariant {
  Default,
  Swc,
  Compiler,
}

#[derive(Debug, Clone, Copy)]
pub enum AngularVariant {
  Angular,
  Analog,
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronRuntime {
  Vite,
  Farm,
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronVitePlatform {
  React,
  Vue,
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronFarmPlatform {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
}

#[derive(Debug, Clone, Copy)]
pub enum TauriPlatform {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
}

#[derive(Debug, Clone, Copy)]
pub enum NestPlatform {
  Express,
  Fastify,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
  TypeScript,
  JavaScript,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildTool {
  Vite,
  Farm,
  Rsbuild,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageManager {
  NPM,
  Yarn,
  PNPM,
  Bun,
}

impl FrameworkConfig for FrontendTool {
  fn needs_build_tool(&self) -> bool {
    match self {
      FrontendTool::React => true,
      FrontendTool::Vue => true,
      FrontendTool::Svelte => true,
      FrontendTool::Solid => true,
      FrontendTool::Preact => true,
      FrontendTool::Lit => true,

      FrontendTool::Qwik => false,
      FrontendTool::Angular => false,
    }
  }

  fn needs_choose_language(&self) -> bool {
    match self {
      FrontendTool::React => true,
      FrontendTool::Lit => true,
      FrontendTool::Qwik => true,
      FrontendTool::Vue => true,
      FrontendTool::Preact => true,
      FrontendTool::Svelte => true,
      FrontendTool::Solid => true,

      FrontendTool::Angular => false,
    }
  }

  fn needs_choose_paltform(&self, build_tool: &Option<BuildTool>) -> bool {
    matches!(
      (self, build_tool),
      (FrontendTool::React, Some(BuildTool::Vite)) | (FrontendTool::Angular, _)
    )
  }

  fn compatible_build_tools(&self) -> Vec<BuildTool> {
    match self {
      FrontendTool::React
      | FrontendTool::Preact
      | FrontendTool::Vue
      | FrontendTool::Svelte
      | FrontendTool::Solid
      | FrontendTool::Lit => vec![BuildTool::Vite, BuildTool::Farm, BuildTool::Rsbuild],

      FrontendTool::Qwik => vec![BuildTool::Vite],

      _ => vec![],
    }
  }

  fn compatible_platforms(&self, build_tool: &Option<BuildTool>) -> Vec<Platform> {
    match (self, build_tool) {
      (FrontendTool::React, Some(BuildTool::Vite)) => vec![
        Platform::React(ReactVariant::Default),
        Platform::React(ReactVariant::Swc),
        Platform::React(ReactVariant::Compiler),
      ],
      (FrontendTool::Angular, _) => vec![
        Platform::Angular(AngularVariant::Analog),
        Platform::Angular(AngularVariant::Angular),
      ],
      _ => vec![],
    }
  }
}

impl FrameworkConfig for MetaFramework {
  fn needs_build_tool(&self) -> bool {
    match self {
      MetaFramework::Next => false,
      MetaFramework::Nuxt => false,
    }
  }

  fn needs_choose_language(&self) -> bool {
    match self {
      MetaFramework::Next => false,
      MetaFramework::Nuxt => false,
    }
  }

  fn needs_choose_paltform(&self, _build_tool: &Option<BuildTool>) -> bool {
    false
  }
}

impl FrameworkConfig for BackendTool {
  fn needs_build_tool(&self) -> bool {
    match self {
      BackendTool::Nest => false,
    }
  }

  fn needs_choose_paltform(&self, build_tool: &Option<BuildTool>) -> bool {
    matches!(
      (self, build_tool),
        | (BackendTool::Nest, _)
    )
  }

  fn needs_choose_language(&self) -> bool {
    match self {
      BackendTool::Nest => false,
    }
  }

  fn compatible_platforms(&self, build_tool: &Option<BuildTool>) -> Vec<Platform> {
    match (self, build_tool) {
      (BackendTool::Nest, _) => vec![
        Platform::Nest(NestPlatform::Express),
        Platform::Nest(NestPlatform::Fastify),
      ],
    }
  }
}

impl FrameworkConfig for DesktopRuntime {
  fn needs_build_tool(&self) -> bool {
    match self {
      DesktopRuntime::Electron => true,
      DesktopRuntime::Tauri => true,
    }
  }

  fn needs_choose_paltform(&self, build_tool: &Option<BuildTool>) -> bool {
    matches!((self, build_tool), |(DesktopRuntime::Electron, _)| (
      DesktopRuntime::Tauri,
      _
    ))
  }

  fn needs_choose_language(&self) -> bool {
    match self {
      DesktopRuntime::Tauri => false,
      DesktopRuntime::Electron => false,
    }
  }

  fn is_tauri(&self) -> bool {
    matches!(self, Self::Tauri)
  }

  fn compatible_build_tools(&self) -> Vec<BuildTool> {
    match self {
      DesktopRuntime::Electron | DesktopRuntime::Tauri => vec![BuildTool::Vite, BuildTool::Farm],
    }
  }

  fn compatible_platforms(&self, build_tool: &Option<BuildTool>) -> Vec<Platform> {
    match (self, build_tool) {
      (DesktopRuntime::Electron, Some(BuildTool::Vite)) => vec![
        Platform::ElectronVite(ElectronVitePlatform::React),
        Platform::ElectronVite(ElectronVitePlatform::Vue),
      ],
      (DesktopRuntime::Electron, Some(BuildTool::Farm)) => vec![
        Platform::ElectronFarm(ElectronFarmPlatform::React),
        Platform::ElectronFarm(ElectronFarmPlatform::Preact),
        Platform::ElectronFarm(ElectronFarmPlatform::Vue),
        Platform::ElectronFarm(ElectronFarmPlatform::Solid),
        Platform::ElectronFarm(ElectronFarmPlatform::Svelte),
      ],
      (DesktopRuntime::Tauri, _) => vec![
        Platform::Tauri(TauriPlatform::Preact),
        Platform::Tauri(TauriPlatform::React),
        Platform::Tauri(TauriPlatform::Vue),
        Platform::Tauri(TauriPlatform::Svelte),
        Platform::Tauri(TauriPlatform::Solid),
      ],

      _ => vec![],
    }
  }
}

impl FrameworkConfig for MobileTool {
  fn needs_build_tool(&self) -> bool {
    false
  }
  fn needs_choose_language(&self) -> bool {
    false
  }

  fn needs_choose_paltform(&self, _build_tool: &Option<BuildTool>) -> bool {
    false
  }
}

impl fmt::Display for ProjectLayer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ProjectLayer::Frontend => "Frontend",
      ProjectLayer::Meta => "Meta",
      ProjectLayer::Desktop => "Desktop",
      ProjectLayer::Backend => "Backend",
      ProjectLayer::Mobile => "Mobile",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for FrontendTool {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      FrontendTool::React => "React",
      FrontendTool::Preact => "Preact",
      FrontendTool::Vue => "Vue",
      FrontendTool::Svelte => "Svelte",
      FrontendTool::Solid => "Solid",
      FrontendTool::Lit => "Lit",
      FrontendTool::Qwik => "Qwik",
      FrontendTool::Angular => "Angular",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for BackendTool {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      BackendTool::Nest => "Nest",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for MetaFramework {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      MetaFramework::Next => "Next",
      MetaFramework::Nuxt => "Nuxt",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for DesktopRuntime {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      DesktopRuntime::Tauri => "Tauri",
      DesktopRuntime::Electron => "Electron",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for MobileTool {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      MobileTool::ReactNative => "React Native",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for Platform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Platform::React(variant) => write!(f, "{variant}"),
      Platform::Angular(variant) => write!(f, "{variant}"),
      Platform::ElectronVite(variant) => write!(f, "{variant}"),
      Platform::ElectronFarm(variant) => write!(f, "{variant}"),
      Platform::Tauri(variant) => write!(f, "{variant}"),
      Platform::Nest(variant) => write!(f, "{variant}"),
    }
  }
}

impl fmt::Display for ReactVariant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ReactVariant::Default => "Default",
      ReactVariant::Swc => "Swc",
      ReactVariant::Compiler => "Compiler",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for AngularVariant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      AngularVariant::Angular => "Angular",
      AngularVariant::Analog => "Analog",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for ElectronRuntime {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronRuntime::Vite => "Vite",
      ElectronRuntime::Farm => "Farm",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for ElectronVitePlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronVitePlatform::React => "React",
      ElectronVitePlatform::Vue => "Vue",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for ElectronFarmPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronFarmPlatform::React => "React",
      ElectronFarmPlatform::Preact => "Preact",
      ElectronFarmPlatform::Vue => "Vue",
      ElectronFarmPlatform::Svelte => "Svelte",
      ElectronFarmPlatform::Solid => "Solid",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for TauriPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      TauriPlatform::React => "React",
      TauriPlatform::Preact => "Preact",
      TauriPlatform::Vue => "Vue",
      TauriPlatform::Svelte => "Svelte",
      TauriPlatform::Solid => "Solid",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for NestPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      NestPlatform::Express => "Express",
      NestPlatform::Fastify => "Fastify",
    };
    write!(f, "{value}")
  }
}

impl fmt::Display for Language {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Language::JavaScript => "JavaScript",
      Language::TypeScript => "TypeScript",
    };

    write!(f, "{value}")
  }
}

impl fmt::Display for BuildTool {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      BuildTool::Vite => "Vite",
      BuildTool::Farm => "Farm",
      BuildTool::Rsbuild => "Rsbuild",
    };

    write!(f, "{value}")
  }
}

impl fmt::Display for PackageManager {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      PackageManager::NPM => "npm",
      PackageManager::Yarn => "yarn",
      PackageManager::PNPM => "pnpm",
      PackageManager::Bun => "bun",
    };
    write!(f, "{value}")
  }
}
