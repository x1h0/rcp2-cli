use rcp2_core::{PadInfo, PadType};

const PLAY_MODES: &[&str] = &["Toggle", "Hold", "One Shot"];

pub fn play_mode_label(value: u32) -> String {
    PLAY_MODES
        .get(value as usize)
        .unwrap_or(&"Unknown")
        .to_string()
}

pub fn play_mode_count() -> u32 {
    u32::try_from(PLAY_MODES.len()).unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldKind {
    Text,
    Number,
    ColorCycle,
    Cycle,
    Toggle,
    FilePicker,
    ReadOnly,
    Action,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub label: String,
    pub kind: FieldKind,
    pub value_display: String,
    pub property: Option<String>,
}

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
        let dur_label = duration.map_or_else(|| "?".into(), |d| format!("{d:.2}s"));

        let mut fields = vec![
            FormField {
                label: "Pad".into(),
                kind: FieldKind::ReadOnly,
                value_display: pad_name.into(),
                property: None,
            },
            FormField {
                label: "File".into(),
                kind: FieldKind::ReadOnly,
                value_display: filename.into(),
                property: None,
            },
            FormField {
                label: "Duration".into(),
                kind: FieldKind::ReadOnly,
                value_display: dur_label,
                property: None,
            },
        ];

        if let Some(dur) = duration {
            fields.push(FormField {
                label: "Start".into(),
                kind: FieldKind::Number,
                value_display: "0.0".into(),
                property: Some("replaceEnvStart".into()),
            });
            fields.push(FormField {
                label: "End".into(),
                kind: FieldKind::Number,
                value_display: format!("{dur:.2}"),
                property: Some("replaceEnvStop".into()),
            });
        }

        fields.push(FormField {
            label: "Confirm replace".into(),
            kind: FieldKind::Action,
            value_display: String::new(),
            property: Some("replaceConfirm".into()),
        });
        fields.push(FormField {
            label: "Cancel".into(),
            kind: FieldKind::Action,
            value_display: String::new(),
            property: Some("replaceCancel".into()),
        });

        DetailForm {
            fields,
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
        let fields = vec![
            FormField {
                label: "Name".into(),
                kind: FieldKind::Text,
                value_display: String::new(),
                property: Some("padName".into()),
            },
            FormField {
                label: "Color".into(),
                kind: FieldKind::ColorCycle,
                value_display: format!("#{:02x}{:02x}{:02x}", 255, 60, 60),
                property: Some("padColourIndex".into()),
            },
            FormField {
                label: "Sound".into(),
                kind: FieldKind::FilePicker,
                value_display: "(none)".into(),
                property: Some("soundFile".into()),
            },
            FormField {
                label: "Create pad".into(),
                kind: FieldKind::Action,
                value_display: String::new(),
                property: Some("create".into()),
            },
        ];

        DetailForm {
            fields,
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
        let mut fields = Self::common_fields(pad);

        if pad.pad_type == PadType::Sound {
            fields.extend(Self::sound_fields(pad));
        }

        if !pad.file_path.is_empty() {
            fields.push(FormField {
                label: "File".into(),
                kind: FieldKind::ReadOnly,
                value_display: pad.file_path.clone(),
                property: None,
            });
        }

        fields.extend(Self::action_fields(pad, dry_run));

        DetailForm {
            fields,
            selected: 0,
            editing_text: None,
            new_pad_idx: None,
            new_pad_color: pad.color_index,
            picked_file_path: None,
            audio_duration: None,
            is_replace: false,
        }
    }

    fn common_fields(pad: &PadInfo) -> Vec<FormField> {
        let (r, g, b) = pad.color.to_rgb();

        vec![
            FormField {
                label: "Name".into(),
                kind: FieldKind::Text,
                value_display: pad.name.clone(),
                property: Some("padName".into()),
            },
            FormField {
                label: "Color".into(),
                kind: FieldKind::ColorCycle,
                value_display: format!("#{r:02x}{g:02x}{b:02x}"),
                property: Some("padColourIndex".into()),
            },
            FormField {
                label: "Type".into(),
                kind: FieldKind::ReadOnly,
                value_display: pad.pad_type.label().into(),
                property: None,
            },
            FormField {
                label: "Gain".into(),
                kind: FieldKind::Number,
                value_display: format!("{:.1} dB", pad.gain),
                property: Some("padGain".into()),
            },
        ]
    }

    fn sound_fields(pad: &PadInfo) -> Vec<FormField> {
        let mut fields = vec![
            FormField {
                label: "Mode".into(),
                kind: FieldKind::Cycle,
                value_display: play_mode_label(pad.play_mode),
                property: Some("padPlayMode".into()),
            },
            FormField {
                label: "Loop".into(),
                kind: FieldKind::Toggle,
                value_display: if pad.looping { "Yes" } else { "No" }.into(),
                property: Some("padLoop".into()),
            },
            FormField {
                label: "Replay".into(),
                kind: FieldKind::Toggle,
                value_display: if pad.replay { "Yes" } else { "No" }.into(),
                property: Some("padReplay".into()),
            },
        ];

        if !pad.file_path.is_empty() {
            fields.push(FormField {
                label: "Start".into(),
                kind: FieldKind::ReadOnly,
                value_display: format!("{:.0}%", pad.env_start * 100.0),
                property: None,
            });
            fields.push(FormField {
                label: "End".into(),
                kind: FieldKind::ReadOnly,
                value_display: format!("{:.0}%", pad.env_stop * 100.0),
                property: None,
            });
        }

        fields
    }

    fn action_fields(pad: &PadInfo, dry_run: bool) -> Vec<FormField> {
        let is_sound = pad.pad_type == PadType::Sound;
        let has_file = !pad.file_path.is_empty();
        let mut fields = vec![];

        if !dry_run && is_sound && has_file {
            fields.push(FormField {
                label: "Download sound".into(),
                kind: FieldKind::Action,
                value_display: String::new(),
                property: Some("download".into()),
            });
        }
        if !dry_run && is_sound {
            fields.push(FormField {
                label: if has_file {
                    "Replace sound".into()
                } else {
                    "Upload sound".into()
                },
                kind: FieldKind::Action,
                value_display: String::new(),
                property: Some("upload".into()),
            });
        }
        fields.push(FormField {
            label: "Play / Stop".into(),
            kind: FieldKind::Action,
            value_display: String::new(),
            property: Some("play".into()),
        });
        if !dry_run {
            fields.push(FormField {
                label: "Move pad".into(),
                kind: FieldKind::Action,
                value_display: String::new(),
                property: Some("move".into()),
            });
        }
        fields.push(FormField {
            label: "Delete pad".into(),
            kind: FieldKind::Action,
            value_display: String::new(),
            property: Some("delete".into()),
        });

        fields
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
