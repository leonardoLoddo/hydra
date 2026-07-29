use std::path::Path;

pub(super) fn safe_terminal_text(value: &str) -> String {
    let mut label = String::new();
    for character in value.chars() {
        if character.is_control() {
            label.extend(character.escape_default());
        } else {
            label.push(character);
        }
    }
    label
}

pub(super) fn safe_path_label(path: &Path) -> String {
    safe_terminal_text(&path.to_string_lossy())
}
