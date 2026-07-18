use clap::Subcommand;

use crate::completions::CompletionShell;

#[derive(Subcommand)]
pub enum AddonCommands {
  #[command(alias = "i", about = "Install and cache an addon (anesis addon i)")]
  Install {
    #[arg(help = "Addon id to install (omit to pick interactively)")]
    addon_id: Option<String>,
  },

  #[command(about = "Validate a local directory and cache it as an addon for local testing")]
  Link {
    #[arg(help = "Path to the addon directory (defaults to the current directory)")]
    path: Option<String>,

    #[arg(
      short,
      long,
      help = "Overwrite an existing cached addon without asking"
    )]
    force: bool,
  },

  #[command(alias = "l", about = "List installed addons (anesis addon l)")]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(
    about = "Show an addon's manifest: description, version, variants, commands, inputs and steps"
  )]
  Info {
    #[arg(help = "Addon id to inspect")]
    addon_id: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Dry-run an addon command on a throwaway copy of a project and show the diff")]
  Test {
    #[arg(help = "Addon id to test")]
    addon_id: String,

    #[arg(help = "Command to run")]
    command: String,

    #[arg(
      long,
      value_name = "PATH",
      help = "Fixture project to test on (defaults to the addon's bundled test-fixture/)"
    )]
    project: Option<String>,
  },

  #[command(alias = "r", about = "Remove a cached addon (anesis addon r)")]
  Remove { addon_id: String },

  #[command(
    alias = "p",
    about = "Publish a GitHub repository as an Anesis addon (anesis addon p)"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    alias = "u",
    about = "Update a GitHub repository as an Anesis addon (anesis addon u)"
  )]
  Update {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    addon_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum TemplateCommands {
  #[command(
    alias = "i",
    about = "Download and cache a template locally (anesis template i)"
  )]
  Install {
    #[arg(help = "Name of the template to install (omit to pick interactively)")]
    template_name: Option<String>,
  },

  #[command(about = "Validate a local directory and cache it as a template for local testing")]
  Link {
    #[arg(help = "Path to the template directory (defaults to the current directory)")]
    path: Option<String>,

    #[arg(
      short,
      long,
      help = "Overwrite an existing cached template without asking"
    )]
    force: bool,
  },

  #[command(
    alias = "l",
    about = "List all locally installed templates (anesis template l)"
  )]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show a template's manifest: description, version and repository")]
  Info {
    #[arg(help = "Name of the template to inspect")]
    template_name: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(
    alias = "r",
    about = "Remove an installed template from the local cache (anesis template r)"
  )]
  Remove {
    #[arg(help = "Name of the template to remove")]
    template_name: String,
  },

  #[command(
    alias = "p",
    about = "Publish a GitHub repository as an Anesis template (anesis template p)"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    alias = "u",
    about = "Update a GitHub repository as an Anesis template (anesis template u)"
  )]
  Update {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    template_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private, org-private"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum StackCommands {
  #[command(
    alias = "i",
    about = "Download and cache a stack from the registry (anesis stack i)"
  )]
  Install {
    #[arg(help = "Stack id to install")]
    stack_id: String,
  },

  #[command(alias = "l", about = "List locally installed stacks (anesis stack l)")]
  List {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show a stack's composition: template and addons")]
  Info {
    #[arg(help = "Stack id to inspect")]
    stack_id: String,

    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(alias = "r", about = "Remove a locally cached stack (anesis stack r)")]
  Remove {
    #[arg(help = "Stack id to remove")]
    stack_id: String,
  },

  #[command(
    alias = "p",
    about = "Publish a GitHub repository as an Anesis stack (anesis stack p)"
  )]
  Publish {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    stack_url: String,

    #[arg(
      long,
      value_name = "VISIBILITY",
      help = "Visibility: public, private (default: public)"
    )]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID to publish under")]
    org_id: Option<String>,
  },

  #[command(
    alias = "u",
    about = "Republish a stack from its GitHub repository (anesis stack u)"
  )]
  Update {
    #[arg(help = "GitHub repository URL (e.g. https://github.com/owner/repo)")]
    stack_url: String,

    #[arg(long, value_name = "VISIBILITY", help = "Visibility: public, private")]
    visibility: Option<String>,

    #[arg(
      long,
      value_name = "UUID",
      help = "Credential ID for private GitHub repositories"
    )]
    credential_id: Option<String>,

    #[arg(long, value_name = "UUID", help = "Organization ID")]
    org_id: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum Commands {
  #[command(alias = "n", about = "Create a new project from a template (anesis n)")]
  New {
    #[arg(help = "Name of the project directory to create")]
    name: String,

    #[arg(help = "Name of the template to use (omit to pick interactively)")]
    template_name: Option<String>,

    #[arg(
      long,
      value_name = "PATH",
      help = "Scaffold from a stack manifest (anesis.stack.json): template + ordered addons"
    )]
    stack: Option<String>,

    #[arg(short, long, help = "Pick only from already-downloaded templates")]
    installed: bool,

    #[arg(short, long, help = "Accept all defaults, skip confirmation prompts")]
    yes: bool,

    #[arg(
      long,
      value_name = "NAME=VALUE",
      help = "Provide an input value non-interactively (repeatable)"
    )]
    input: Vec<String>,
  },

  #[command(alias = "t", about = "Manage templates (anesis t)")]
  Template {
    #[command(subcommand)]
    command: TemplateCommands,
  },

  #[command(alias = "in", about = "Log in to your Anesis account (anesis in)")]
  Login,

  #[command(alias = "out", about = "Log out of your Anesis account (anesis out)")]
  Logout,

  #[command(about = "Show information about the currently logged-in account")]
  Account {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(alias = "a", about = "Manage addons (anesis a)")]
  Addon {
    #[command(subcommand)]
    command: AddonCommands,
  },

  #[command(
    alias = "s",
    about = "Manage stacks: template + ordered addons (anesis s)"
  )]
  Stack {
    #[command(subcommand)]
    command: StackCommands,
  },

  #[command(
    about = "Run an addon command in the current project (anesis use [ADDON_ID] [COMMAND])"
  )]
  Use {
    #[arg(help = "Addon id (omit to pick interactively)")]
    addon_id: Option<String>,

    #[arg(help = "Command to run (omit to list the addon's commands)")]
    command: Option<String>,

    #[arg(short, long, help = "Pick only from already-downloaded addons")]
    installed: bool,

    #[arg(short, long, help = "Accept all defaults, skip confirmation prompts")]
    yes: bool,

    #[arg(
      long,
      value_name = "NAME=VALUE",
      help = "Provide an input value non-interactively (repeatable)"
    )]
    input: Vec<String>,

    #[arg(
      long,
      help = "Show the plan (variant, inputs, steps) without changing any files"
    )]
    dry_run: bool,
  },

  #[command(about = "Revert an applied addon's changes in the current project")]
  Undo {
    #[arg(help = "Addon id to revert")]
    addon_id: String,

    #[arg(short = 'y', long, help = "Skip the confirmation prompt")]
    yes: bool,
  },

  #[command(about = "List applied addons that have a newer version in the registry")]
  Outdated,

  #[command(about = "Upgrade an applied addon to the registry's latest version")]
  Update {
    #[arg(help = "Addon id to update")]
    addon_id: String,

    #[arg(
      short = 'y',
      long,
      help = "Accept all defaults, skip confirmation prompts"
    )]
    yes: bool,
  },

  #[command(about = "Download and install the latest Anesis release")]
  Upgrade,

  #[command(
    about = "Run an MCP (Model Context Protocol) stdio server so AI agents can drive Anesis"
  )]
  Mcp,

  #[command(about = "Install shell tab completion for anesis")]
  Completions {
    #[arg(value_enum, help = "Shell to install completions for")]
    shell: CompletionShell,
  },

  #[command(about = "Interactively search the registry for templates and addons")]
  Search {
    #[arg(help = "Pre-fill the filter (optional; you can also type in the picker)")]
    query: Option<String>,

    #[arg(
      long,
      help = "Output matching results as JSON instead of opening the picker"
    )]
    json: bool,
  },

  #[command(
    alias = "doctor",
    about = "Show CLI version, data paths and login status"
  )]
  Info {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },

  #[command(about = "Show this project's template and applied addons (anesis status)")]
  Status {
    #[arg(long, help = "Output as JSON")]
    json: bool,
  },
}
