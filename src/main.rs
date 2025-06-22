mod config;
mod error;
mod patch_info;
mod signature;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{
    config::{AppSection, Config},
    error::Error,
    patch_info::PatchInfo,
};
use iced::{
    Length, Task, Theme,
    keyboard::{Event as KeyboardEvent, Key, key::Named},
    padding::Padding,
    widget::{
        button, column, container, horizontal_space, pick_list, row, text, text_input,
        vertical_rule, vertical_space,
    },
};

#[derive(Debug, Clone)]
enum Message {
    Event(iced::Event),
    SelectGameDir,
    LoadConfig,
    ConfigLoaded(Result<Config, Error>),
    AppSelected(String),
    GameDirChanged(String),
    WidthCHanged(String),
    HeightChanged(String),
    ApplyPatch,
}

#[derive(Debug, Default, Clone)]
enum ConfigState {
    #[default]
    NotLoaded,
    Loaded(Config),
    Error(Error),
}

type Element<'a> = iced::Element<'a, Message>;

fn bordered_container<'a>(
    e: impl Into<Element<'a>>,
    width: f32,
) -> iced::widget::Container<'a, Message> {
    container(e).style(move |theme| container::Style {
        border: iced::Border {
            color: theme.palette().text,
            width,
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Default)]
struct App {
    config: ConfigState,
    selected_section: Option<String>,
    game_dir: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
}
impl App {
    async fn load_config(path: impl AsRef<Path>) -> Result<Config, Error> {
        tokio::fs::read_to_string(path)
            .await
            .map_err(Error::from)
            .and_then(|x: String| Config::new(&x))
    }

    fn apply_patch_to_file(
        &self,
        game_dir: &Path,
        patch_info: &PatchInfo,
        iteration: usize,
    ) -> Result<bool, Error> {
        use std::borrow::Cow;

        let width = self.width.ok_or(Error::state_error("Missing width"))?;
        let height = self.height.ok_or(Error::state_error("Missing height"))?;

        let mod_file_path = game_dir.join(patch_info.modfile.as_str());
        let undo_file_path = {
            let undo_file = patch_info
                .undofile
                .as_deref()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| {
                    Cow::Owned(format!(
                        "{}.undo{}",
                        mod_file_path.to_string_lossy(),
                        iteration
                    ))
                });

            game_dir.join(&*undo_file)
        };

        let mut file_data = {
            let mut file = File::open(&mod_file_path)?;
            let capacity = file.metadata().map(|m| m.len()).unwrap_or_default();
            let mut buf = Vec::with_capacity(capacity as usize);
            file.read_to_end(&mut buf)?;
            buf
        };

        patch_info.apply_patch(&mut file_data, width, height)
            .map_err(|e| Error::config_error(format!("{e}, iteration: {iteration}")))?;
        std::fs::copy(&mod_file_path, &undo_file_path)?;
        let mut file = File::options()
            .write(true)
            .truncate(true)
            .open(mod_file_path)?;

        file.write_all(&file_data)?;

        Ok(true)
    }

    fn apply_patches(&self, section: &AppSection) -> Result<bool, Error> {
        if let Some(dir) = self.game_dir.as_deref() {
            let game_path = Path::new(dir);

            let patched_successfully = section
                .patches
                .iter()
                .enumerate()
                .map(|(i, x)| self.apply_patch_to_file(game_path, x, i))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .all(|x| *x);

            Ok(patched_successfully)
        } else {
            Err(Error::state_error("Missing game dir"))
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen().map(Message::Event)
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Event(e) => match e {
                iced::Event::Keyboard(KeyboardEvent::KeyPressed {
                    key: Key::Named(Named::Tab),
                    modifiers,
                    ..
                }) => {
                    if modifiers.shift() {
                        iced::widget::focus_previous()
                    } else {
                        iced::widget::focus_next()
                    }
                }
                _ => Task::none(),
            },
            Message::SelectGameDir => {
                let dir = rfd::FileDialog::new().pick_folder();
                self.game_dir = dir.as_ref().map(|x| x.to_string_lossy().into_owned());

                Task::none()
            }
            Message::GameDirChanged(dir) => {
                self.game_dir = Some(dir);
                Task::none()
            }
            Message::AppSelected(app) => {
                self.selected_section = Some(app);
                Task::none()
            }
            Message::LoadConfig => {
                let file = rfd::FileDialog::new()
                    .add_filter("Config file", &["ini"])
                    .set_title("Load config file")
                    .pick_file();

                match file {
                    Some(file) => Task::perform(Self::load_config(file), Message::ConfigLoaded),
                    None => Task::none(),
                }
            }
            Message::ConfigLoaded(config) => {
                self.config = match config {
                    Ok(config) => {
                        self.selected_section = config.apps.apps.first().cloned();
                        ConfigState::Loaded(config)
                    }
                    Err(e) => ConfigState::Error(e),
                };

                Task::none()
            }
            Message::WidthCHanged(width) => {
                self.width = if width.is_empty() {
                    None
                } else if let Ok(value) = width.parse() {
                    Some(value)
                } else {
                    self.width
                };

                Task::none()
            }
            Message::HeightChanged(height) => {
                self.height = if height.is_empty() {
                    None
                } else if let Ok(value) = height.parse() {
                    Some(value)
                } else {
                    self.height
                };

                Task::none()
            }
            Message::ApplyPatch => {
                let result = match self.get_selected_app_section() {
                    Some(section) => self.apply_patches(section),
                    None => Ok(false),
                };

                match result {
                    Ok(true) => {
                        rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Info)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .set_description("Patch applied successfully")
                            .show();
                    }
                    Ok(false) => {
                        rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .set_description("Patch failed to apply")
                            .show();
                    }
                    Err(e) => {
                        rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .set_description(format!("Patch failed to apply: {e}"))
                            .show();
                    }
                }

                Task::none()
            }
        }
    }

    fn can_patch(&self, selected_section: &AppSection) -> bool {
        let game_dir = self
            .game_dir
            .as_deref()
            .map(Path::new)
            .filter(|x| x.exists());

        let has_checkfile = game_dir
            .and_then(|x| x.read_dir().ok())
            .map(|mut dir| {
                dir.any(|x| {
                    if let Ok(file) = x {
                        file.file_name()
                            .eq_ignore_ascii_case(&selected_section.checkfile)
                    } else {
                        false
                    }
                })
            })
            .unwrap_or(false);

        has_checkfile && self.width.is_some() && self.height.is_some()
    }

    fn get_selected_app_section(&self) -> Option<&AppSection> {
        match &self.config {
            ConfigState::Loaded(config) => self
                .selected_section
                .as_deref()
                .and_then(|selected| config.sections.iter().find(|x| x.name == selected)),
            _ => None,
        }
    }

    fn view(&self) -> Element {
        let config_bar = row![
            text_input(
                "Game file directory",
                self.game_dir.as_deref().unwrap_or("")
            )
            .on_input(Message::GameDirChanged),
            button("...").on_press(Message::SelectGameDir),
            vertical_rule(16),
            button("Load config").on_press(Message::LoadConfig)
        ]
        .height(Length::Shrink)
        .spacing(8)
        .padding(8);

        let body: Element = match &self.config {
            ConfigState::NotLoaded => vertical_space().into(),
            ConfigState::Loaded(config) => {
                let options = config.apps.apps.as_slice();
                let picker =
                    pick_list(options, self.selected_section.clone(), Message::AppSelected)
                        .width(Length::Fill);

                let selected = self
                    .selected_section
                    .as_deref()
                    .and_then(|selected| config.sections.iter().find(|x| x.name == selected));

                let content = {
                    let content = selected.map(|x| x.details.clone()).unwrap_or_default();
                    let t = text(content).size(20);
                    bordered_container(t, 2.0)
                        .padding(8)
                        .width(Length::Fill)
                        .height(Length::Fill)
                };

                let settings_row = row![
                    text("Width:"),
                    text_input(
                        "Width...",
                        &self.width.map(|x| x.to_string()).unwrap_or_default()
                    )
                    .on_input(Message::WidthCHanged),
                    horizontal_space(),
                    text("Height:"),
                    text_input(
                        "Height...",
                        &self.height.map(|x| x.to_string()).unwrap_or_default()
                    )
                    .on_input(Message::HeightChanged),
                ]
                .align_y(iced::alignment::Vertical::Center)
                .spacing(8);

                let patch_button = {
                    let exe_name = selected.map(|x| x.checkfile.as_str()).unwrap_or_default();
                    let content = row![
                        horizontal_space(),
                        text(format!("Patch {}", exe_name)),
                        horizontal_space(),
                    ];

                    let patch_button_enabled = selected.map(|s| self.can_patch(s)).unwrap_or(false);

                    button(content)
                        .width(Length::Fill)
                        .on_press_maybe(patch_button_enabled.then_some(Message::ApplyPatch))
                };

                column![picker, content, settings_row, patch_button]
                    .spacing(8)
                    .into()
            }
            ConfigState::Error(e) => text(e.to_string())
                .color(iced::Color::from_rgb(1.0, 0.0, 0.0))
                .into(),
        };

        let body = container(body).padding(8);
        let body = container(body)
            .padding(Padding {
                top: 0.0,
                ..iced::Padding::from(8)
            })
            .style(|theme| container::Style {
                border: iced::Border {
                    width: 2.0,
                    color: theme.palette().text,
                    ..Default::default()
                },
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill);

        column![config_bar, body]
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn run(self) -> iced::Result {
        let task = {
            let path = Path::new("patches.ini");
            if path.exists() {
                Task::perform(App::load_config(path), Message::ConfigLoaded)
            } else {
                Task::none()
            }
        };

        iced::application(env!("CARGO_BIN_NAME"), Self::update, Self::view)
            .subscription(Self::subscription)
            .centered()
            .window_size((1280.0, 720.0))
            .theme(Self::theme)
            .exit_on_close_request(true)
            .run_with(|| (self, task))
    }
}

fn main() {
    let app = App::default();
    app.run().expect("Failed to run app");
}
