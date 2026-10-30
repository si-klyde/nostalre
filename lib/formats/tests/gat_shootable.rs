use nostalre_formats::gat::{GatCell, GatFile};

fn cell(raw_flags: u16) -> GatCell {
    GatCell {
        height_sw: 0.0,
        height_se: 0.0,
        height_nw: 0.0,
        height_ne: 0.0,
        cell_flags: raw_flags,
    }
}

#[test]
fn shootable_out_of_bounds() {
    let gat = GatFile {
        version: (1, 2),
        width: 1,
        height: 1,
        cells: vec![cell(u16::MAX)],
    };
    assert!(gat.is_shootable(0, 0));
    assert!(!gat.is_shootable(-1, 0));
    assert!(!gat.is_shootable(0, 1));
    assert!(!gat.is_shootable(1, 0));
}
