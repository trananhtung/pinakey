//! Các bài test cho engine biến đổi (transformation), chuyển nguyên vẹn từ các bộ test vector Telex/VNI/VIQR tham chiếu.
//! Chúng chạy thông qua API công khai của `pinakey-core` như một bên sử dụng bên ngoài.

use pinakey_core::{flag, mode, new_engine, parse_builtin_input_method, IEngine, PinaKeyEngine};

fn new_std_engine() -> PinaKeyEngine {
    let im = parse_builtin_input_method("Telex 2");
    new_engine(im, flag::STD_FLAGS)
}

const VIE: u32 = mode::VIETNAMESE;
const ENG: u32 = mode::ENGLISH;

#[test]
fn test_process_string() {
    let mut ng = new_std_engine();
    ng.process_string("aw", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ă");
    ng.reset();
    ng.process_string("uw", VIE);
    ng.process_string("o", VIE);
    ng.process_string("w", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ươ");
    ng.reset();
    ng.process_string("chuaarn", VIE);
    assert_eq!(ng.get_processed_string(VIE), "chuẩn");
    ng.reset();
    ng.process_string("giamaf", VIE);
    assert_eq!(ng.get_processed_string(VIE), "giầm");
}

#[test]
fn test_process_dd_string() {
    let mut ng = new_std_engine();
    ng.process_string("dd", VIE);
    assert!(ng.is_valid(false));
    ng.reset();
    ng.process_string("ddafi", VIE);
    assert_eq!(ng.get_processed_string(VIE), "đài");
}

#[test]
fn test_process_muoiwq_string() {
    let mut ng = new_std_engine();
    ng.process_string("Muoiwq", VIE);
    assert_eq!(ng.get_processed_string(ENG), "Muoiwq");
    ng.reset();
    ng.process_string("mootj", VIE);
    assert_eq!(ng.get_processed_string(VIE), "một");
}

#[test]
fn test_process_thuow_string() {
    let mut ng = new_std_engine();
    ng.process_string("Thuow", VIE);
    assert_eq!(ng.get_processed_string(VIE), "Thuơ");
    ng.remove_last_char(true);
    assert_eq!(ng.get_processed_string(VIE), "Thu");
}

#[test]
fn test_remove_last_char_engine() {
    let mut ng = new_std_engine();
    ng.remove_last_char(true);
    ng.process_string(" ", ENG);
    ng.remove_last_char(true);
    ng.process_string("loanj", VIE);
    assert_eq!(ng.get_processed_string(VIE), "loạn");
    ng.remove_last_char(true);
    assert_eq!(ng.get_processed_string(VIE), "lọa");
    ng.process_string(":", ENG);
    ng.remove_last_char(true);
    assert_eq!(ng.get_processed_string(VIE), "lọa");
}

#[test]
fn test_process_upper_string() {
    let mut ng = new_std_engine();
    ng.process_string("VIEETJ", VIE);
    assert_eq!(ng.get_processed_string(VIE), "VIỆT");
    ng.remove_last_char(false);
    assert_eq!(ng.get_processed_string(VIE), "VIỆ");
    ng.process_key('Q', VIE);
    assert_eq!(ng.get_processed_string(ENG), "VIEEJQ");
    ng.reset();
    ng.process_string("IB", ENG);
    assert_eq!(ng.get_processed_string(ENG), "IB");
}

#[test]
fn test_spelling_check() {
    let mut ng = new_std_engine();
    ng.process_string("noww", VIE);
    assert_eq!(ng.get_processed_string(ENG), "noww");
    assert_eq!(ng.get_processed_string(VIE), "now");
    ng.reset();
    ng.process_string("sawss", VIE);
    assert_eq!(ng.get_processed_string(ENG), "sawss");
    ng.reset();
    ng.process_string("sawss", VIE);
    assert_eq!(ng.get_processed_string(VIE), "săs");
}

#[test]
fn test_process_dd() {
    let mut ng = new_std_engine();
    ng.process_string("dd", VIE);
    assert!(ng.is_valid(false));
    assert_eq!(ng.get_processed_string(VIE), "đ");
    ng.reset();
    ng.process_string("SD", VIE);
    ng.process_string("D", VIE);
    assert_eq!(ng.get_processed_string(VIE), "SĐ");
}

#[test]
fn test_telex23() {
    let mut ng = new_std_engine();
    ng.process_string("t ]", ENG);
    ng.process_string("a", VIE);
    assert_eq!(ng.get_processed_string(VIE), "]a");
    ng.reset();
    ng.process_string("]]a", VIE);
    assert_eq!(ng.get_processed_string(VIE), "]a");
    let im = parse_builtin_input_method("Telex 2");
    let mut ng = new_engine(im, flag::STD_FLAGS);
    ng.process_string("[", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ơ");
    ng.reset();
    ng.process_string("{", VIE);
    assert_eq!(ng.get_processed_string(VIE), "Ơ");
}

#[test]
fn test_process_nguwowfi_string() {
    let mut ng = new_std_engine();
    ng.process_string("wowfi", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ười");
}

#[test]
fn test_remove_last_char() {
    let mut ng = new_std_engine();
    ng.process_string("hanhj", VIE);
    ng.remove_last_char(true);
    assert_eq!(ng.get_processed_string(VIE), "hạn");
}

#[test]
fn test_process_catr_string() {
    let mut ng = new_std_engine();
    ng.process_string("catr", VIE);
    assert_eq!(ng.get_processed_string(VIE), "catr");
}

#[test]
fn test_process_toowi_string() {
    let mut ng = new_std_engine();
    ng.process_string("toowi", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tơi");
}

#[test]
fn test_process_aloo_string() {
    let mut ng = new_std_engine();
    ng.process_string("aloo", VIE);
    assert_eq!(ng.get_processed_string(VIE), "alô");
}

#[test]
fn test_spelling_check_for_giw() {
    let mut ng = new_std_engine();
    ng.process_string("giw", VIE);
    assert!(ng.is_valid(false));
}

#[test]
fn test_double_brackets() {
    let mut ng = new_std_engine();
    ng.process_string("[[", VIE);
    assert_eq!(ng.get_processed_string(ENG), "[");
}

#[test]
fn test_double_brackets_o() {
    let mut ng = new_std_engine();
    ng.process_string("tooss", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tôs");
    ng.reset();
    ng.process_string("tosos", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tôs");
}

#[test]
fn test_double_w() {
    let mut ng = new_std_engine();
    ng.process_string("ww", VIE);
    assert_eq!(ng.get_processed_string(ENG), "w");
    assert_eq!(ng.get_processed_string(VIE), "w");
}

#[test]
fn test_double_w2() {
    let mut ng = new_std_engine();
    ng.process_string("wiw", VIE);
    assert_eq!(ng.get_processed_string(VIE), "uiw");
    assert_eq!(ng.get_processed_string(ENG), "wiw");
}

#[test]
fn test_process_duwoi() {
    let mut ng = new_std_engine();
    ng.process_string("duwoi", VIE);
    assert_eq!(ng.get_processed_string(VIE), "dươi");
}

#[test]
fn test_process_refresh() {
    let mut ng = new_std_engine();
    ng.process_string("reff", VIE);
    ng.process_string("resh", ENG);
    assert_eq!(ng.get_processed_string(ENG), "reffresh");
    assert_eq!(ng.get_processed_string(VIE), "refresh");
}

#[test]
fn test_process_refresh2() {
    let mut ng = new_std_engine();
    ng.process_string("reff", VIE);
    ng.remove_last_char(true);
    ng.process_key('f', VIE);
    assert_eq!(ng.get_processed_string(VIE), "rè");
}

#[test]
fn test_process_dd_seq() {
    let mut ng = new_std_engine();
    ng.process_string("oddp", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ođp");
}

#[test]
fn test_process_gisa() {
    let mut ng = new_std_engine();
    ng.process_string("gis", VIE);
    ng.process_string("a", VIE);
    assert_eq!(ng.get_processed_string(VIE), "giá");
}

#[test]
fn test_process_kimso() {
    let mut ng = new_std_engine();
    ng.process_string("kimso", VIE);
    assert_eq!(ng.get_processed_string(VIE), "kímo");
}

#[test]
fn test_process_to() {
    let mut ng = new_std_engine();
    ng.process_string("to", VIE);
    assert!(ng.is_valid(true));
}

#[test]
fn test_process_toorr() {
    let mut ng = new_std_engine();
    ng.process_string("toorr", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tôr");
}

#[test]
fn test_process_tnoss() {
    let mut ng = new_std_engine();
    ng.process_string("tnoss", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tnos");
}

#[test]
fn test_process_eenghf() {
    let im = parse_builtin_input_method("Telex 2");
    let mut ng = new_engine(im, flag::STD_FLAGS);
    ng.process_string("ddawks", VIE);
    assert_eq!(ng.get_processed_string(VIE), "đắk");
}

#[test]
fn test_process_hieeur() {
    let mut ng = new_std_engine();
    ng.process_string("tooi oo HIEEUR", VIE);
    assert_eq!(ng.get_processed_string(VIE), "HIỂU");
}

#[test]
fn test_process_nguoiw() {
    let mut ng = new_std_engine();
    ng.process_string("NGUOIW", VIE);
    assert_eq!(ng.get_processed_string(VIE), "NGƯƠI");
}

#[test]
fn test_process_t_os() {
    let mut ng = new_std_engine();
    ng.process_string("{s", VIE);
    assert_eq!(ng.get_processed_string(VIE), "Ớ");
}

#[test]
fn test_process_to5() {
    let im = parse_builtin_input_method("VNI");
    let mut ng = new_engine(im, flag::STD_FLAGS);
    ng.process_string("o55", VIE);
    assert_eq!(ng.get_processed_string(VIE), "o5");
}

#[test]
fn test_process_huoswc() {
    let mut ng = new_std_engine();
    ng.process_string("duwongwj", VIE);
    assert_eq!(ng.get_processed_string(VIE), "duongwj");
}

#[test]
fn test_process_choas() {
    let im = parse_builtin_input_method("Telex 2");
    let mut ng = new_engine(im, flag::STD_FLAGS & !flag::STD_TONE_STYLE);
    ng.process_string("choas", VIE);
    assert_eq!(ng.get_processed_string(VIE), "choá");
    ng.reset();
    ng.process_string("bieecs", VIE);
    assert_eq!(ng.get_processed_string(VIE), "biếc");
    ng.reset();
    ng.process_string("uese", VIE);
    assert_eq!(ng.get_processed_string(VIE), "uế");
}

#[test]
fn test_restore_last_word() {
    let mut ng = new_std_engine();
    ng.process_string("duwongj tooi", VIE);
    ng.restore_last_word(false);
    assert_eq!(ng.get_processed_string(VIE), "tooi");
}

#[test]
fn test_restore_last_word_tcvn() {
    let im = parse_builtin_input_method("Microsoft layout");
    let mut ng = new_engine(im, flag::STD_FLAGS);
    ng.process_string("112", VIE);
    assert_eq!(ng.get_processed_string(VIE), "1â");
    ng.restore_last_word(false);
    assert_eq!(ng.get_processed_string(ENG), "12");
    ng.reset();
    ng.process_string("d[]ng9 t4i", VIE);
    ng.restore_last_word(false);
    assert_eq!(ng.get_processed_string(VIE), "t4i");
}

#[test]
fn test_z_processing() {
    let mut ng = new_std_engine();
    ng.process_string("loz", VIE);
    assert_eq!(ng.get_processed_string(VIE), "loz");
    ng.reset();
    ng.process_string("losz", VIE);
    assert_eq!(ng.get_processed_string(VIE), "lo");
    assert_eq!(ng.get_processed_string(ENG), "losz");
}

#[test]
fn test_process_vn_word() {
    let mut ng = new_std_engine();
    ng.process_string("tôifs", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tối");
    assert_eq!(ng.get_processed_string(ENG), "tôifs");
    ng.reset();
    ng.process_string("tốif", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tồi");
    assert_eq!(ng.get_processed_string(ENG), "tốif");
    ng.reset();
    ng.process_string("tốiz", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tôi");
}

#[test]
fn test_double_typing() {
    let mut ng = new_std_engine();
    ng.process_string("linux", VIE);
    ng.process_string("x", VIE);
    assert_eq!(ng.get_processed_string(VIE), "linux");
    ng.reset();
    ng.process_string("buwo", VIE);
    ng.process_string("o", VIE);
    assert_eq!(ng.get_processed_string(VIE), "buô");
    ng.reset();
    ng.process_string("buowc", VIE);
    ng.process_string("o", VIE);
    assert_eq!(ng.get_processed_string(VIE), "buôc");
    ng.reset();
    ng.process_string("cuoiw", VIE);
    ng.process_string("o", VIE);
    assert_eq!(ng.get_processed_string(VIE), "cuôi");
    ng.reset();
    ng.process_string("ach", VIE);
    ng.process_string("a", VIE);
    assert_eq!(ng.get_processed_string(VIE), "acha");
    ng.reset();
    ng.process_string("nhuw", VIE);
    assert_eq!(ng.get_processed_string(VIE), "như");
    assert!(ng.is_valid(true));
    ng.reset();
    ng.process_string("thuw", VIE);
    assert!(ng.is_valid(true));
    ng.reset();
    ng.process_string("thow", VIE);
    assert!(ng.is_valid(true));
    ng.reset();
    ng.process_string("tooi", VIE);
    assert_eq!(ng.get_processed_string(VIE), "tôi");
    assert!(ng.is_valid(true));
    ng.reset();
    ng.process_string("arch", VIE);
    assert!(!ng.is_valid(false));
    ng.reset();
    ng.process_string("[[", VIE);
    ng.process_string("oo", VIE);
    assert_eq!(ng.get_processed_string(VIE), "[ô");
    ng.reset();
    ng.process_string("oo]", VIE);
    assert_eq!(ng.get_processed_string(VIE), "ôư");
    ng.reset();
    ng.process_string("chury", VIE);
    assert!(ng.is_valid(true));
    ng.reset();
    ng.process_string("turyn", VIE);
    ng.remove_last_char(true);
    ng.remove_last_char(true);
    assert_eq!(ng.get_processed_string(VIE), "tủ");
    ng.reset();
    ng.process_string("chuyển", VIE);
    ng.process_string("z", VIE);
    assert_eq!(ng.get_processed_string(VIE), "chuyên");
    ng.reset();
    ng.process_string("nhueej", VIE);
    assert_eq!(ng.get_processed_string(VIE), "nhuệ");
    ng.reset();
    ng.process_string("cuongw", VIE);
    assert_eq!(ng.get_processed_string(VIE), "cương");
    ng.reset();
    ng.process_string("quawcj", VIE);
    assert_eq!(ng.get_processed_string(VIE), "quặc");
    ng.reset();
    ng.process_string("quawcj", VIE);
    assert_eq!(ng.get_processed_string(VIE), "quặc");
    ng.reset();
    ng.process_string("tôi）t", ENG);
    assert_eq!(ng.get_processed_string(VIE), "t");
}
