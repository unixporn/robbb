use poise::{
    serenity_prelude::{command::CommandOptionType, CreateApplicationCommandOption},
    ApplicationCommandOrAutocompleteInteraction, SlashArgument,
};
use serde::{Deserialize, Serialize};
use serenity::{async_trait, client};
use std::{fmt, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub enum FetchField {
    Distro,
    Kernel,
    Terminal,
    Editor,
    #[serde(rename = "DE/WM")]
    DEWM,
    Bar,
    Resolution,
    #[serde(rename = "Display Protocol")]
    DisplayProtocol,
    Shell,
    #[serde(rename = "GTK3 Theme")]
    GTK3,
    #[serde(rename = "GTK Icon Theme")]
    Icons,
    CPU,
    GPU,
    Memory,
    Description,
    Git,
    Dotfiles,
    #[serde(rename = "image")]
    Image,
}

pub static FETCH_KEY_ORDER: &[FetchField] = &[
    FetchField::Distro,
    FetchField::Kernel,
    FetchField::Terminal,
    FetchField::Editor,
    FetchField::DEWM,
    FetchField::Bar,
    FetchField::Resolution,
    FetchField::DisplayProtocol,
    FetchField::Shell,
    FetchField::GTK3,
    FetchField::Icons,
    FetchField::CPU,
    FetchField::GPU,
    FetchField::Memory,
    FetchField::Description,
    FetchField::Git,
    FetchField::Dotfiles,
    FetchField::Image,
];

impl fmt::Display for FetchField {
    fn fmt(&self, writer: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FetchField::DEWM => write!(writer, "DE/WM"),
            FetchField::DisplayProtocol => write!(writer, "Display Protocol"),
            FetchField::GTK3 => write!(writer, "GTK3 Theme"),
            FetchField::Icons => write!(writer, "GTK Icon Theme"),
            FetchField::Image => write!(writer, "Image"),
            _ => write!(writer, "{:?}", self),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Not a valid fetch field")]
pub struct FetchFieldParseError;

impl std::str::FromStr for FetchField {
    type Err = FetchFieldParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "distro" => Ok(Self::Distro),
            "kernel" => Ok(Self::Kernel),
            "terminal" => Ok(Self::Terminal),
            "editor" => Ok(Self::Editor),
            "dewm" | "de" | "wm" | "de/wm" => Ok(Self::DEWM),
            "bar" => Ok(Self::Bar),
            "resolution" => Ok(Self::Resolution),
            "display protocol" => Ok(Self::DisplayProtocol),
            "shell" => Ok(Self::Shell),
            "gtk theme" | "gtk3 theme" | "theme" | "gtk" => Ok(Self::GTK3),
            "icons" | "icon theme" | "gtk icon theme" => Ok(Self::Icons),
            "cpu" => Ok(Self::CPU),
            "gpu" => Ok(Self::GPU),
            "memory" => Ok(Self::Memory),
            "description" => Ok(Self::Description),
            "git" => Ok(Self::Git),
            "dotfiles" => Ok(Self::Dotfiles),
            "image" => Ok(Self::Image),
            _ => Err(FetchFieldParseError),
        }
    }
}

#[async_trait]
impl SlashArgument for FetchField {
    async fn extract(
        _: &client::Context,
        _: ApplicationCommandOrAutocompleteInteraction<'_>,
        value: &serde_json::Value,
    ) -> Result<Self, poise::SlashArgError> {
        let s = value
            .as_str()
            .ok_or(poise::SlashArgError::CommandStructureMismatch("Expected String"))?;
        Ok(FetchField::from_str(s).map_err(|e| poise::SlashArgError::Parse {
            error: Box::new(e),
            input: s.to_string(),
        })?)
    }
    fn create(builder: &mut CreateApplicationCommandOption) {
        builder.kind(CommandOptionType::String);
        for value in FETCH_KEY_ORDER.iter() {
            builder.add_string_choice(value.to_string(), value.to_string());
        }
    }
}
