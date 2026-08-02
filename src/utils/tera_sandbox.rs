use std::collections::HashMap;

use tera::{Tera, Value};

const ONE_OFF_TEMPLATE_NAME: &str = "__anesis_one_off__";

pub fn hardened_tera() -> Tera {
  let mut tera = Tera::default();
  tera.register_function("get_env", deny_get_env);
  tera
}

fn deny_get_env(_args: &HashMap<String, Value>) -> tera::Result<Value> {
  Err(tera::Error::msg(
    "get_env is not available in Anesis addon/template rendering",
  ))
}

pub fn render_string(s: &str, ctx: &tera::Context) -> anyhow::Result<String> {
  let mut tera = hardened_tera();
  tera.add_raw_template(ONE_OFF_TEMPLATE_NAME, s)?;
  Ok(tera.render(ONE_OFF_TEMPLATE_NAME, ctx)?)
}
