use std::fmt;

use clap::ValueEnum;

#[derive(Debug, Clone)]
pub enum LevelFilter {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LevelFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ValueEnum for LevelFilter {
    fn value_variants<'a>() -> &'a [Self] {
        static VARIANTS: &'static [LevelFilter] = &[
            //TODO Improve this
            LevelFilter::Off,
            LevelFilter::Error,
            LevelFilter::Warn,
            LevelFilter::Info,
            LevelFilter::Debug,
            LevelFilter::Trace,
        ];
        return VARIANTS;
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        return Some(clap::builder::PossibleValue::new(&self.to_string()));
    }
}

impl From<LevelFilter> for log::LevelFilter {
    fn from(item: LevelFilter) -> Self {
        match item {
            LevelFilter::Off => log::LevelFilter::Off,
            LevelFilter::Error => log::LevelFilter::Error,
            LevelFilter::Warn => log::LevelFilter::Warn,
            LevelFilter::Info => log::LevelFilter::Info,
            LevelFilter::Debug => log::LevelFilter::Debug,
            LevelFilter::Trace => log::LevelFilter::Trace,
        }
    }
}
