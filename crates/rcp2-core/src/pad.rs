use rcp2_protocol::device::DeviceProfile;
use rcp2_protocol::types::Structured;

use crate::{get_bool, get_f64, get_string, get_u32};

#[derive(Debug, Clone)]
pub struct PadInfo {
    pub idx: usize,
    pub child_index: usize,
    pub color_index: usize,
    pub name: String,
    pub pad_type: PadType,
    pub color: PadColor,
    pub active: bool,
    pub progress: f64,
    pub gain: f64,
    pub looping: bool,
    pub replay: bool,
    pub play_mode: u32,
    pub file_path: String,
    pub env_start: f64,
    pub env_stop: f64,
}

impl PadInfo {
    #[must_use]
    pub fn from_node_at(node: &Structured, child_index: usize) -> Self {
        let color_index = get_u32(node, "padColourIndex") as usize;
        PadInfo {
            idx: get_u32(node, "padIdx") as usize,
            child_index,
            color_index,
            name: get_string(node, "padName"),
            pad_type: PadType::from_u32(get_u32(node, "padType")),
            color: PadColor::from_index(color_index),
            active: get_bool(node, "padActive"),
            progress: get_f64(node, "padProgress"),
            gain: get_f64(node, "padGain"),
            looping: get_bool(node, "padLoop"),
            replay: get_bool(node, "padReplay"),
            play_mode: get_u32(node, "padPlayMode"),
            file_path: get_string(node, "padFilePath"),
            env_start: get_f64(node, "padEnvStart"),
            env_stop: get_f64(node, "padEnvStop"),
        }
    }

    #[must_use]
    pub fn bank(&self, pads_per_bank: usize) -> usize {
        self.idx / pads_per_bank
    }

    #[must_use]
    pub fn position_in_bank(&self, pads_per_bank: usize) -> usize {
        self.idx % pads_per_bank
    }

    #[must_use]
    pub fn has_sound(&self) -> bool {
        self.pad_type == PadType::Sound && !self.file_path.is_empty()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && self.file_path.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadType {
    Sound,
    Effect,
    Special,
    Unknown(u32),
}

impl PadType {
    #[must_use]
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 | 1 => PadType::Sound,
            2 => PadType::Effect,
            3 => PadType::Special,
            other => PadType::Unknown(other),
        }
    }

    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            PadType::Sound => "SND",
            PadType::Effect => "FX",
            PadType::Special => "SPC",
            PadType::Unknown(_) => "???",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadColor {
    Red,
    Orange,
    Yellow,
    LightGreen,
    Green,
    Mint,
    Cyan,
    LightBlue,
    Blue,
    Purple,
    Pink,
    Magenta,
}

impl PadColor {
    #[must_use]
    pub fn from_index(i: usize) -> Self {
        const COLORS: [PadColor; 12] = [
            PadColor::Red,
            PadColor::Orange,
            PadColor::Yellow,
            PadColor::LightGreen,
            PadColor::Green,
            PadColor::Mint,
            PadColor::Cyan,
            PadColor::LightBlue,
            PadColor::Blue,
            PadColor::Purple,
            PadColor::Pink,
            PadColor::Magenta,
        ];
        COLORS[i % COLORS.len()]
    }

    #[must_use]
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            PadColor::Red => (255, 60, 60),
            PadColor::Orange => (255, 140, 40),
            PadColor::Yellow => (255, 220, 40),
            PadColor::LightGreen => (140, 230, 60),
            PadColor::Green => (40, 200, 80),
            PadColor::Mint => (40, 210, 170),
            PadColor::Cyan => (40, 200, 220),
            PadColor::LightBlue => (60, 150, 255),
            PadColor::Blue => (80, 80, 255),
            PadColor::Purple => (160, 80, 230),
            PadColor::Pink => (230, 80, 180),
            PadColor::Magenta => (230, 60, 120),
        }
    }
}

pub struct BankView {
    pub bank: usize,
    pub pads: Vec<Option<PadInfo>>,
    pub rows: usize,
    pub cols: usize,
}

impl BankView {
    #[must_use]
    pub fn from_pads(all_pads: &[PadInfo], bank: usize, profile: &DeviceProfile) -> Self {
        let pads_per_bank = profile.pads_per_bank;
        let order = profile.physical_order;

        let mut by_position: Vec<Option<PadInfo>> = vec![None; pads_per_bank];
        for pad in all_pads {
            if pad.bank(pads_per_bank) == bank {
                let pos = pad.position_in_bank(pads_per_bank);
                if pos < pads_per_bank {
                    by_position[pos] = Some(pad.clone());
                }
            }
        }

        let mut pads: Vec<Option<PadInfo>> = vec![None; pads_per_bank];
        for (display_idx, &logical_idx) in order.iter().enumerate() {
            if display_idx < pads_per_bank && logical_idx < pads_per_bank {
                pads[display_idx].clone_from(&by_position[logical_idx]);
            }
        }

        BankView {
            bank,
            pads,
            rows: profile.pad_rows,
            cols: profile.pad_cols,
        }
    }

    #[must_use]
    pub fn logical_index(display_index: usize, profile: &DeviceProfile) -> usize {
        profile
            .physical_order
            .get(display_index)
            .copied()
            .unwrap_or(display_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcp2_protocol::device::model;
    use rcp2_protocol::types::{Structured, Value};
    use std::collections::HashMap;

    fn make_node(props: Vec<(&str, Value)>) -> Structured {
        let mut properties = HashMap::new();
        for (k, v) in props {
            properties.insert(k.to_string(), v);
        }
        Structured {
            name: "PAD".to_string(),
            properties,
            children: vec![],
        }
    }

    fn make_pad(idx: usize) -> PadInfo {
        let node = make_node(vec![(
            "padIdx",
            Value::U32(u32::try_from(idx).unwrap_or(0)),
        )]);
        PadInfo::from_node_at(&node, 0)
    }

    #[test]
    fn pad_info_from_node() {
        let node = make_node(vec![
            ("padIdx", Value::U32(5)),
            ("padName", Value::String("Test".into())),
            ("padColourIndex", Value::U32(3)),
            ("padType", Value::U32(1)),
            ("padGain", Value::F64(-12.0)),
            ("padLoop", Value::Bool(true)),
            ("padReplay", Value::Bool(false)),
            ("padPlayMode", Value::U32(2)),
            ("padFilePath", Value::String("sound.wav".into())),
            ("padActive", Value::Bool(false)),
            ("padProgress", Value::F64(0.5)),
            ("padEnvStart", Value::F64(0.0)),
            ("padEnvStop", Value::F64(1.0)),
        ]);

        let pad = PadInfo::from_node_at(&node, 7);

        assert_eq!(pad.idx, 5);
        assert_eq!(pad.child_index, 7);
        assert_eq!(pad.name, "Test");
        assert_eq!(pad.color_index, 3);
        assert_eq!(pad.pad_type, PadType::Sound);
        assert_eq!(pad.color, PadColor::LightGreen);
        assert!(!pad.active);
        assert!((pad.progress - 0.5).abs() < f64::EPSILON);
        assert!((pad.gain + 12.0).abs() < f64::EPSILON);
        assert!(pad.looping);
        assert!(!pad.replay);
        assert_eq!(pad.play_mode, 2);
        assert_eq!(pad.file_path, "sound.wav");
        assert!(pad.env_start.abs() < f64::EPSILON);
        assert!((pad.env_stop - 1.0).abs() < f64::EPSILON);
        assert_eq!(pad.bank(8), 0);
        assert_eq!(pad.position_in_bank(8), 5);
    }

    #[test]
    fn pad_info_defaults_on_missing_props() {
        let node = make_node(vec![]);
        let pad = PadInfo::from_node_at(&node, 0);

        assert_eq!(pad.idx, 0);
        assert_eq!(pad.name, "");
        assert_eq!(pad.color_index, 0);
        assert_eq!(pad.pad_type, PadType::Sound);
        assert!(!pad.active);
        assert!(pad.progress.abs() < f64::EPSILON);
        assert!(pad.gain.abs() < f64::EPSILON);
        assert!(!pad.looping);
        assert!(!pad.replay);
        assert_eq!(pad.play_mode, 0);
        assert_eq!(pad.file_path, "");
        assert!(pad.env_start.abs() < f64::EPSILON);
        assert!(pad.env_stop.abs() < f64::EPSILON);
    }

    #[test]
    fn bank_view_physical_order_pro_ii() {
        let profile = &model::PRO_II;
        let pads: Vec<PadInfo> = (0..8).map(make_pad).collect();
        let view = BankView::from_pads(&pads, 0, profile);

        for (display_idx, &expected_pos) in profile.physical_order.iter().enumerate() {
            let pad = view.pads[display_idx].as_ref().unwrap();
            assert_eq!(
                pad.position_in_bank(profile.pads_per_bank),
                expected_pos,
                "display_idx={display_idx}"
            );
        }
    }

    #[test]
    fn bank_view_physical_order_duo() {
        let profile = &model::DUO;
        let pads: Vec<PadInfo> = (0..6).map(make_pad).collect();
        let view = BankView::from_pads(&pads, 0, profile);

        assert_eq!(view.pads.len(), 6);
        assert_eq!(view.rows, 3);
        assert_eq!(view.cols, 2);

        for (display_idx, &expected_pos) in profile.physical_order.iter().enumerate() {
            let pad = view.pads[display_idx].as_ref().unwrap();
            assert_eq!(
                pad.position_in_bank(profile.pads_per_bank),
                expected_pos,
                "display_idx={display_idx}"
            );
        }
    }

    #[test]
    fn bank_view_filters_by_bank() {
        let profile = &model::PRO_II;
        let mut pads = Vec::new();
        for i in 0..16 {
            pads.push(make_pad(i));
        }

        let view = BankView::from_pads(&pads, 1, profile);
        assert_eq!(view.bank, 1);
        for slot in &view.pads {
            let pad = slot.as_ref().unwrap();
            assert_eq!(pad.bank(profile.pads_per_bank), 1);
        }
    }

    #[test]
    fn logical_index_mapping_pro_ii() {
        let profile = &model::PRO_II;
        assert_eq!(BankView::logical_index(0, profile), 0);
        assert_eq!(BankView::logical_index(1, profile), 4);
        assert_eq!(BankView::logical_index(2, profile), 1);
        assert_eq!(BankView::logical_index(3, profile), 5);
        assert_eq!(BankView::logical_index(4, profile), 2);
        assert_eq!(BankView::logical_index(5, profile), 6);
        assert_eq!(BankView::logical_index(6, profile), 3);
        assert_eq!(BankView::logical_index(7, profile), 7);
    }

    #[test]
    fn logical_index_mapping_duo() {
        let profile = &model::DUO;
        assert_eq!(BankView::logical_index(0, profile), 0);
        assert_eq!(BankView::logical_index(1, profile), 3);
        assert_eq!(BankView::logical_index(2, profile), 1);
        assert_eq!(BankView::logical_index(3, profile), 4);
        assert_eq!(BankView::logical_index(4, profile), 2);
        assert_eq!(BankView::logical_index(5, profile), 5);
    }

    #[test]
    fn bank_calculation_duo() {
        let pad = make_pad(7);
        assert_eq!(pad.bank(6), 1);
        assert_eq!(pad.position_in_bank(6), 1);
    }
}
