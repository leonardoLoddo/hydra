use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq)]
pub(super) enum CommandTemplateError {
    UnsupportedPlaceholder,
    Nul,
}

pub(super) fn expand(
    template: &str,
    placeholders: &BTreeMap<&'static str, &str>,
) -> Result<String, CommandTemplateError> {
    let mut remaining = template;
    let mut expanded = String::with_capacity(template.len());
    while let Some(open) = remaining.find('{') {
        let (literal, placeholder_and_rest) = remaining.split_at(open);
        if literal.contains('}') {
            return Err(CommandTemplateError::UnsupportedPlaceholder);
        }
        expanded.push_str(literal);
        let Some(close) = placeholder_and_rest.find('}') else {
            return Err(CommandTemplateError::UnsupportedPlaceholder);
        };
        let (placeholder, rest) = placeholder_and_rest.split_at(close + 1);
        let value = placeholders
            .get(placeholder)
            .ok_or(CommandTemplateError::UnsupportedPlaceholder)?;
        expanded.push_str(value);
        remaining = rest;
    }
    if remaining.contains('}') {
        return Err(CommandTemplateError::UnsupportedPlaceholder);
    }
    expanded.push_str(remaining);
    if expanded.contains('\0') {
        return Err(CommandTemplateError::Nul);
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{CommandTemplateError, expand};

    #[test]
    fn placeholder_values_may_contain_literal_braces() {
        let placeholders = BTreeMap::from([("{path}", "/projects/{demo}/payment")]);

        let expanded =
            expand("--folder={path}", &placeholders).expect("value braces should remain literal");

        assert_eq!(expanded, "--folder=/projects/{demo}/payment");
    }

    #[test]
    fn unsupported_placeholders_are_rejected() {
        let placeholders = BTreeMap::from([("{path}", "/projects/demo/payment")]);

        let error = expand("{unknown}", &placeholders)
            .expect_err("unknown placeholders must not reach an adapter");

        assert_eq!(error, CommandTemplateError::UnsupportedPlaceholder);
    }

    #[test]
    fn expanded_nul_is_rejected_before_process_launch() {
        let placeholders = BTreeMap::from([("{path}", "/projects/demo\0payment")]);

        let error = expand("--folder={path}", &placeholders)
            .expect_err("NUL must not reach a process argument");

        assert_eq!(error, CommandTemplateError::Nul);
    }
}
