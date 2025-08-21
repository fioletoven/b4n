use crossterm::event::KeyCode;
use std::{cell::RefCell, rc::Rc};

use crate::core::AppData;

use super::*;

#[test]
fn esc_reverts_value_test() {
    let data = Rc::new(RefCell::new(AppData::default()));
    let mut filter = Filter::new(data, None, 60);

    filter.show();
    filter.process_key(KeyCombination::from('t'));
    filter.process_key(KeyCombination::from('e'));
    filter.process_key(KeyCombination::from('s'));
    filter.process_key(KeyCombination::from('t'));

    assert_eq!("test", filter.value());

    filter.process_key(KeyCombination::from(KeyCode::Esc));

    assert_eq!("", filter.value());
}

#[test]
fn enter_stores_value_test() {
    let data = Rc::new(RefCell::new(AppData::default()));
    let mut filter = Filter::new(data, None, 60);

    filter.show();
    filter.process_key(KeyCombination::from('t'));
    filter.process_key(KeyCombination::from('e'));
    filter.process_key(KeyCombination::from('s'));
    filter.process_key(KeyCombination::from('t'));

    assert_eq!("test", filter.value());

    filter.process_key(KeyCombination::from(KeyCode::Enter));

    assert_eq!("test", filter.value());
}
