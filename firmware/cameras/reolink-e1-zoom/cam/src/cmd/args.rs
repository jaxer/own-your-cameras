//! `--name VALUE` options that may appear anywhere among a command's words.

use super::{CliError, failed};

pub struct Opts<'a> {
    values: Vec<(&'a str, &'a str)>,
    pub words: Vec<&'a str>,
}

impl<'a> Opts<'a> {
    /// Splits `args` into the `names` options (each followed by one value) and everything else.
    pub fn split(args: &'a [String], names: &[&'a str]) -> Result<Self, CliError> {
        let mut values = Vec::new();
        let mut words = Vec::new();
        let mut it = args.iter().map(String::as_str);
        while let Some(a) = it.next() {
            match names.iter().find(|n| **n == a) {
                Some(name) => values.push((
                    *name,
                    it.next()
                        .ok_or_else(|| failed(format!("{a} needs a value")))?,
                )),
                None => words.push(a),
            }
        }
        Ok(Self { values, words })
    }

    /// The last value given for `name`, or `default`.
    pub fn get(&self, name: &str, default: &str) -> String {
        self.values
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map_or(default, |(_, v)| v)
            .to_string()
    }
}

pub fn number(s: &str) -> Result<u32, CliError> {
    s.parse().map_err(|_| failed(format!("not a number: {s}")))
}

#[cfg(test)]
pub fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_come_out_wherever_they_are() {
        let a = words("home --dev /x stop --cfg /y");
        let o = Opts::split(&a, &["--dev", "--cfg"]).unwrap();
        assert_eq!(o.words, ["home", "stop"]);
        assert_eq!(
            (o.get("--dev", "d"), o.get("--cfg", "c")),
            ("/x".into(), "/y".into())
        );
    }

    #[test]
    fn missing_options_use_the_default_and_the_last_value_wins() {
        let a = words("--dev /a --dev /b");
        let o = Opts::split(&a, &["--dev", "--cfg"]).unwrap();
        assert_eq!(
            (o.get("--dev", "d"), o.get("--cfg", "c")),
            ("/b".into(), "c".into())
        );
    }

    #[test]
    fn an_option_without_a_value_is_an_error() {
        let a = words("home --dev");
        assert_eq!(
            Opts::split(&a, &["--dev"]).err(),
            Some(CliError::Failed("--dev needs a value".into()))
        );
    }

    #[test]
    fn numbers_reject_junk_and_negatives() {
        assert_eq!(number("12"), Ok(12));
        assert!(number("-1").is_err() && number("far").is_err());
    }
}
