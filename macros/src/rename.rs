use darling::FromMeta;
use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
    ToUpperCamelCase,
};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum RenameStrategy {
    UpperCamelCase,
    LowerCamelCase,
    SnakeCase,
    KebabCase,
    ShoutySnakeCase,
    TitleCase,
    ShoutyKebabCase,
}

impl RenameStrategy {
    pub fn transform(&self, s: &str) -> String {
        match self {
            RenameStrategy::UpperCamelCase => s.to_upper_camel_case(),
            RenameStrategy::LowerCamelCase => s.to_lower_camel_case(),
            RenameStrategy::SnakeCase => s.to_snake_case(),
            RenameStrategy::KebabCase => s.to_kebab_case(),
            RenameStrategy::ShoutySnakeCase => s.to_shouty_snake_case(),
            RenameStrategy::TitleCase => s.to_title_case(),
            RenameStrategy::ShoutyKebabCase => s.to_shouty_kebab_case(),
        }
    }
}

impl FromMeta for RenameStrategy {
    fn from_string(value: &str) -> darling::Result<Self> {
        RenameStrategy::try_from(value)
            .map_err(|_| darling::Error::custom("Invalid rename strategy"))
    }
}

impl TryFrom<&str> for RenameStrategy {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "UpperCamelCase" => Ok(Self::UpperCamelCase),
            "lowerCamelCase" => Ok(Self::LowerCamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SHOUTY_SNAKE_CASE" | "SCREAMING_SNAKE_CASE" | "SHOUTING_SNAKE_CASE" => {
                Ok(Self::ShoutySnakeCase)
            }
            "Title Case" => Ok(Self::TitleCase),
            "SHOUTY-KEBAB-CASE" | "SCREAMING-KEBAB-CASE" | "SHOUTING-KEBAB-CASE" => {
                Ok(Self::ShoutyKebabCase)
            }
            _ => Err(()),
        }
    }
}
