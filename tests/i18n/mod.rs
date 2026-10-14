// Module: magic_hotspot.tests.i18n
// Purpose: Verify saved locale precedence and preserve opaque device names.
// Created: 2026-10-13
// Architecture: Pure locale inputs and an isolated settings directory.
use super::*;
#[test]
fn saved_choice_overrides_the_default_locale() {
    assert_eq!(detect(Some("el"), ["pt_BR.UTF-8".into()]), Lang::El);
    assert_eq!(detect(None, ["es_ES.UTF-8".into()]), Lang::Es);
    assert_eq!(detect(None, ["pt-BR".into()]), Lang::Pt);
    assert_eq!(detect(None, ["el_GR.UTF-8".into()]), Lang::El);
    assert_eq!(detect(None, ["C".into(), "fr_FR".into()]), Lang::En);
    assert_eq!(detect(Some("invalid"), ["el_GR".into()]), Lang::El);
    assert_eq!(detect(None, ["fr_FR".into(), "es_ES".into()]), Lang::Es);
    assert_eq!(Lang::from_code("english-noise"), None);
}
#[test]
fn choice_is_saved_and_loaded_without_touching_user_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("magic-hotspot/language");
    for language in Lang::ALL {
        save(&file, language).unwrap();
        let saved = std::fs::read_to_string(&file).unwrap();
        assert_eq!(detect(Some(&saved), ["C".into()]), language);
    }
}
#[test]
fn translations_keep_template_placeholders_and_device_names() {
    for row in TABLE {
        for lang in [Lang::Pt, Lang::Es, Lang::El] {
            assert!(!translate(lang, row[0]).is_empty());
            let placeholders = |text: &str| {
                text.split('{')
                    .skip(1)
                    .filter_map(|s| s.split_once('}').map(|(key, _)| key.to_string()))
                    .collect::<Vec<_>>()
            };
            assert_eq!(placeholders(row[0]), placeholders(translate(lang, row[0])));
        }
    }
    LANG.with(|lang| lang.set(Lang::El));
    assert_eq!(
        trf("{name} via Wi-Fi", &[("name", "Phone {mode}")]),
        "Phone {mode} μέσω Wi-Fi"
    );
    LANG.with(|lang| lang.set(Lang::En));
}
