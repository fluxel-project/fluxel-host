use super::*;

#[test]
fn logical_keys_are_normalized_and_named_keys_are_preserved() {
    assert_eq!(
        translate_key(&WinitKey::Character("A".into())),
        Key::Character('a')
    );
    assert_eq!(
        translate_key(&WinitKey::Named(NamedKey::ArrowLeft)),
        Key::ArrowLeft
    );
    assert_eq!(
        translate_key(&WinitKey::Named(NamedKey::Control)),
        Key::Control
    );
    assert_eq!(translate_key(&WinitKey::Character("ab".into())), Key::Other);
}
