use crate::transport::hid::{PRODUCT_IDS_DUO, PRODUCT_IDS_PRO_II};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceModel {
    ProII,
    Duo,
}

pub struct DeviceProfile {
    pub model: DeviceModel,
    pub display_name: &'static str,
    pub physical_faders: usize,
    pub virtual_faders: usize,
    pub pads_per_bank: usize,
    pub pad_rows: usize,
    pub pad_cols: usize,
    pub max_banks: usize,
    pub physical_order: &'static [usize],
    pub padbutton_offset: usize,
}

static PRO_II_PHYSICAL_ORDER: [usize; 8] = [0, 4, 1, 5, 2, 6, 3, 7];

// TODO(duo): verify physical pad layout with real hardware
static DUO_PHYSICAL_ORDER: [usize; 6] = [0, 3, 1, 4, 2, 5];

pub static PRO_II: DeviceProfile = DeviceProfile {
    model: DeviceModel::ProII,
    display_name: "R\u{00D8}DECaster Pro II",
    physical_faders: 6,
    virtual_faders: 3,
    pads_per_bank: 8,
    pad_rows: 4,
    pad_cols: 2,
    max_banks: 8,
    physical_order: &PRO_II_PHYSICAL_ORDER,
    padbutton_offset: 35,
};

pub static DUO: DeviceProfile = DeviceProfile {
    model: DeviceModel::Duo,
    display_name: "R\u{00D8}DECaster Duo",
    physical_faders: 4,
    virtual_faders: 5,
    pads_per_bank: 6,
    pad_rows: 3,
    pad_cols: 2,
    max_banks: 8,
    physical_order: &DUO_PHYSICAL_ORDER,
    // TODO(duo): verify with real hardware
    padbutton_offset: 35,
};

impl DeviceModel {
    #[must_use]
    pub fn profile(self) -> &'static DeviceProfile {
        match self {
            DeviceModel::ProII => &PRO_II,
            DeviceModel::Duo => &DUO,
        }
    }

    #[must_use]
    pub fn from_product_id(pid: u16) -> Option<Self> {
        if PRODUCT_IDS_PRO_II.contains(&pid) {
            Some(DeviceModel::ProII)
        } else if PRODUCT_IDS_DUO.contains(&pid) {
            Some(DeviceModel::Duo)
        } else {
            None
        }
    }
}

impl std::fmt::Display for DeviceModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.profile().display_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_ii_from_known_pids() {
        for &pid in PRODUCT_IDS_PRO_II {
            assert_eq!(DeviceModel::from_product_id(pid), Some(DeviceModel::ProII));
        }
    }

    #[test]
    fn duo_from_known_pids() {
        for &pid in PRODUCT_IDS_DUO {
            assert_eq!(DeviceModel::from_product_id(pid), Some(DeviceModel::Duo));
        }
    }

    #[test]
    fn unknown_pid_returns_none() {
        assert_eq!(DeviceModel::from_product_id(0xFFFF), None);
    }

    #[test]
    fn profile_physical_order_matches_pads_per_bank() {
        assert_eq!(PRO_II.physical_order.len(), PRO_II.pads_per_bank);
        assert_eq!(DUO.physical_order.len(), DUO.pads_per_bank);
    }

    #[test]
    fn profile_grid_matches_pads_per_bank() {
        assert_eq!(PRO_II.pad_rows * PRO_II.pad_cols, PRO_II.pads_per_bank);
        assert_eq!(DUO.pad_rows * DUO.pad_cols, DUO.pads_per_bank);
    }
}
