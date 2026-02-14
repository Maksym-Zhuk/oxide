pub mod variables;

use std::fmt;

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum Framework {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
  Lit,
  Qwik,
  Angular,
  Nest,
  Next,
  Nuxt,
  Electron,
  Tauri,
}

impl Framework {
  pub fn needs_build_tool(&self) -> bool {
    match self {
      Framework::React => true,
      Framework::Vue => true,
      Framework::Svelte => true,
      Framework::Solid => true,
      Framework::Preact => true,
      Framework::Lit => true,
      Framework::Electron => true,
      Framework::Tauri => true,

      Framework::Qwik => false,
      Framework::Angular => false,
      Framework::Next => false,
      Framework::Nuxt => false,
      Framework::Nest => false,
    }
  }

  pub fn needs_choose_paltform(&self, build_tool: &Option<BuildTool>) -> bool {
    matches!(
      (self, build_tool),
      (Framework::React, Some(BuildTool::Vite))
        | (Framework::Angular, _)
        | (Framework::Electron, _)
        | (Framework::Tauri, _)
        | (Framework::Nest, _)
    )
  }

  pub fn needs_choose_language(&self) -> bool {
    match self {
      Framework::React => true,
      Framework::Lit => true,
      Framework::Qwik => true,
      Framework::Vue => true,
      Framework::Preact => true,
      Framework::Svelte => true,
      Framework::Solid => true,

      Framework::Angular => false,
      Framework::Nest => false,
      Framework::Tauri => false,
      Framework::Electron => false,
      Framework::Next => false,
      Framework::Nuxt => false,
    }
  }

  pub fn compatible_build_tools(&self) -> Vec<BuildTool> {
    match self {
      Framework::React
      | Framework::Preact
      | Framework::Vue
      | Framework::Svelte
      | Framework::Solid
      | Framework::Lit => vec![BuildTool::Vite, BuildTool::Farm, BuildTool::Rsbuild],

      Framework::Electron | Framework::Tauri => vec![BuildTool::Vite, BuildTool::Farm],
      Framework::Qwik => vec![BuildTool::Vite],

      _ => vec![],
    }
  }

  pub fn compatible_platform(&self, build_tool: &Option<BuildTool>) -> Vec<Platform> {
    match (self, build_tool) {
      (Framework::React, Some(BuildTool::Vite)) => vec![
        Platform::React(ReactVariant::Default),
        Platform::React(ReactVariant::Swc),
        Platform::React(ReactVariant::Compiler),
      ],
      (Framework::Angular, _) => vec![
        Platform::Angular(AngularVariant::Analog),
        Platform::Angular(AngularVariant::Angular),
      ],
      (Framework::Electron, Some(BuildTool::Vite)) => vec![
        Platform::ElectronVite(ElectronVitePlatform::React),
        Platform::ElectronVite(ElectronVitePlatform::Vue),
      ],
      (Framework::Electron, Some(BuildTool::Farm)) => vec![
        Platform::ElectronFarm(ElectronFarmPlatform::React),
        Platform::ElectronFarm(ElectronFarmPlatform::Preact),
        Platform::ElectronFarm(ElectronFarmPlatform::Vue),
        Platform::ElectronFarm(ElectronFarmPlatform::Solid),
        Platform::ElectronFarm(ElectronFarmPlatform::Svelte),
      ],
      (Framework::Tauri, _) => vec![
        Platform::Tauri(TauriPlatform::Preact),
        Platform::Tauri(TauriPlatform::React),
        Platform::Tauri(TauriPlatform::Vue),
        Platform::Tauri(TauriPlatform::Svelte),
        Platform::Tauri(TauriPlatform::Solid),
      ],
      (Framework::Nest, _) => vec![
        Platform::Nest(NestPlatform::Express),
        Platform::Nest(NestPlatform::Fastify),
      ],
      _ => vec![],
    }
  }
}

impl fmt::Display for Framework {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Framework::React => "react",
      Framework::Preact => "preact",
      Framework::Vue => "vue",
      Framework::Svelte => "svelte",
      Framework::Solid => "solid",
      Framework::Lit => "lit",
      Framework::Qwik => "qwik",
      Framework::Angular => "angular",
      Framework::Nest => "nest",
      Framework::Next => "next",
      Framework::Nuxt => "nuxt",
      Framework::Electron => "electron",
      Framework::Tauri => "tauri",
    };

    write!(f, "{value}")
  }
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

#[derive(Debug, Clone, Copy)]
pub enum ReactVariant {
  Default,
  Swc,
  Compiler,
}

impl fmt::Display for ReactVariant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ReactVariant::Default => "default",
      ReactVariant::Swc => "swc",
      ReactVariant::Compiler => "conpilator",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum AngularVariant {
  Angular,
  Analog,
}

impl fmt::Display for AngularVariant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      AngularVariant::Angular => "angular",
      AngularVariant::Analog => "analog",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronRuntime {
  Vite,
  Farm,
}

impl fmt::Display for ElectronRuntime {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronRuntime::Vite => "vite",
      ElectronRuntime::Farm => "farm",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronVitePlatform {
  React,
  Vue,
}

impl fmt::Display for ElectronVitePlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronVitePlatform::React => "react",
      ElectronVitePlatform::Vue => "vue",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum ElectronFarmPlatform {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
}

impl fmt::Display for ElectronFarmPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      ElectronFarmPlatform::React => "react",
      ElectronFarmPlatform::Preact => "preact",
      ElectronFarmPlatform::Vue => "vue",
      ElectronFarmPlatform::Svelte => "svelte",
      ElectronFarmPlatform::Solid => "solid",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum TauriPlatform {
  React,
  Preact,
  Vue,
  Svelte,
  Solid,
}

impl fmt::Display for TauriPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      TauriPlatform::React => "react",
      TauriPlatform::Preact => "preact",
      TauriPlatform::Vue => "vue",
      TauriPlatform::Svelte => "svelte",
      TauriPlatform::Solid => "solid",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy)]
pub enum NestPlatform {
  Express,
  Fastify,
}

impl fmt::Display for NestPlatform {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      NestPlatform::Express => "express",
      NestPlatform::Fastify => "fastify",
    };
    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
  TypeScript,
  JavaScript,
}

impl fmt::Display for Language {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      Language::JavaScript => "js",
      Language::TypeScript => "ts",
    };

    write!(f, "{value}")
  }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildTool {
  Vite,
  Farm,
  Rsbuild,
}

impl fmt::Display for BuildTool {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = match self {
      BuildTool::Vite => "vite",
      BuildTool::Farm => "farm",
      BuildTool::Rsbuild => "rsbuild",
    };

    write!(f, "{value}")
  }
}
