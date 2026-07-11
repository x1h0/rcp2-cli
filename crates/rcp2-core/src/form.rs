use crate::{PadInfo, PadType};

const PLAY_MODES: &[&str] = &["Toggle", "Hold", "One Shot"];

#[must_use]
pub fn play_mode_label(value: u32) -> String {
    PLAY_MODES
        .get(value as usize)
        .unwrap_or(&"Unknown")
        .to_string()
}

#[must_use]
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

impl FormField {
    fn new(label: &str, kind: FieldKind, value_display: String, property: Option<&str>) -> Self {
        FormField {
            label: label.into(),
            kind,
            value_display,
            property: property.map(Into::into),
        }
    }
}

#[must_use]
pub fn new_pad_fields() -> Vec<FormField> {
    vec![
        FormField::new("Name", FieldKind::Text, String::new(), Some("padName")),
        FormField::new(
            "Color",
            FieldKind::ColorCycle,
            format!("#{:02x}{:02x}{:02x}", 255, 60, 60),
            Some("padColourIndex"),
        ),
        FormField::new(
            "Sound",
            FieldKind::FilePicker,
            "(none)".into(),
            Some("soundFile"),
        ),
        FormField::new("Create pad", FieldKind::Action, String::new(), Some("create")),
    ]
}

#[must_use]
pub fn replace_sound_fields(pad_name: &str, filename: &str, duration: Option<f64>) -> Vec<FormField> {
    let dur_label = duration.map_or_else(|| "?".into(), |d| format!("{d:.2}s"));

    let mut fields = vec![
        FormField::new("Pad", FieldKind::ReadOnly, pad_name.into(), None),
        FormField::new("File", FieldKind::ReadOnly, filename.into(), None),
        FormField::new("Duration", FieldKind::ReadOnly, dur_label, None),
    ];

    if let Some(dur) = duration {
        fields.push(FormField::new(
            "Start",
            FieldKind::Number,
            "0.0".into(),
            Some("replaceEnvStart"),
        ));
        fields.push(FormField::new(
            "End",
            FieldKind::Number,
            format!("{dur:.2}"),
            Some("replaceEnvStop"),
        ));
    }

    fields.push(FormField::new(
        "Confirm replace",
        FieldKind::Action,
        String::new(),
        Some("replaceConfirm"),
    ));
    fields.push(FormField::new(
        "Cancel",
        FieldKind::Action,
        String::new(),
        Some("replaceCancel"),
    ));
    fields
}

#[must_use]
pub fn pad_fields(pad: &PadInfo, dry_run: bool) -> Vec<FormField> {
    let mut fields = common_fields(pad);

    if pad.pad_type == PadType::Sound {
        fields.extend(sound_fields(pad));
    }

    if !pad.file_path.is_empty() {
        fields.push(FormField::new(
            "File",
            FieldKind::ReadOnly,
            pad.file_path.clone(),
            None,
        ));
    }

    fields.extend(action_fields(pad, dry_run));
    fields
}

fn common_fields(pad: &PadInfo) -> Vec<FormField> {
    let (r, g, b) = pad.color.to_rgb();

    vec![
        FormField::new("Name", FieldKind::Text, pad.name.clone(), Some("padName")),
        FormField::new(
            "Color",
            FieldKind::ColorCycle,
            format!("#{r:02x}{g:02x}{b:02x}"),
            Some("padColourIndex"),
        ),
        FormField::new(
            "Type",
            FieldKind::ReadOnly,
            pad.pad_type.label().into(),
            None,
        ),
        FormField::new(
            "Gain",
            FieldKind::Number,
            format!("{:.1} dB", pad.gain),
            Some("padGain"),
        ),
    ]
}

fn sound_fields(pad: &PadInfo) -> Vec<FormField> {
    let mut fields = vec![
        FormField::new(
            "Mode",
            FieldKind::Cycle,
            play_mode_label(pad.play_mode),
            Some("padPlayMode"),
        ),
        FormField::new(
            "Loop",
            FieldKind::Toggle,
            if pad.looping { "Yes" } else { "No" }.into(),
            Some("padLoop"),
        ),
        FormField::new(
            "Replay",
            FieldKind::Toggle,
            if pad.replay { "Yes" } else { "No" }.into(),
            Some("padReplay"),
        ),
    ];

    if !pad.file_path.is_empty() {
        fields.push(FormField::new(
            "Start",
            FieldKind::ReadOnly,
            format!("{:.0}%", pad.env_start * 100.0),
            None,
        ));
        fields.push(FormField::new(
            "End",
            FieldKind::ReadOnly,
            format!("{:.0}%", pad.env_stop * 100.0),
            None,
        ));
    }

    fields
}

fn action_fields(pad: &PadInfo, dry_run: bool) -> Vec<FormField> {
    let is_sound = pad.pad_type == PadType::Sound;
    let has_file = !pad.file_path.is_empty();
    let mut fields = vec![];

    if !dry_run && is_sound && has_file {
        fields.push(FormField::new(
            "Download sound",
            FieldKind::Action,
            String::new(),
            Some("download"),
        ));
    }
    if !dry_run && is_sound {
        fields.push(FormField::new(
            if has_file { "Replace sound" } else { "Upload sound" },
            FieldKind::Action,
            String::new(),
            Some("upload"),
        ));
    }
    fields.push(FormField::new(
        "Play / Stop",
        FieldKind::Action,
        String::new(),
        Some("play"),
    ));
    if !dry_run {
        fields.push(FormField::new("Move pad", FieldKind::Action, String::new(), Some("move")));
    }
    fields.push(FormField::new(
        "Delete pad",
        FieldKind::Action,
        String::new(),
        Some("delete"),
    ));

    fields
}
