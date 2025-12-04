use eddist_core::utils::is_prod;
use handlebars::Handlebars;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub template: TemplateConfig,
}

#[derive(Debug, Deserialize)]
pub struct TemplateConfig {
    pub engine: String,
    pub templates: Vec<Template>,
}

#[derive(Debug, Deserialize)]
pub struct Template {
    pub name: String,
    pub path: String,
}

pub fn load_template_engine() -> Handlebars<'static> {
    let template_config = toml::from_str::<Config>(
        &std::fs::read_to_string(if is_prod() {
            "resources/templates.prod.toml"
        } else {
            "eddist-server/resources/templates.local.toml"
        })
        .expect("Failed to read templates.toml"),
    )
    .expect("Failed to parse templates.toml");

    let mut handlebars = Handlebars::new();

    if template_config.template.engine != "handlebars" {
        panic!(
            "Unsupported template engine: {}",
            template_config.template.engine
        );
    }

    for Template { name, path } in template_config.template.templates {
        handlebars
            .register_template_file(&name, path)
            .expect("Failed to register template");
    }

    handlebars
}
