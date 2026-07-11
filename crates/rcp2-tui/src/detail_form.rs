use rcp2_core::PadInfo;

pub use rcp2_core::form::{FieldKind, FormField, play_mode_count, play_mode_label};

pub struct DetailForm {
    pub fields: Vec<FormField>,
    pub selected: usize,
    pub editing_text: Option<TextEdit>,
    pub new_pad_idx: Option<usize>,
    pub new_pad_color: usize,
    pub picked_file_path: Option<String>,
    pub audio_duration: Option<f64>,
    pub is_replace: bool,
}

pub struct TextEdit {
    pub input: String,
    pub cursor: usize,
}

impl DetailForm {
    pub fn replace_sound(
        pad_name: &str,
        filename: &str,
        file_path: String,
        duration: Option<f64>,
    ) -> Self {
        DetailForm {
            fields: rcp2_core::form::replace_sound_fields(pad_name, filename, duration),
            selected: 3,
            editing_text: None,
            new_pad_idx: None,
            new_pad_color: 0,
            picked_file_path: Some(file_path),
            audio_duration: duration,
            is_replace: true,
        }
    }

    pub fn new_pad(pad_idx: usize) -> Self {
        DetailForm {
            fields: rcp2_core::form::new_pad_fields(),
            selected: 0,
            editing_text: None,
            new_pad_idx: Some(pad_idx),
            new_pad_color: 0,
            picked_file_path: None,
            audio_duration: None,
            is_replace: false,
        }
    }

    pub fn from_pad(pad: &PadInfo, dry_run: bool) -> Self {
        DetailForm {
            fields: rcp2_core::form::pad_fields(pad, dry_run),
            selected: 0,
            editing_text: None,
            new_pad_idx: None,
            new_pad_color: pad.color_index,
            picked_file_path: None,
            audio_duration: None,
            is_replace: false,
        }
    }

    pub fn selected_field(&self) -> Option<&FormField> {
        self.fields.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.skip_readonly_up();
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.fields.len() {
            self.selected += 1;
            self.skip_readonly_down();
        }
    }

    fn skip_readonly_up(&mut self) {
        while self.selected > 0 && self.fields[self.selected].kind == FieldKind::ReadOnly {
            self.selected -= 1;
        }
    }

    fn skip_readonly_down(&mut self) {
        let start = self.selected;
        while self.selected + 1 < self.fields.len()
            && self.fields[self.selected].kind == FieldKind::ReadOnly
        {
            self.selected += 1;
        }
        if self.fields[self.selected].kind == FieldKind::ReadOnly {
            self.selected = start;
        }
    }

    pub fn start_text_edit(&mut self) {
        if let Some(field) = self.fields.get(self.selected)
            && matches!(field.kind, FieldKind::Text | FieldKind::Number)
        {
            let input = field.value_display.clone();
            let cursor = input.len();
            self.editing_text = Some(TextEdit { input, cursor });
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing_text.is_some()
    }

    pub fn edit_type_char(&mut self, c: char) {
        if let Some(ref mut edit) = self.editing_text {
            edit.input.insert(edit.cursor, c);
            edit.cursor += c.len_utf8();
        }
    }

    pub fn edit_backspace(&mut self) {
        if let Some(ref mut edit) = self.editing_text
            && edit.cursor > 0
        {
            let prev = edit.input[..edit.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            edit.input.remove(prev);
            edit.cursor = prev;
        }
    }

    pub fn finish_text_edit(&mut self) -> Option<String> {
        let edit = self.editing_text.take()?;
        if let Some(field) = self.fields.get_mut(self.selected) {
            field.value_display.clone_from(&edit.input);
        }
        Some(edit.input)
    }

    pub fn cancel_text_edit(&mut self) {
        self.editing_text = None;
    }
}
