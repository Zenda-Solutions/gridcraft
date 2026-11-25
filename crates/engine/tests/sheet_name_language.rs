//! New sheets take the interface language's name («Hoja1» in Spanish). Its own test binary: the sheet
//! name is process-wide, and the other tests expect Excel's English default.

use gridcraft_engine::Session;

#[test]
fn sheets_take_the_interface_language_name() {
    // Zenda's E2E of Work 0.2.30: the sheet tab said «Sheet1» with Planilla in Spanish.
    let mut s = Session::new();
    s.new_workbook();
    assert_eq!(s.doc().unwrap().wb.sheets[0].name, "Sheet1");
    s.set_sheet_name("Hoja");
    let d = s.doc().unwrap();
    assert_eq!(d.wb.sheets[0].name, "Hoja1", "the untouched blank book is renamed");
    assert!(!d.is_dirty(), "and it is still unchanged, so closing it asks nothing");
    s.new_workbook();
    let d = s.doc().unwrap();
    assert_eq!(d.wb.sheets[0].name, "Hoja1");
    assert_eq!(d.wb.next_sheet_name(), "Hoja2");
}
