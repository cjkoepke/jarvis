use crate::agents::{self, Agent};
use crate::files::find_on_path;
use crate::paths::Env;
use crate::settings::{ReviewSettings, Settings};
use crate::ui::message;
use crate::ui::prompt::{Answer, Picker, Radio, Row, Screen, TextInput};
use anyhow::{Context, Result};
use std::process::ExitCode;

const MENU_HELP: &str = "↑↓ move · enter/space change · esc close";
const CHOICE_HELP: &str = "↑↓ move · enter/space select · esc back";
const OTHER_MODEL: &str = "Other…";

pub fn run(env: &Env) -> Result<ExitCode> {
    let file = env.settings_file();
    let saved = Settings::load(&file)?;
    let mut settings = saved.clone();
    let mut screen =
        Screen::open().with_context(|| format!("`jarvis settings` cannot open its screen; edit {} instead", file.display()))?;

    let mut cursor = 0;
    loop {
        let menu = Picker {
            title: "Settings",
            subtitle: file.to_str().unwrap_or_default(),
            rows: Setting::ALL.iter().map(|setting| setting.menu_row(&settings.review)).collect(),
            cursor,
            help: MENU_HELP,
        };
        let Answer::Chosen(chosen) = menu.ask(&mut screen)? else {
            break;
        };
        cursor = chosen;
        let before_edit = settings.clone();
        let answer = Setting::ALL[chosen].edit(&mut screen, env, &mut settings.review)?;
        if settings != before_edit {
            settings.save(&file)?;
        }
        if let Answer::Quit = answer {
            break;
        }
    }
    drop(screen);

    if settings != saved {
        message::done("settings", &format!("saved to {}", file.display()));
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Clone, Copy)]
enum Setting {
    ReviewAgent,
    ReviewModel,
}

impl Setting {
    const ALL: [Setting; 2] = [Setting::ReviewAgent, Setting::ReviewModel];

    fn label(self) -> &'static str {
        match self {
            Setting::ReviewAgent => "Pre-push reviewer",
            Setting::ReviewModel => "Review model",
        }
    }

    fn menu_row(self, review: &ReviewSettings) -> Row {
        let value = match self {
            Setting::ReviewAgent => agent_name(&review.agent).to_string(),
            Setting::ReviewModel => model_label(review, review.model.as_ref()),
        };
        Row {
            label: self.label().to_string(),
            note: value,
            radio: Radio::Absent,
        }
    }

    fn edit(self, screen: &mut Screen, env: &Env, review: &mut ReviewSettings) -> Result<Answer<()>> {
        match self {
            Setting::ReviewAgent => choose_review_agent(screen, review),
            Setting::ReviewModel => choose_review_model(screen, env, review),
        }
    }
}

fn choose_review_agent(screen: &mut Screen, review: &mut ReviewSettings) -> Result<Answer<()>> {
    let agents = agents::all();
    let rows = agents.iter().map(|agent| Row {
        label: agent.display_name().to_string(),
        note: missing_executable_note(*agent),
        radio: radio(agent.id() == review.agent),
    });
    let picker = Picker {
        title: Setting::ReviewAgent.label(),
        subtitle: "The agent that reviews outgoing commits before every push.",
        rows: rows.collect(),
        cursor: agents.iter().position(|agent| agent.id() == review.agent).unwrap_or(0),
        help: CHOICE_HELP,
    };
    let answer = picker.ask(screen)?;
    Ok(answer.map(|chosen| review.switch_agent(agents[chosen].id())))
}

fn missing_executable_note(agent: &dyn Agent) -> String {
    match find_on_path(agent.executable_name()) {
        Some(_) => String::new(),
        None => format!("`{}` is not on PATH, so pushes would go out unreviewed", agent.executable_name()),
    }
}

fn choose_review_model(screen: &mut Screen, env: &Env, review: &mut ReviewSettings) -> Result<Answer<()>> {
    let offered = agents::by_id(&review.agent)
        .map(|agent| agent.review_models(env))
        .unwrap_or_default();
    let about = |model: &Option<String>| offered.iter().find(|offered| Some(&offered.name) == model.as_ref());
    let mut models: Vec<Option<String>> = vec![None];
    models.extend(offered.iter().map(|model| Some(model.name.clone())));
    if !models.contains(&review.model) {
        models.push(review.model.clone());
    }

    let mut rows: Vec<Row> = models
        .iter()
        .map(|model| Row {
            label: model_label(review, model.as_ref()),
            note: about(model).map(|offered| offered.about.clone()).unwrap_or_default(),
            radio: radio(*model == review.model),
        })
        .collect();
    rows.push(Row {
        label: OTHER_MODEL.to_string(),
        note: "type a model name".to_string(),
        radio: Radio::Empty,
    });

    let picker = Picker {
        title: Setting::ReviewModel.label(),
        subtitle: &format!("The model {} reviews on.", agent_name(&review.agent)),
        rows,
        cursor: models.iter().position(|model| *model == review.model).unwrap_or(0),
        help: CHOICE_HELP,
    };
    let model = match picker.ask(screen)? {
        Answer::Chosen(chosen) if chosen == models.len() => {
            let input = TextInput {
                title: Setting::ReviewModel.label(),
                subtitle: &format!("A model name {} accepts.", agent_name(&review.agent)),
                text: String::new(),
            };
            input.ask(screen)?.map(Some)
        }
        answer => answer.map(|chosen| models[chosen].clone()),
    };
    Ok(model.map(|model| review.model = model))
}

fn radio(is_current: bool) -> Radio {
    if is_current {
        Radio::Filled
    } else {
        Radio::Empty
    }
}

fn agent_name(agent_id: &str) -> &str {
    agents::by_id(agent_id).map_or(agent_id, |agent| agent.display_name())
}

fn model_label(review: &ReviewSettings, model: Option<&String>) -> String {
    model.cloned().unwrap_or_else(|| format!("{}'s default", agent_name(&review.agent)))
}
