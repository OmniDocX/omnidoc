use super::*;

#[test]
fn test_inches_to_emu() {
    let emu: Emu = Inches(1.0).into();
    assert_eq!(emu.0, 914400);
}

#[test]
fn test_cm_to_emu() {
    let emu: Emu = Cm(1.0).into();
    assert_eq!(emu.0, 360000);
}

#[test]
fn test_pt_to_emu() {
    let emu: Emu = Pt(1.0).into();
    assert_eq!(emu.0, 12700);
}

#[test]
fn test_mm_to_emu() {
    let emu: Emu = Mm(1.0).into();
    assert_eq!(emu.0, 36000);
}

#[test]
fn test_emu_to_inches() {
    let inches: Inches = Emu(914400).into();
    assert!((inches.0 - 1.0).abs() < 1e-10);
}

#[test]
fn test_emu_round_trip() {
    let original = Inches(2.5);
    let emu: Emu = original.into();
    let back: Inches = emu.into();
    assert!((back.0 - original.0).abs() < 1e-10);
}

#[test]
fn test_emu_arithmetic() {
    assert_eq!(Emu(100) + Emu(200), Emu(300));
    assert_eq!(Emu(300) - Emu(100), Emu(200));
}

#[test]
fn test_centipoints_to_emu() {
    let emu: Emu = Centipoints(1.0).into();
    assert_eq!(emu.0, 127);
    let emu: Emu = Centipoints(100.0).into();
    assert_eq!(emu.0, 12700);
}

#[test]
fn test_emu_to_centipoints() {
    let cp: Centipoints = Emu(127).into();
    assert!((cp.0 - 1.0).abs() < 1e-10);
    let cp: Centipoints = Emu(12700).into();
    assert!((cp.0 - 100.0).abs() < 1e-10);
}

#[test]
fn test_centipoints_round_trip() {
    let original = Centipoints(250.0);
    let emu: Emu = original.into();
    let back: Centipoints = emu.into();
    assert!((back.0 - original.0).abs() < 1e-10);
}

#[test]
fn test_twips_to_emu() {
    let emu: Emu = Twips(1.0).into();
    assert_eq!(emu.0, 635);
    let emu: Emu = Twips(20.0).into();
    assert_eq!(emu.0, 12700);
}

#[test]
fn test_emu_to_twips() {
    let tw: Twips = Emu(635).into();
    assert!((tw.0 - 1.0).abs() < 1e-10);
    let tw: Twips = Emu(12700).into();
    assert!((tw.0 - 20.0).abs() < 1e-10);
}

#[test]
fn test_twips_round_trip() {
    let original = Twips(50.0);
    let emu: Emu = original.into();
    let back: Twips = emu.into();
    assert!((back.0 - original.0).abs() < 1e-10);
}

#[test]
fn test_emu_to_centipoints_method() {
    assert!((Emu(12700).to_centipoints() - 100.0).abs() < 1e-10);
}

#[test]
fn test_emu_to_twips_method() {
    assert!((Emu(12700).to_twips() - 20.0).abs() < 1e-10);
}
