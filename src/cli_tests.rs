//! Tests for CLI parser options.

#[cfg(test)]
mod tests {
    use crate::cli::CliArgs;
    use clap::Parser;

    #[test]
    fn parses_allow_wide_scope_global_flag() {
        let cli =
            CliArgs::try_parse_from(["dsx", "--allow-wide-scope", "edit", "доработай", "проект"])
                .unwrap();

        assert!(cli.allow_wide_scope);
    }
}
